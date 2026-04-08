pub mod commands;
pub mod log;
pub mod session;
pub mod telegram;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Shared gateway state for live channel status and counters.
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

    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

pub use log::{log_info, new_shared, SharedLogBuffer};
pub use telegram::{start_telegram_gateway, TelegramGatewayConfig};
