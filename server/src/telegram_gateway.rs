/// Telegram Gateway — long-polling loop that processes incoming messages.
///
/// Kanban slash commands (/todo_list, /todo_add, /todo_done) are executed
/// directly via core::mcp::kanban without touching the LLM.
/// All other messages are routed through the AgentManager for LLM processing.

use ai_launcher_core::agent::manager::AgentManager;
use ai_launcher_core::agent::protocol::AgentEvent;
use serde::Deserialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

// ── Shared gateway state ─────────────────────────────────────────────────────

/// Shared state for the Telegram gateway — tracks running status and message counts.
/// Stored as server-wide shared state so `list_channels` and `configure_channel`
/// can read/write it.
pub struct GatewayState {
    pub running: AtomicBool,
    pub messages_received: AtomicU64,
    pub messages_sent: AtomicU64,
}

impl GatewayState {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            messages_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
        }
    }

    /// Signal the gateway to stop.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// Configuration for the Telegram gateway, read from channels.json.
#[derive(Debug, Clone)]
pub struct TelegramGatewayConfig {
    pub token: String,
    pub enabled: bool,
    /// Telegram user IDs allowed to interact with the bot.
    /// Empty = allow all (backward compat, but logs a warning).
    pub allowed_user_ids: Vec<i64>,
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

        // Decrypt the token (handles both encrypted and plaintext)
        let token = match ai_launcher_core::secrets::decrypt_token(&raw_token, data_dir) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(error = %e, "Failed to decrypt Telegram token");
                return None;
            }
        };

        // Parse allowed_users array from config
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

        Some(Self { token, enabled, allowed_user_ids })
    }
}

// ── Telegram API types ───────────────────────────────────────────────────────

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

// ── Per-chat session ─────────────────────────────────────────────────────────

/// Represents a single message in a chat session.
struct ChatMessage {
    role: &'static str, // "user" or "assistant"
    sender: String,
    text: String,
}

/// Per-chat conversation buffer. Keeps the last N messages as context.
const MAX_HISTORY: usize = 20;

type ChatSessions = std::collections::HashMap<i64, Vec<ChatMessage>>;

fn push_message(sessions: &mut ChatSessions, chat_id: i64, role: &'static str, sender: &str, text: &str) {
    let history = sessions.entry(chat_id).or_default();
    history.push(ChatMessage {
        role,
        sender: sender.to_string(),
        text: text.to_string(),
    });
    // Trim to max history
    if history.len() > MAX_HISTORY {
        let drain_count = history.len() - MAX_HISTORY;
        history.drain(..drain_count);
    }
}

fn build_context(sessions: &ChatSessions, chat_id: i64, current_msg: &str) -> String {
    if let Some(history) = sessions.get(&chat_id) {
        if history.is_empty() {
            return current_msg.to_string();
        }
        let mut ctx = String::from("[Conversation context for this Telegram chat]\n");
        for msg in history.iter() {
            ctx.push_str(&format!("{} ({}): {}\n", msg.role, msg.sender, msg.text));
        }
        ctx.push_str(&format!("\n[Current message]\n{}", current_msg));
        ctx
    } else {
        current_msg.to_string()
    }
}

// ── Gateway loop ─────────────────────────────────────────────────────────────

/// Start the Telegram gateway in a background tokio task.
/// The gateway checks `state.running` each poll cycle and exits when false.
/// Each chat_id maintains isolated conversation history.
pub fn start_telegram_gateway(
    config: TelegramGatewayConfig,
    agent_manager: Arc<tokio::sync::Mutex<AgentManager>>,
    handle: tokio::runtime::Handle,
    state: Arc<GatewayState>,
) {
    // If already running, don't start a second loop
    if state.running.load(Ordering::SeqCst) {
        println!("  Telegram:    gateway already running, skipping");
        return;
    }

    state.running.store(true, Ordering::SeqCst);
    let token = config.token.clone();
    let allowed_user_ids = config.allowed_user_ids.clone();
    println!("  Telegram:    gateway started (polling)");

    handle.spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client for Telegram");

        let mut offset: i64 = 0;
        let mut sessions: ChatSessions = std::collections::HashMap::new();

        loop {
            // Check shutdown flag
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
                                // Re-check shutdown between messages
                                if !state.running.load(Ordering::SeqCst) {
                                    break;
                                }

                                offset = update.update_id + 1;

                                if let Some(msg) = update.message {
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

                                    // ── Whitelist check ──
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

                                    // Try direct kanban execution first
                                    let response = match try_kanban_direct(&text) {
                                        Some(result) => result,
                                        None => {
                                            // Build context with chat history for isolation
                                            let prompt = build_context(&sessions, chat_id, &text);
                                            // Route through AgentManager
                                            process_with_agent(&prompt, &agent_manager).await
                                        }
                                    };

                                    // Record user message + bot response in session
                                    push_message(&mut sessions, chat_id, "user", &sender, &text);
                                    push_message(&mut sessions, chat_id, "assistant", "NDE-OS", &response);

                                    // Send response back to Telegram
                                    if let Err(e) =
                                        send_telegram_message(&client, &token, chat_id, &response)
                                            .await
                                    {
                                        tracing::error!(
                                            error = %e,
                                            chat_id = chat_id,
                                            "Failed to send Telegram reply"
                                        );
                                    } else {
                                        state.messages_sent.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Telegram poll error, retrying in 5s");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }

        state.running.store(false, Ordering::SeqCst);
        println!("  Telegram:    gateway stopped");
    });
}

/// Try to handle kanban slash commands directly without the LLM.
/// Returns Some(response_text) if handled, None otherwise.
fn try_kanban_direct(text: &str) -> Option<String> {
    let text_trimmed = text.trim();

    // /todo_list — list all tasks
    if text_trimmed == "/todo_list" || text_trimmed.starts_with("/todo_list ") {
        match ai_launcher_core::mcp::kanban::execute(
            "nde_kanban_get_tasks",
            &serde_json::json!({}),
        ) {
            Ok(result) => {
                if let Ok(tasks) = serde_json::from_str::<Vec<serde_json::Value>>(&result) {
                    if tasks.is_empty() {
                        return Some(
                            "📋 No tasks found.\nUse /todo_add <title> to create one.".into(),
                        );
                    }
                    let mut lines = vec![format!("📋 Kanban Board ({} tasks)\n", tasks.len())];
                    for task in &tasks {
                        let status = task
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Plan");
                        let title = task
                            .get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Untitled");
                        let filename = task
                            .get("filename")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let emoji = match status {
                            "Plan" => "🔴",
                            "YOLO mode" => "🟡",
                            "Done by AI" => "🟢",
                            "Verified" => "✅",
                            "Re-open" => "🔴",
                            "Waiting Approval" => "🟠",
                            _ => "⚪",
                        };
                        lines.push(format!("{} {} — {} ({})", emoji, title, status, filename));
                    }
                    return Some(lines.join("\n"));
                }
                Some(result)
            }
            Err(e) => Some(format!("❌ Failed to list tasks: {}", e)),
        }
    }
    // /todo_add <title> — create a task
    else if text_trimmed.starts_with("/todo_add ") {
        let title = text_trimmed["/todo_add ".len()..].trim();
        if title.is_empty() {
            return Some("❌ Usage: /todo_add <task title>".into());
        }
        let params = serde_json::json!({ "title": title });
        match ai_launcher_core::mcp::kanban::execute("nde_kanban_create_task", &params) {
            Ok(result) => {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&result) {
                    let fname = data
                        .get("filename")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    Some(format!("✅ Task created: {}\n📄 File: {}", title, fname))
                } else {
                    Some(format!("✅ Task created: {}", title))
                }
            }
            Err(e) => Some(format!("❌ Failed to create task: {}", e)),
        }
    }
    // /todo_done <filename> — mark as done
    else if text_trimmed.starts_with("/todo_done ") {
        let filename = text_trimmed["/todo_done ".len()..].trim();
        if filename.is_empty() {
            return Some("❌ Usage: /todo_done <filename.md>".into());
        }
        let fname = if filename.ends_with(".md") {
            filename.to_string()
        } else {
            format!("{}.md", filename)
        };
        let params = serde_json::json!({ "filename": fname, "status": "Done by AI" });
        match ai_launcher_core::mcp::kanban::execute("nde_kanban_update_task", &params) {
            Ok(_) => Some(format!("✔️ Task marked as done: {}", fname)),
            Err(e) => Some(format!("❌ Failed to update task: {}", e)),
        }
    }
    // /start — Telegram bot welcome
    else if text_trimmed == "/start" {
        Some(
            "👋 Welcome to NDE-OS Agent!\n\n\
            Available commands:\n\
            /todo_list — List all kanban tasks\n\
            /todo_add <title> — Create a new task\n\
            /todo_done <filename> — Mark task as done\n\
            /help — Show all commands\n\n\
            Or just type any message and I'll use the AI agent to help you."
                .into(),
        )
    }
    // /help
    else if text_trimmed == "/help" {
        Some(
            "🤖 NDE-OS Agent Commands:\n\n\
            📋 Kanban:\n\
            /todo_list — List all tasks\n\
            /todo_add <title> — Create task\n\
            /todo_done <file> — Mark done\n\n\
            💬 Any other message will be processed by the AI agent with 30+ tools \
            (file I/O, shell, web search, git, and more)."
                .into(),
        )
    } else {
        None
    }
}

/// Process a message through the AgentManager (LLM-based).
async fn process_with_agent(
    message: &str,
    agent_manager: &Arc<tokio::sync::Mutex<AgentManager>>,
) -> String {
    let mgr = agent_manager.lock().await;
    let mut rx = mgr.subscribe();

    let task_id = match mgr.spawn(message).await {
        Ok(id) => id,
        Err(e) => return format!("❌ Agent error: {}", e),
    };
    drop(mgr); // Release lock so events can flow

    let timeout = tokio::time::Duration::from_secs(120);
    let mut final_output = String::new();

    loop {
        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Ok(event)) => {
                if event.task_id() != task_id {
                    continue;
                }

                if let AgentEvent::TaskCompleted { ref output, .. } = event {
                    final_output = output.clone();
                }

                if event.is_terminal() {
                    break;
                }
            }
            Ok(Err(_)) => break,
            Err(_) => {
                return "⏰ Agent timed out (2 minutes). Try a simpler request.".into();
            }
        }
    }

    if final_output.is_empty() {
        "🤖 Agent completed but produced no output.".into()
    } else {
        // Telegram has a 4096 char limit per message
        if final_output.len() > 4000 {
            format!("{}…\n\n(truncated)", &final_output[..4000])
        } else {
            final_output
        }
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
