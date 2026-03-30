//! Typed SSE event protocol for agent communication.
//!
//! Every event the agent can emit is represented as a variant of [`AgentEvent`].
//! These events are serialized to SSE `data:` lines for streaming to clients.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::models::TaskState;
use crate::llm::Usage;

/// A single event emitted by the agent runtime.
///
/// Clients receive these as SSE `data:` lines. Each event carries the `task_id`
/// so multiple tasks can be multiplexed on a single connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    // ── Lifecycle ────────────────────────────────────────────────────────
    /// Task has been created and queued.
    TaskCreated {
        task_id: String,
        input: String,
        timestamp: DateTime<Utc>,
    },

    /// Task execution has started.
    TaskStarted {
        task_id: String,
        timestamp: DateTime<Utc>,
    },

    /// Task completed successfully.
    TaskCompleted {
        task_id: String,
        output: String,
        tokens_used: u64,
        tool_calls_made: u32,
        iterations: u32,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },

    /// Task failed (may retry).
    TaskFailed {
        task_id: String,
        error: String,
        retry_count: u32,
        will_retry: bool,
        timestamp: DateTime<Utc>,
    },

    /// Task was cancelled by user or system.
    TaskCancelled {
        task_id: String,
        timestamp: DateTime<Utc>,
    },

    /// Task exceeded its timeout.
    TaskTimedOut {
        task_id: String,
        timeout_secs: u64,
        timestamp: DateTime<Utc>,
    },

    /// Task state changed (generic).
    TaskStateChanged {
        task_id: String,
        old_state: TaskState,
        new_state: TaskState,
        timestamp: DateTime<Utc>,
    },

    // ── Streaming content ───────────────────────────────────────────────
    /// A text delta from the LLM (token-level streaming).
    TextDelta { task_id: String, content: String },

    /// A tool call is about to be executed.
    ToolCallStart {
        task_id: String,
        tool_name: String,
        arguments: serde_json::Value,
    },

    /// A tool call has completed.
    ToolCallResult {
        task_id: String,
        tool_name: String,
        output: String,
        is_error: bool,
        duration_ms: u64,
    },

    /// An agent loop iteration has completed.
    IterationComplete {
        task_id: String,
        iteration: u32,
        tokens_used: u64,
    },

    // ── Health ───────────────────────────────────────────────────────────
    /// Periodic heartbeat for active tasks.
    Heartbeat {
        task_id: String,
        uptime_secs: u64,
        iterations: u32,
        tokens_used: u64,
        memory_bytes: u64,
        timestamp: DateTime<Utc>,
    },
}

impl AgentEvent {
    /// Get the task ID this event belongs to.
    pub fn task_id(&self) -> &str {
        match self {
            AgentEvent::TaskCreated { task_id, .. }
            | AgentEvent::TaskStarted { task_id, .. }
            | AgentEvent::TaskCompleted { task_id, .. }
            | AgentEvent::TaskFailed { task_id, .. }
            | AgentEvent::TaskCancelled { task_id, .. }
            | AgentEvent::TaskTimedOut { task_id, .. }
            | AgentEvent::TaskStateChanged { task_id, .. }
            | AgentEvent::TextDelta { task_id, .. }
            | AgentEvent::ToolCallStart { task_id, .. }
            | AgentEvent::ToolCallResult { task_id, .. }
            | AgentEvent::IterationComplete { task_id, .. }
            | AgentEvent::Heartbeat { task_id, .. } => task_id,
        }
    }

    /// Serialize this event to an SSE `data:` line.
    pub fn to_sse(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());
        format!("data: {}\n\n", json)
    }

    /// Whether this event indicates the task is done (terminal).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentEvent::TaskCompleted { .. }
                | AgentEvent::TaskFailed {
                    will_retry: false,
                    ..
                }
                | AgentEvent::TaskCancelled { .. }
                | AgentEvent::TaskTimedOut { .. }
        )
    }
}

// ── Helper constructors ─────────────────────────────────────────────────────

impl AgentEvent {
    pub fn created(task_id: &str, input: &str) -> Self {
        AgentEvent::TaskCreated {
            task_id: task_id.to_string(),
            input: input.to_string(),
            timestamp: Utc::now(),
        }
    }

    pub fn started(task_id: &str) -> Self {
        AgentEvent::TaskStarted {
            task_id: task_id.to_string(),
            timestamp: Utc::now(),
        }
    }

    pub fn text_delta(task_id: &str, content: &str) -> Self {
        AgentEvent::TextDelta {
            task_id: task_id.to_string(),
            content: content.to_string(),
        }
    }

    pub fn tool_start(task_id: &str, tool_name: &str, arguments: serde_json::Value) -> Self {
        AgentEvent::ToolCallStart {
            task_id: task_id.to_string(),
            tool_name: tool_name.to_string(),
            arguments,
        }
    }

    pub fn tool_result(
        task_id: &str,
        tool_name: &str,
        output: &str,
        is_error: bool,
        duration_ms: u64,
    ) -> Self {
        AgentEvent::ToolCallResult {
            task_id: task_id.to_string(),
            tool_name: tool_name.to_string(),
            output: output.to_string(),
            is_error,
            duration_ms,
        }
    }

    pub fn completed(
        task_id: &str,
        output: &str,
        tokens_used: u64,
        tool_calls_made: u32,
        iterations: u32,
        duration_ms: u64,
    ) -> Self {
        AgentEvent::TaskCompleted {
            task_id: task_id.to_string(),
            output: output.to_string(),
            tokens_used,
            tool_calls_made,
            iterations,
            duration_ms,
            timestamp: Utc::now(),
        }
    }

    pub fn failed(task_id: &str, error: &str, retry_count: u32, will_retry: bool) -> Self {
        AgentEvent::TaskFailed {
            task_id: task_id.to_string(),
            error: error.to_string(),
            retry_count,
            will_retry,
            timestamp: Utc::now(),
        }
    }

    pub fn cancelled(task_id: &str) -> Self {
        AgentEvent::TaskCancelled {
            task_id: task_id.to_string(),
            timestamp: Utc::now(),
        }
    }

    pub fn heartbeat(
        task_id: &str,
        uptime_secs: u64,
        iterations: u32,
        tokens_used: u64,
        memory_bytes: u64,
    ) -> Self {
        AgentEvent::Heartbeat {
            task_id: task_id.to_string(),
            uptime_secs,
            iterations,
            tokens_used,
            memory_bytes,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = AgentEvent::text_delta("task-123", "Hello ");
        let sse = event.to_sse();
        assert!(sse.starts_with("data: "));
        assert!(sse.contains("text_delta"));
        assert!(sse.contains("Hello "));
    }

    #[test]
    fn test_event_task_id() {
        let event = AgentEvent::started("my-task");
        assert_eq!(event.task_id(), "my-task");
    }

    #[test]
    fn test_terminal_events() {
        assert!(AgentEvent::completed("t", "out", 0, 0, 0, 0).is_terminal());
        assert!(AgentEvent::cancelled("t").is_terminal());
        assert!(!AgentEvent::started("t").is_terminal());
        assert!(!AgentEvent::failed("t", "err", 1, true).is_terminal()); // will retry
        assert!(AgentEvent::failed("t", "err", 3, false).is_terminal()); // no retry
    }
}
