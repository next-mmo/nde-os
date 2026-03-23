use super::{ChannelMessage, ChannelResponse, ChannelType};
use serde::{Deserialize, Serialize};

/// Normalize a raw platform message into a `ChannelMessage`.
pub fn normalize_rest_message(
    message: &str,
    sender_id: &str,
    conversation_id: Option<&str>,
) -> ChannelMessage {
    ChannelMessage {
        id: uuid::Uuid::new_v4().to_string(),
        channel_type: ChannelType::Rest,
        channel_name: "nde-desktop".to_string(),
        sender_id: sender_id.to_string(),
        sender_name: None,
        content: message.to_string(),
        attachments: Vec::new(),
        metadata: serde_json::json!({}),
        timestamp: chrono::Utc::now(),
        conversation_ref: conversation_id.map(|s| s.to_string()),
    }
}

/// Format an agent response for a specific channel's conventions.
pub fn format_response(
    content: &str,
    channel_type: ChannelType,
) -> String {
    match channel_type {
        ChannelType::Telegram => {
            // Telegram uses HTML or Markdown formatting
            // Convert code blocks for Telegram markdown
            content
                .replace("```", "```")
                .to_string()
        }
        ChannelType::Discord => {
            // Discord supports standard Markdown
            content.to_string()
        }
        ChannelType::Slack => {
            // Slack uses mrkdwn (their own markdown variant)
            content
                .replace("**", "*")
                .replace("```", "```")
                .to_string()
        }
        _ => content.to_string(),
    }
}

/// Gateway routing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub channels: Vec<ChannelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub name: String,
    pub channel_type: ChannelType,
    pub enabled: bool,
    #[serde(default)]
    pub config: serde_json::Value,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            channels: vec![ChannelConfig {
                name: "nde-desktop".into(),
                channel_type: ChannelType::Rest,
                enabled: true,
                config: serde_json::json!({}),
            }],
        }
    }
}
