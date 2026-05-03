/// Per-gateway chat session management.
///
/// Formats cross-channel memory for Telegram.

pub(crate) fn build_canonical_context(
    chat_id: i64,
    current_msg: &str,
    summary: Option<String>,
    recent_messages: &[ai_launcher_core::memory::types::Message],
) -> String {
    let mut ctx = String::from("[Cross-Channel Memory Context]\n");
    if let Some(s) = summary {
        ctx.push_str(&format!("[Previous Summary]\n{}\n\n", s));
    }
    
    if !recent_messages.is_empty() {
        ctx.push_str("[Recent History]\n");
        for msg in recent_messages {
            let role_str = match msg.role {
                ai_launcher_core::memory::types::Role::User => "User",
                ai_launcher_core::memory::types::Role::Assistant => "Assistant",
                ai_launcher_core::memory::types::Role::System => "System",
            };
            ctx.push_str(&format!("{}: {}\n", role_str, msg.content));
        }
        ctx.push_str("\n");
    }
    ctx.push_str(&format!("[Current Message (from Telegram chat_id: {})]\n{}", chat_id, current_msg));
    ctx
}
