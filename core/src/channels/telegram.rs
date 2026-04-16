use super::{Channel, ChannelMessage, ChannelResponse, ChannelType};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Telegram Bot channel using long-polling via the Bot API.
pub struct TelegramChannel {
    name: String,
    token: String,
    client: reqwest::Client,
    running: Arc<AtomicBool>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl TelegramChannel {
    /// Create from bot token. Token can be passed directly or read from env.
    pub fn new(name: &str, token: &str) -> Self {
        Self {
            name: name.to_string(),
            token: token.to_string(),
            client: reqwest::Client::new(),
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: None,
        }
    }

    pub fn from_env(name: &str) -> Result<Self> {
        let token = std::env::var("TELEGRAM_BOT_TOKEN")
            .context("TELEGRAM_BOT_TOKEN environment variable not set")?;
        Ok(Self::new(name, &token))
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, method)
    }
}

// ── Telegram API types ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
}

#[derive(Deserialize)]
struct TelegramMessage {
    message_id: i64,
    from: Option<TelegramUser>,
    chat: TelegramChat,
    text: Option<String>,
    #[serde(default)]
    document: Option<TelegramDocument>,
}

#[derive(Deserialize)]
struct TelegramUser {
    id: i64,
    first_name: String,
    #[serde(default)]
    last_name: Option<String>,
    #[serde(default)]
    username: Option<String>,
}

#[derive(Deserialize)]
struct TelegramChat {
    id: i64,
    #[serde(rename = "type")]
    chat_type: String,
}

#[derive(Deserialize)]
struct TelegramDocument {
    file_name: Option<String>,
    file_size: Option<u64>,
    mime_type: Option<String>,
}

// ── Channel implementation ───────────────────────────────────────────────────

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        &self.name
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Telegram
    }

    async fn start(&mut self, tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let client = self.client.clone();
        let token = self.token.clone();
        let name = self.name.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut offset: i64 = 0;

            loop {
                // Check for shutdown
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                // Long-poll for updates
                let url = format!(
                    "https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=30",
                    token, offset
                );

                match client.get(&url).send().await {
                    Ok(resp) => {
                        if let Ok(data) = resp.json::<TelegramResponse<Vec<TelegramUpdate>>>().await
                        {
                            if let Some(updates) = data.result {
                                for update in updates {
                                    offset = update.update_id + 1;

                                    if let Some(msg) = update.message {
                                        let text = msg.text.unwrap_or_default();
                                        if text.is_empty() {
                                            continue;
                                        }

                                        let sender_id = msg
                                            .from
                                            .as_ref()
                                            .map(|u| u.id.to_string())
                                            .unwrap_or_default();
                                        let sender_name = msg.from.as_ref().map(|u| {
                                            let mut n = u.first_name.clone();
                                            if let Some(last) = &u.last_name {
                                                n.push(' ');
                                                n.push_str(last);
                                            }
                                            n
                                        });

                                        let channel_msg = ChannelMessage {
                                            id: msg.message_id.to_string(),
                                            channel_type: ChannelType::Telegram,
                                            channel_name: name.clone(),
                                            sender_id,
                                            sender_name,
                                            content: text,
                                            attachments: Vec::new(),
                                            metadata: serde_json::json!({
                                                "chat_id": msg.chat.id,
                                                "chat_type": msg.chat.chat_type,
                                            }),
                                            timestamp: chrono::Utc::now(),
                                            conversation_ref: Some(msg.chat.id.to_string()),
                                        };

                                        if tx.send(channel_msg).await.is_err() {
                                            tracing::error!("Channel message receiver dropped");
                                            running.store(false, Ordering::SeqCst);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "Telegram long-poll error, retrying in 5s"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }

            tracing::info!("Telegram polling loop exited");
        });

        Ok(())
    }

    async fn send(&self, response: &ChannelResponse) -> Result<()> {
        let chat_id = response.conversation_ref.as_ref().ok_or_else(|| {
            anyhow::anyhow!("No chat_id in conversation_ref for Telegram response")
        })?;

        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": response.content,
            "parse_mode": "Markdown",
        });

        let resp = self
            .client
            .post(&self.api_url("sendMessage"))
            .json(&body)
            .send()
            .await
            .context("Failed to send Telegram message")?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            tracing::error!(error = %text, "Telegram sendMessage failed");
        }

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}
