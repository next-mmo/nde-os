//! Lifecycle event bus for NDE-OS app operations.
//!
//! Provides a typed event system that streams install/launch/stop
//! progress to the frontend (Tauri events or SSE). Similar to
//! Pinokio's step-by-step install progress UI.
//!
//! Usage:
//! ```ignore
//! let (bus, rx) = EventBus::new();
//! bus.emit(AppEvent::install_step("sample-node", 1, 5, "Creating sandbox..."));
//! // rx receives the event for streaming to UI
//! ```

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Maximum event buffer size before oldest events are dropped.
const EVENT_BUFFER: usize = 256;

/// All lifecycle events emitted by NDE-OS.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppEvent {
    /// An install/launch step completed.
    Step {
        app_id: String,
        phase: EventPhase,
        step: u32,
        total_steps: u32,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// A log line from a running process (stdout/stderr).
    Log {
        app_id: String,
        stream: LogStream,
        line: String,
    },
    /// App status changed.
    StatusChange {
        app_id: String,
        from: String,
        to: String,
    },
    /// An error occurred during a lifecycle operation.
    Error {
        app_id: String,
        phase: EventPhase,
        message: String,
    },
    /// An operation completed successfully.
    Complete {
        app_id: String,
        phase: EventPhase,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },
}

/// Which lifecycle phase the event belongs to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventPhase {
    Install,
    Launch,
    Stop,
    Uninstall,
    Update,
}

/// Which output stream a log line came from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogStream {
    Stdout,
    Stderr,
}

impl AppEvent {
    /// Convenience: create an install step event.
    pub fn install_step(app_id: &str, step: u32, total: u32, msg: &str) -> Self {
        Self::Step {
            app_id: app_id.to_string(),
            phase: EventPhase::Install,
            step,
            total_steps: total,
            message: msg.to_string(),
            detail: None,
        }
    }

    /// Convenience: create an install step with detail.
    pub fn install_step_detail(app_id: &str, step: u32, total: u32, msg: &str, detail: &str) -> Self {
        Self::Step {
            app_id: app_id.to_string(),
            phase: EventPhase::Install,
            step,
            total_steps: total,
            message: msg.to_string(),
            detail: Some(detail.to_string()),
        }
    }

    /// Convenience: create a launch step event.
    pub fn launch_step(app_id: &str, step: u32, total: u32, msg: &str) -> Self {
        Self::Step {
            app_id: app_id.to_string(),
            phase: EventPhase::Launch,
            step,
            total_steps: total,
            message: msg.to_string(),
            detail: None,
        }
    }

    /// Convenience: create an error event.
    pub fn error(app_id: &str, phase: EventPhase, msg: &str) -> Self {
        Self::Error {
            app_id: app_id.to_string(),
            phase,
            message: msg.to_string(),
        }
    }

    /// Convenience: create a completion event.
    pub fn complete(app_id: &str, phase: EventPhase, msg: &str, duration_ms: Option<u64>) -> Self {
        Self::Complete {
            app_id: app_id.to_string(),
            phase,
            message: msg.to_string(),
            duration_ms,
        }
    }

    /// Convenience: create a log event.
    pub fn log(app_id: &str, stream: LogStream, line: &str) -> Self {
        Self::Log {
            app_id: app_id.to_string(),
            stream,
            line: line.to_string(),
        }
    }

    /// Convenience: create a status change event.
    pub fn status_change(app_id: &str, from: &str, to: &str) -> Self {
        Self::StatusChange {
            app_id: app_id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    /// Get the app_id from any event variant.
    pub fn app_id(&self) -> &str {
        match self {
            Self::Step { app_id, .. }
            | Self::Log { app_id, .. }
            | Self::StatusChange { app_id, .. }
            | Self::Error { app_id, .. }
            | Self::Complete { app_id, .. } => app_id,
        }
    }
}

/// Broadcast event bus. Clone-cheap (Arc internally).
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AppEvent>,
}

impl EventBus {
    /// Create a new event bus and return (bus, receiver).
    pub fn new() -> (Self, broadcast::Receiver<AppEvent>) {
        let (tx, rx) = broadcast::channel(EVENT_BUFFER);
        (Self { tx }, rx)
    }

    /// Create a bus from an existing sender (for sharing across threads).
    pub fn from_sender(tx: broadcast::Sender<AppEvent>) -> Self {
        Self { tx }
    }

    /// Emit an event. Returns Err only if there are no receivers.
    pub fn emit(&self, event: AppEvent) {
        // Ignore error when no receivers are listening
        let _ = self.tx.send(event);
    }

    /// Subscribe to events. Returns a new receiver.
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }

    /// Get the underlying sender (for sharing with other systems).
    pub fn sender(&self) -> broadcast::Sender<AppEvent> {
        self.tx.clone()
    }

    /// Number of active receivers.
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new().0
    }
}

/// Global event bus instance (wrapped in Arc for thread-safe sharing).
pub type SharedEventBus = Arc<EventBus>;

/// Create a shared event bus.
pub fn shared_event_bus() -> (SharedEventBus, broadcast::Receiver<AppEvent>) {
    let (bus, rx) = EventBus::new();
    (Arc::new(bus), rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_send_receive() {
        let (bus, mut rx) = EventBus::new();
        bus.emit(AppEvent::install_step("test-app", 1, 3, "Creating sandbox..."));

        let event = rx.recv().await.unwrap();
        assert_eq!(event.app_id(), "test-app");
        match event {
            AppEvent::Step { step, total_steps, .. } => {
                assert_eq!(step, 1);
                assert_eq!(total_steps, 3);
            }
            _ => panic!("Expected Step event"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let (bus, mut rx1) = EventBus::new();
        let mut rx2 = bus.subscribe();

        bus.emit(AppEvent::status_change("app1", "Installed", "Running"));

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert_eq!(e1.app_id(), "app1");
        assert_eq!(e2.app_id(), "app1");
    }

    #[test]
    fn test_event_serialization() {
        let event = AppEvent::install_step_detail(
            "my-app",
            2,
            5,
            "Installing dependencies",
            "pnpm install",
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"step\""));
        assert!(json.contains("\"install\""));
        assert!(json.contains("pnpm install"));
    }

    #[test]
    fn test_error_event() {
        let event = AppEvent::error("bad-app", EventPhase::Install, "Python not found");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("Python not found"));
    }

    #[test]
    fn test_emit_without_receivers() {
        let bus = EventBus::default();
        // Should not panic even without receivers
        bus.emit(AppEvent::install_step("orphan", 1, 1, "test"));
    }
}
