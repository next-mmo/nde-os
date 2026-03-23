use super::{Channel, ChannelMessage, ChannelResponse, ChannelStatus, ChannelType};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Manages all registered channels and routes messages to/from the agent.
pub struct ChannelManager {
    channels: HashMap<String, ChannelEntry>,
    message_tx: mpsc::Sender<ChannelMessage>,
    message_rx: Option<mpsc::Receiver<ChannelMessage>>,
}

struct ChannelEntry {
    channel: Box<dyn Channel>,
    stats: Arc<ChannelStats>,
}

struct ChannelStats {
    received: AtomicU64,
    sent: AtomicU64,
}

impl ChannelManager {
    /// Create a new channel manager with a message buffer.
    pub fn new(buffer_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer_size);
        Self {
            channels: HashMap::new(),
            message_tx: tx,
            message_rx: Some(rx),
        }
    }

    /// Register a channel. Does NOT start it yet.
    pub fn register(&mut self, channel: Box<dyn Channel>) {
        let name = channel.name().to_string();
        tracing::info!(channel = %name, channel_type = %channel.channel_type(), "Registered channel");
        self.channels.insert(
            name,
            ChannelEntry {
                channel,
                stats: Arc::new(ChannelStats {
                    received: AtomicU64::new(0),
                    sent: AtomicU64::new(0),
                }),
            },
        );
    }

    /// Start a specific channel.
    pub async fn start_channel(&mut self, name: &str) -> Result<()> {
        let entry = self
            .channels
            .get_mut(name)
            .ok_or_else(|| anyhow!("Channel '{}' not registered", name))?;

        if entry.channel.is_running() {
            return Err(anyhow!("Channel '{}' is already running", name));
        }

        let tx = self.message_tx.clone();
        entry.channel.start(tx).await?;
        tracing::info!(channel = name, "Channel started");
        Ok(())
    }

    /// Stop a specific channel.
    pub async fn stop_channel(&mut self, name: &str) -> Result<()> {
        let entry = self
            .channels
            .get_mut(name)
            .ok_or_else(|| anyhow!("Channel '{}' not registered", name))?;

        entry.channel.stop().await?;
        tracing::info!(channel = name, "Channel stopped");
        Ok(())
    }

    /// Start all registered channels.
    pub async fn start_all(&mut self) -> Result<()> {
        let names: Vec<String> = self.channels.keys().cloned().collect();
        for name in names {
            if let Err(e) = self.start_channel(&name).await {
                tracing::error!(channel = %name, error = %e, "Failed to start channel");
            }
        }
        Ok(())
    }

    /// Stop all running channels.
    pub async fn stop_all(&mut self) -> Result<()> {
        let names: Vec<String> = self.channels.keys().cloned().collect();
        for name in names {
            if let Err(e) = self.stop_channel(&name).await {
                tracing::warn!(channel = %name, error = %e, "Failed to stop channel");
            }
        }
        Ok(())
    }

    /// Take the message receiver (can only be called once).
    /// Use this in the agent loop to receive incoming messages.
    pub fn take_receiver(&mut self) -> Option<mpsc::Receiver<ChannelMessage>> {
        self.message_rx.take()
    }

    /// Send a response through the appropriate channel.
    pub async fn send_response(&self, response: &ChannelResponse) -> Result<()> {
        let entry = self
            .channels
            .get(&response.channel_name)
            .ok_or_else(|| {
                anyhow!(
                    "Channel '{}' not found for response",
                    response.channel_name
                )
            })?;

        entry.channel.send(response).await?;
        entry.stats.sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Get status of all channels.
    pub fn status(&self) -> Vec<ChannelStatus> {
        self.channels
            .iter()
            .map(|(name, entry)| ChannelStatus {
                name: name.clone(),
                channel_type: entry.channel.channel_type(),
                is_running: entry.channel.is_running(),
                messages_received: entry.stats.received.load(Ordering::Relaxed),
                messages_sent: entry.stats.sent.load(Ordering::Relaxed),
            })
            .collect()
    }

    /// List registered channel names.
    pub fn channel_names(&self) -> Vec<String> {
        self.channels.keys().cloned().collect()
    }
}
