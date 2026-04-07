/// Shared gateway log buffer — a thread-safe ring buffer for backend events.
///
/// Sources push structured log entries here; the frontend polls
/// `GET /api/logs?since=<id>` to receive new entries.
use serde::Serialize;
use std::sync::Mutex;

const MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone, Serialize)]
pub struct GatewayLogEntry {
    pub id: u64,
    pub timestamp: String,
    pub level: &'static str,   // "info" | "success" | "warning" | "error"
    pub source: &'static str,  // "gateway" | "telegram" | "chat" | "system"
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
}

pub struct GatewayLogBuffer {
    entries: Vec<GatewayLogEntry>,
    next_id: u64,
}

impl GatewayLogBuffer {
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(MAX_ENTRIES),
            next_id: 1,
        }
    }

    /// Push a log entry with optional app_id.
    pub fn push(&mut self, level: &'static str, source: &'static str, message: String, app_id: Option<&str>) {
        let entry = GatewayLogEntry {
            id: self.next_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
            level,
            source,
            message,
            app_id: app_id.map(str::to_string),
        };
        self.next_id += 1;
        self.entries.push(entry);
        if self.entries.len() > MAX_ENTRIES {
            let drain_count = self.entries.len() - MAX_ENTRIES;
            self.entries.drain(..drain_count);
        }
    }

    /// Get all entries with id > since_id.
    pub fn since(&self, since_id: u64) -> Vec<GatewayLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.id > since_id)
            .cloned()
            .collect()
    }
}

/// Thread-safe shared log buffer.
pub type SharedLogBuffer = std::sync::Arc<Mutex<GatewayLogBuffer>>;

pub fn new_shared() -> SharedLogBuffer {
    std::sync::Arc::new(Mutex::new(GatewayLogBuffer::new()))
}

// ── Convenience push helpers (avoid holding lock manually) ──────────────────

pub fn log_info(buf: &SharedLogBuffer, source: &'static str, msg: impl Into<String>) {
    if let Ok(mut b) = buf.lock() {
        b.push("info", source, msg.into(), None);
    }
}

pub fn log_success(buf: &SharedLogBuffer, source: &'static str, msg: impl Into<String>) {
    if let Ok(mut b) = buf.lock() {
        b.push("success", source, msg.into(), None);
    }
}

pub fn log_warn(buf: &SharedLogBuffer, source: &'static str, msg: impl Into<String>) {
    if let Ok(mut b) = buf.lock() {
        b.push("warning", source, msg.into(), None);
    }
}

pub fn log_error(buf: &SharedLogBuffer, source: &'static str, msg: impl Into<String>) {
    if let Ok(mut b) = buf.lock() {
        b.push("error", source, msg.into(), None);
    }
}
