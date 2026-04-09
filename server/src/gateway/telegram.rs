/// Telegram gateway transport.
///
/// This module owns Telegram-specific concerns only: config loading, Bot API
/// DTOs, polling, authorization, and Telegram message/photo delivery.
use crate::gateway::commands::{self, EmulatorAction};
use crate::gateway::session::{self, ChatSessions};
use crate::gateway::GatewayState;
use crate::router::DesktopActionQueue;
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
    callback_query: Option<TgCallbackQuery>,
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
struct TgCallbackQuery {
    id: String,
    from: TgUser,
    message: Option<TgCallbackMessage>,
    data: Option<String>,
}

#[derive(Deserialize)]
struct TgCallbackMessage {
    #[allow(dead_code)]
    message_id: i64,
    chat: TgChat,
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
    desktop_actions: DesktopActionQueue,
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

                                // ── Handle callback queries (inline button presses) ──
                                if let Some(cb) = update.callback_query {
                                    let cb_chat_id = cb
                                        .message
                                        .as_ref()
                                        .map(|m| m.chat.id)
                                        .unwrap_or(0);
                                    let cb_user_id = cb.from.id;

                                    // Auth check
                                    if !allowed_user_ids.is_empty()
                                        && !allowed_user_ids.contains(&cb_user_id)
                                    {
                                        let _ = answer_callback_query(
                                            &client,
                                            &token,
                                            &cb.id,
                                            Some("⛔ Unauthorized"),
                                        )
                                        .await;
                                        continue;
                                    }

                                    if let Some(data) = &cb.data {
                                        let response =
                                            handle_callback_query(data, &llm_manager, &agent_manager, &desktop_actions)
                                                .await;
                                        let _ = answer_callback_query(
                                            &client, &token, &cb.id, None,
                                        )
                                        .await;
                                        if cb_chat_id != 0 {
                                            let _ = send_telegram_message(
                                                &client, &token, cb_chat_id, &response,
                                            )
                                            .await;
                                        }
                                    }
                                    continue;
                                }

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
                                } else if text.trim() == "/apps" {
                                    // Interactive app list with inline keyboard
                                    let keyboard = build_apps_keyboard();
                                    let _ = send_telegram_message_with_keyboard(
                                        &client, &token, chat_id, &keyboard.0, &keyboard.1,
                                    )
                                    .await;
                                    continue;
                                } else if let Some(result) = commands::try_desktop_commands(&text, &desktop_actions) {
                                    result
                                } else if let Some(result) = commands::try_shield(&text, &data_dir) {
                                    result
                                } else if let Some(action) = commands::try_emulator(&text, &data_dir) {
                                    handle_emulator_action(&client, &token, chat_id, action).await
                                } else if text.trim() == "/models" {
                                    // Interactive model list with inline keyboard
                                    let keyboard = build_models_keyboard(&llm_manager);
                                    let _ = send_telegram_message_with_keyboard(
                                        &client, &token, chat_id, &keyboard.0, &keyboard.1,
                                    )
                                    .await;
                                    continue;
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

/// Build an interactive inline keyboard listing all providers & models.
fn build_models_keyboard(
    llm_manager: &Arc<Mutex<LlmManager>>,
) -> (String, serde_json::Value) {
    let mgr = match llm_manager.lock() {
        Ok(m) => m,
        Err(_) => {
            return (
                "❌ LLM manager lock failed".into(),
                serde_json::json!({ "inline_keyboard": [] }),
            )
        }
    };

    let configs = mgr.configs();
    let active_name = mgr.active_name().to_string();

    if configs.is_empty() {
        return (
            "🧠 No LLM providers configured.\nAdd one from Settings → Models.".into(),
            serde_json::json!({ "inline_keyboard": [] }),
        );
    }

    let mut text = format!("🧠 LLM Providers ({})\n", configs.len());
    text.push_str("Tap a provider to switch:\n\n");

    let mut rows: Vec<serde_json::Value> = Vec::new();

    for cfg in configs {
        let is_active = cfg.name == active_name;
        let marker = if is_active { " ✅" } else { "" };
        text.push_str(&format!(
            "  {} {} — {} ({}){marker}\n",
            if is_active { "▸" } else { "•" },
            cfg.name,
            cfg.model,
            cfg.provider_type,
        ));

        let label = if is_active {
            format!("✅ {} — {}", cfg.name, cfg.model)
        } else {
            format!("⬜ {} — {}", cfg.name, cfg.model)
        };

        rows.push(serde_json::json!([
            { "text": label, "callback_data": format!("model_switch:{}", cfg.name) }
        ]));
    }

    let keyboard = serde_json::json!({ "inline_keyboard": rows });
    (text, keyboard)
}

/// Build an interactive inline keyboard listing all desktop apps.
fn build_apps_keyboard() -> (String, serde_json::Value) {
    use crate::gateway::commands::STATIC_APP_IDS;

    let mut text = format!("🖥️ Desktop Apps ({})\n", STATIC_APP_IDS.len());
    text.push_str("Tap to open on desktop:\n");

    let mut rows: Vec<serde_json::Value> = Vec::new();

    // Group apps into rows of 2 buttons each for compact layout
    let mut row_pair: Vec<serde_json::Value> = Vec::new();
    for (id, title) in STATIC_APP_IDS {
        row_pair.push(serde_json::json!({
            "text": title, "callback_data": format!("app_open:{}", id)
        }));
        if row_pair.len() == 2 {
            rows.push(serde_json::Value::Array(row_pair.clone()));
            row_pair.clear();
        }
    }
    if !row_pair.is_empty() {
        rows.push(serde_json::Value::Array(row_pair));
    }

    let keyboard = serde_json::json!({ "inline_keyboard": rows });
    (text, keyboard)
}

/// Handle a callback query from an inline keyboard press.
async fn handle_callback_query(
    data: &str,
    llm_manager: &Arc<Mutex<LlmManager>>,
    agent_manager: &Arc<tokio::sync::Mutex<AgentManager>>,
    desktop_actions: &DesktopActionQueue,
) -> String {
    if let Some(provider_name) = data.strip_prefix("model_switch:") {
        let result = {
            let mut mgr = match llm_manager.lock() {
                Ok(m) => m,
                Err(_) => return "❌ LLM manager lock failed".into(),
            };

            // Fuzzy match provider name
            let target = {
                let names = mgr.provider_names();
                if names.iter().any(|n| n == provider_name) {
                    Some(provider_name.to_string())
                } else {
                    let lower = provider_name.to_lowercase();
                    names.into_iter().find(|n| n.to_lowercase() == lower)
                }
            };

            match target {
                Some(name) => match mgr.switch(&name) {
                    Ok(()) => Ok(name),
                    Err(e) => Err(format!("❌ Failed to switch: {}", e)),
                },
                None => Err(format!("❌ Provider '{}' not found.", provider_name)),
            }
        };

        match result {
            Ok(name) => {
                commands::sync_agent_provider_from_llm(agent_manager, llm_manager).await;
                // Build updated status
                let model = {
                    let mgr = llm_manager.lock().ok();
                    mgr.and_then(|m| {
                        m.configs()
                            .iter()
                            .find(|c| c.name == name)
                            .map(|c| c.model.clone())
                    })
                    .unwrap_or_default()
                };
                format!("✅ Switched to: {} ({})", name, model)
            }
            Err(msg) => msg,
        }
    } else if let Some(app_id) = data.strip_prefix("app_open:") {
        // Handle desktop app open from inline keyboard
        use crate::router::DesktopAction;
        use crate::gateway::commands::STATIC_APP_IDS;

        let lower = app_id.to_lowercase();
        let matched = STATIC_APP_IDS
            .iter()
            .find(|(id, _)| id.to_lowercase() == lower);

        match matched {
            Some((canonical_id, title)) => {
                if let Ok(mut q) = desktop_actions.lock() {
                    q.push(DesktopAction {
                        kind: "open_app".to_string(),
                        app_id: canonical_id.to_string(),
                    });
                }
                format!("✅ Opening {} on desktop…", title)
            }
            None => format!("❌ Unknown app '{}'", app_id),
        }
    } else if let Some(model_name) = data.strip_prefix("model_set:") {
        let result = {
            let mut mgr = match llm_manager.lock() {
                Ok(m) => m,
                Err(_) => return "❌ LLM manager lock failed".into(),
            };
            mgr.update_active_model(model_name)
        };

        match result {
            Ok(()) => {
                commands::sync_agent_provider_from_llm(agent_manager, llm_manager).await;
                format!("✅ Model changed to: {}", model_name)
            }
            Err(e) => format!("❌ Failed to change model: {}", e),
        }
    } else {
        format!("❓ Unknown action: {}", data)
    }
}

/// Send a text message via Telegram Bot API.
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

/// Send a message with an inline keyboard via Telegram Bot API.
async fn send_telegram_message_with_keyboard(
    client: &reqwest::Client,
    token: &str,
    chat_id: i64,
    text: &str,
    reply_markup: &serde_json::Value,
) -> Result<(), String> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "reply_markup": reply_markup,
    });

    client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send Telegram message: {}", e))?;

    Ok(())
}

/// Acknowledge a callback query to dismiss the loading spinner.
async fn answer_callback_query(
    client: &reqwest::Client,
    token: &str,
    callback_query_id: &str,
    text: Option<&str>,
) -> Result<(), String> {
    let url = format!("https://api.telegram.org/bot{}/answerCallbackQuery", token);
    let mut body = serde_json::json!({
        "callback_query_id": callback_query_id,
    });
    if let Some(t) = text {
        body["text"] = serde_json::Value::String(t.to_string());
        body["show_alert"] = serde_json::Value::Bool(true);
    }

    client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to answer callback query: {}", e))?;

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
