/// Telegram gateway transport.
///
/// This module owns Telegram-specific concerns only: config loading, Bot API
/// DTOs, polling, authorization, and Telegram message/photo delivery.
use crate::gateway::commands::{self, EmulatorAction};
use crate::gateway::session::{self, ChatSessions};
use crate::gateway::GatewayState;
use ai_launcher_core::agent::manager::AgentManager;
use ai_launcher_core::llm::manager::LlmManager;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

/// Configuration for the Telegram gateway, read from channels.json.
#[derive(Debug, Clone)]
pub struct TelegramGatewayConfig {
    pub token: String,
    pub allowed_user_ids: Vec<i64>,
    pub data_dir: PathBuf,
}

impl TelegramGatewayConfig {
    /// Load from channels.json in the data directory.
    /// Decrypts the token if it was stored encrypted.
    pub fn load(data_dir: &Path) -> Option<Self> {
        let config_path = data_dir.join("channels.json");
        let content = std::fs::read_to_string(&config_path).ok()?;
        let config: serde_json::Value = serde_json::from_str(&content).ok()?;

        let tg = config.get("telegram")?;
        let enabled = tg.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let raw_token = tg
            .get("token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !enabled || raw_token.is_empty() {
            return None;
        }

        let token = match ai_launcher_core::secrets::decrypt_token(&raw_token, data_dir) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(error = %e, "Failed to decrypt Telegram token");
                return None;
            }
        };

        let allowed_user_ids: Vec<i64> = tg
            .get("allowed_users")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();

        if allowed_user_ids.is_empty() {
            tracing::warn!("Telegram whitelist is empty — all users can interact with the bot");
        } else {
            tracing::info!(count = allowed_user_ids.len(), "Telegram whitelist loaded");
        }

        Some(Self {
            token,
            allowed_user_ids,
            data_dir: data_dir.to_path_buf(),
        })
    }
}

#[derive(Deserialize)]
struct TgResponse<T> {
    #[allow(dead_code)]
    ok: bool,
    result: Option<T>,
    #[allow(dead_code)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct TgUpdate {
    update_id: i64,
    message: Option<TgMessage>,
}

#[derive(Deserialize)]
struct TgMessage {
    #[allow(dead_code)]
    message_id: i64,
    from: Option<TgUser>,
    chat: TgChat,
    text: Option<String>,
}

#[derive(Deserialize)]
struct TgUser {
    #[allow(dead_code)]
    id: i64,
    first_name: String,
    #[allow(dead_code)]
    last_name: Option<String>,
}

#[derive(Deserialize)]
struct TgChat {
    id: i64,
}

/// Start the Telegram gateway in a background tokio task.
pub fn start_telegram_gateway(
    config: TelegramGatewayConfig,
    agent_manager: Arc<tokio::sync::Mutex<AgentManager>>,
    llm_manager: Arc<Mutex<LlmManager>>,
    handle: tokio::runtime::Handle,
    state: Arc<GatewayState>,
    log_buffer: super::log::SharedLogBuffer,
) {
    if state.running.load(Ordering::SeqCst) {
        println!("  Telegram:    gateway already running, skipping");
        return;
    }

    state.running.store(true, Ordering::SeqCst);
    let token = config.token.clone();
    let allowed_user_ids = config.allowed_user_ids.clone();
    let data_dir = config.data_dir.clone();

    super::log::log_success(
        &log_buffer,
        "telegram",
        "Telegram gateway started (polling)",
    );
    println!("  Telegram:    gateway started (polling)");

    handle.spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client for Telegram");

        let mut offset = 0_i64;
        let mut sessions: ChatSessions = std::collections::HashMap::new();

        loop {
            if !state.running.load(Ordering::SeqCst) {
                tracing::info!("Telegram gateway shutting down");
                break;
            }

            let url = format!(
                "https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=30",
                token, offset
            );

            match client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<TgResponse<Vec<TgUpdate>>>().await {
                        if let Some(updates) = data.result {
                            for update in updates {
                                if !state.running.load(Ordering::SeqCst) {
                                    break;
                                }

                                offset = update.update_id + 1;

                                let Some(msg) = update.message else {
                                    continue;
                                };

                                let text = msg.text.unwrap_or_default();
                                if text.is_empty() {
                                    continue;
                                }

                                let chat_id = msg.chat.id;
                                let user_id = msg.from.as_ref().map(|u| u.id);
                                let sender = msg
                                    .from
                                    .as_ref()
                                    .map(|u| u.first_name.clone())
                                    .unwrap_or_else(|| "User".to_string());

                                if !allowed_user_ids.is_empty() {
                                    let authorized = user_id
                                        .map(|id| allowed_user_ids.contains(&id))
                                        .unwrap_or(false);

                                    if !authorized {
                                        tracing::warn!(
                                            user_id = ?user_id,
                                            chat_id = chat_id,
                                            sender = %sender,
                                            "Unauthorized Telegram user rejected"
                                        );
                                        super::log::log_warn(
                                            &log_buffer,
                                            "telegram",
                                            format!("Rejected unauthorized user {} (id: {:?})", sender, user_id),
                                        );
                                        let _ = send_telegram_message(
                                            &client,
                                            &token,
                                            chat_id,
                                            "⛔ Unauthorized. Your user ID is not in the allowed list.",
                                        )
                                        .await;
                                        continue;
                                    }
                                }

                                state.messages_received.fetch_add(1, Ordering::Relaxed);

                                tracing::info!(
                                    chat_id = chat_id,
                                    sender = %sender,
                                    text = %text,
                                    "Telegram message received"
                                );
                                super::log::log_info(
                                    &log_buffer,
                                    "telegram",
                                    format!(
                                        "{}: {}",
                                        sender,
                                        if text.len() > 80 {
                                            format!("{}...", &text[..80])
                                        } else {
                                            text.clone()
                                        }
                                    ),
                                );

                                let response = if let Some(result) = commands::try_kanban(&text) {
                                    result
                                } else if let Some(result) = commands::try_shield(&text, &data_dir) {
                                    result
                                } else if let Some(action) = commands::try_emulator(&text, &data_dir) {
                                    handle_emulator_action(&client, &token, chat_id, action).await
                                } else if let Some((result, model_changed)) =
                                    commands::try_llm(&text, &llm_manager)
                                {
                                    if model_changed {
                                        commands::sync_agent_provider_from_llm(
                                            &agent_manager,
                                            &llm_manager,
                                        )
                                        .await;
                                    }
                                    result
                                } else if let Some(topic) =
                                    text.strip_prefix("/research ").map(str::trim)
                                {
                                    if topic.is_empty() {
                                        "❌ Usage: /research <topic>\nExample: /research AI news 2026".to_string()
                                    } else {
                                        let _ = send_telegram_message(
                                            &client,
                                            &token,
                                            chat_id,
                                            &format!("⏳ Researching \"{}\"… please wait.", topic),
                                        )
                                        .await;
                                        super::log::log_info(
                                            &log_buffer,
                                            "telegram",
                                            format!("Research request: {}", topic),
                                        );
                                        let prompt = commands::build_research_prompt(topic);
                                        let raw = commands::process_with_agent(&prompt, &agent_manager).await;
                                        commands::format_research_response(topic, &raw)
                                    }
                                } else {
                                    let prompt = session::build_context(&sessions, chat_id, &text);
                                    commands::process_with_agent(&prompt, &agent_manager).await
                                };

                                session::push_message(&mut sessions, chat_id, "user", &sender, &text);
                                session::push_message(&mut sessions, chat_id, "assistant", "NDE-OS", &response);

                                if let Err(e) = send_telegram_message(&client, &token, chat_id, &response).await {
                                    tracing::error!(
                                        error = %e,
                                        chat_id = chat_id,
                                        "Failed to send Telegram reply"
                                    );
                                    super::log::log_error(
                                        &log_buffer,
                                        "telegram",
                                        format!("Failed to send reply: {}", e),
                                    );
                                } else {
                                    state.messages_sent.fetch_add(1, Ordering::Relaxed);
                                    super::log::log_success(
                                        &log_buffer,
                                        "telegram",
                                        format!("Reply sent to {}", sender),
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Telegram poll error, retrying in 5s");
                    super::log::log_warn(&log_buffer, "telegram", format!("Poll error: {}", e));
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }

        state.running.store(false, Ordering::SeqCst);
        super::log::log_info(&log_buffer, "telegram", "Telegram gateway stopped");
        println!("  Telegram:    gateway stopped");
    });
}

async fn handle_emulator_action(
    client: &reqwest::Client,
    token: &str,
    chat_id: i64,
    action: EmulatorAction,
) -> String {
    match action {
        EmulatorAction::Reply(message) => message,
        EmulatorAction::SendScreenshot { path, caption } => {
            let _ = send_telegram_message(client, token, chat_id, "📸 Sending screenshot...").await;
            let send_result = send_telegram_photo(client, token, chat_id, &path, &caption).await;
            let _ = std::fs::remove_file(&path);
            match send_result {
                Ok(()) => "✅ Screenshot sent.".into(),
                Err(e) => format!("❌ Failed to send photo to Telegram: {}", e),
            }
        }
    }
}

async fn send_telegram_message(
    client: &reqwest::Client,
    token: &str,
    chat_id: i64,
    text: &str,
) -> Result<(), String> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
    });

    client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send Telegram message: {}", e))?;

    Ok(())
}

async fn send_telegram_photo(
    client: &reqwest::Client,
    token: &str,
    chat_id: i64,
    photo_path: &Path,
    caption: &str,
) -> Result<(), String> {
    let url = format!("https://api.telegram.org/bot{}/sendPhoto", token);

    let photo_bytes =
        std::fs::read(photo_path).map_err(|e| format!("Failed to read photo off disk: {}", e))?;

    let filename = photo_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let part = reqwest::multipart::Part::bytes(photo_bytes)
        .file_name(filename)
        .mime_str("image/png")
        .unwrap_or_else(|_| reqwest::multipart::Part::bytes(vec![]));

    let form = reqwest::multipart::Form::new()
        .text("chat_id", chat_id.to_string())
        .text("caption", caption.to_string())
        .part("photo", part);

    client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to send photo: {}", e))?;

    Ok(())
}
