/// Per-gateway chat session management.
///
/// Maintains isolated conversation history per chat_id, reusable across
/// any gateway transport (Telegram, Discord, WhatsApp, etc.).
use std::collections::HashMap;

const MAX_HISTORY: usize = 20;

pub(crate) struct ChatMessage {
    pub role: &'static str,
    pub sender: String,
    pub text: String,
}

pub(crate) type ChatSessions = HashMap<i64, Vec<ChatMessage>>;

pub(crate) fn push_message(
    sessions: &mut ChatSessions,
    chat_id: i64,
    role: &'static str,
    sender: &str,
    text: &str,
) {
    let history = sessions.entry(chat_id).or_default();
    history.push(ChatMessage {
        role,
        sender: sender.to_string(),
        text: text.to_string(),
    });
    if history.len() > MAX_HISTORY {
        let drain_count = history.len() - MAX_HISTORY;
        history.drain(..drain_count);
    }
}

pub(crate) fn build_context(sessions: &ChatSessions, chat_id: i64, current_msg: &str) -> String {
    if let Some(history) = sessions.get(&chat_id) {
        if history.is_empty() {
            return current_msg.to_string();
        }
        let mut ctx = String::from("[Conversation context for this chat]\n");
        for msg in history.iter() {
            ctx.push_str(&format!("{} ({}): {}\n", msg.role, msg.sender, msg.text));
        }
        ctx.push_str(&format!("\n[Current message]\n{}", current_msg));
        ctx
    } else {
        current_msg.to_string()
    }
}
