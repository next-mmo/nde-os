/// Channel system — pluggable messaging gateways for the agent.
/// Channels normalize messages from different platforms (REST, Telegram, Discord, etc.)
/// into a unified format for the agent to process.
pub mod gateway;
pub mod manager;
pub mod telegram;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── Channel types ────────────────────────────────────────────────────────────

/// Type of messaging channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// NDE-OS desktop chat (REST API)
    Rest,
    /// Telegram bot (long-polling)
    Telegram,
    /// Discord bot (Gateway WebSocket)
    Discord,
    /// Slack bot (Events API)
    Slack,
    /// Embeddable web chat widget
    WebChat,
    /// CLI stdin/stdout
    Cli,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rest => write!(f, "rest"),
            Self::Telegram => write!(f, "telegram"),
            Self::Discord => write!(f, "discord"),
            Self::Slack => write!(f, "slack"),
            Self::WebChat => write!(f, "web_chat"),
            Self::Cli => write!(f, "cli"),
        }
    }
}

// ── Normalized message ───────────────────────────────────────────────────────

/// A message normalized from any channel into a common format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// Unique message ID (from the source platform)
    pub id: String,
    /// Which channel this came from
    pub channel_type: ChannelType,
    /// Channel instance name (e.g., "telegram-bot-1")
    pub channel_name: String,
    /// Sender identifier (user ID on the platform)
    pub sender_id: String,
    /// Display name of the sender
    pub sender_name: Option<String>,
    /// Text content of the message
    pub content: String,
    /// File/image attachments
    pub attachments: Vec<Attachment>,
    /// Platform-specific metadata
    pub metadata: serde_json::Value,
    /// When the message was sent
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Conversation/thread/chat ID on the platform
    pub conversation_ref: Option<String>,
}

/// A response to send back through the channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelResponse {
    /// ID of the original message being replied to
    pub reply_to: String,
    /// Channel to reply through
    pub channel_type: ChannelType,
    /// Channel instance name
    pub channel_name: String,
    /// Response text content
    pub content: String,
    /// Response attachments
    pub attachments: Vec<Attachment>,
    /// Platform-specific reply metadata (e.g., thread_ts for Slack)
    pub metadata: serde_json::Value,
    /// Conversation/thread reference to reply in
    pub conversation_ref: Option<String>,
}

/// File or media attachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    /// URL or base64 data
    pub data: String,
    pub size_bytes: Option<u64>,
}

// ── Channel trait ────────────────────────────────────────────────────────────

/// Trait for a messaging channel gateway.
/// Implementations handle platform-specific protocols and normalize messages.
#[async_trait]
pub trait Channel: Send + Sync {
    /// Human-readable name of this channel instance.
    fn name(&self) -> &str;

    /// What type of channel this is.
    fn channel_type(&self) -> ChannelType;

    /// Start receiving messages. The channel should send normalized messages
    /// through the provided sender.
    async fn start(&mut self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> Result<()>;

    /// Send a response back through this channel.
    async fn send(&self, response: &ChannelResponse) -> Result<()>;

    /// Stop the channel gracefully.
    async fn stop(&mut self) -> Result<()>;

    /// Check if the channel is currently running.
    fn is_running(&self) -> bool;
}

/// Status of a registered channel.
#[derive(Debug, Clone, Serialize)]
pub struct ChannelStatus {
    pub name: String,
    pub channel_type: ChannelType,
    pub is_running: bool,
    pub messages_received: u64,
    pub messages_sent: u64,
}
