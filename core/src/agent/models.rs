//! Agent task state machine types.
//!
//! Every agent invocation is modelled as an `AgentTask` that transitions through
//! a well-defined state machine: Pending → Running → Completed/Failed/Cancelled/TimedOut.
//! Tasks can also be Paused and later Resumed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ── Task state machine ──────────────────────────────────────────────────────

/// Lifecycle state of an agent task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Queued but not yet started.
    Pending,
    /// Actively running the agent loop.
    Running,
    /// Paused by user — can be resumed.
    Paused,
    /// Completed successfully with output.
    Completed,
    /// Failed after exhausting retries.
    Failed,
    /// Cancelled by user or system.
    Cancelled,
    /// Exceeded the configured timeout.
    TimedOut,
}

impl TaskState {
    /// Whether the task is in a terminal state.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled | TaskState::TimedOut
        )
    }

    /// Whether the task is actively doing work.
    pub fn is_active(self) -> bool {
        matches!(self, TaskState::Running)
    }
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TaskState::Pending => "pending",
            TaskState::Running => "running",
            TaskState::Paused => "paused",
            TaskState::Completed => "completed",
            TaskState::Failed => "failed",
            TaskState::Cancelled => "cancelled",
            TaskState::TimedOut => "timed_out",
        };
        f.write_str(s)
    }
}

// ── Agent task ──────────────────────────────────────────────────────────────

/// A single unit of work submitted to the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Unique task identifier (UUID v4).
    pub id: String,

    /// Current lifecycle state.
    pub state: TaskState,

    /// The user's input message.
    pub input: String,

    /// Final output (populated on completion).
    pub output: Option<String>,

    /// Error message (populated on failure).
    pub error: Option<String>,

    /// Linked conversation for message persistence.
    pub conversation_id: Option<String>,

    /// Parent task ID for chained / sub-task execution.
    pub parent_task_id: Option<String>,

    // ── Retry & timeout ─────────────────────────────────────────────────
    /// How many times this task has been retried.
    pub retry_count: u32,

    /// Maximum retry attempts before moving to `Failed`.
    pub max_retries: u32,

    /// Per-task timeout in seconds (0 = no timeout).
    pub timeout_secs: u64,

    // ── Metrics ─────────────────────────────────────────────────────────
    /// Number of agent loop iterations completed.
    pub iterations: u32,

    /// Total LLM tokens consumed.
    pub tokens_used: u64,

    /// Number of tool calls executed.
    pub tool_calls_made: u32,

    // ── Timestamps ──────────────────────────────────────────────────────
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl AgentTask {
    /// Create a new task in `Pending` state.
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            state: TaskState::Pending,
            input: input.into(),
            output: None,
            error: None,
            conversation_id: None,
            parent_task_id: None,
            retry_count: 0,
            max_retries: 3,
            timeout_secs: 300, // 5 minutes default
            iterations: 0,
            tokens_used: 0,
            tool_calls_made: 0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Builder: set conversation ID.
    pub fn with_conversation(mut self, id: impl Into<String>) -> Self {
        self.conversation_id = Some(id.into());
        self
    }

    /// Builder: set parent task for chaining.
    pub fn with_parent(mut self, id: impl Into<String>) -> Self {
        self.parent_task_id = Some(id.into());
        self
    }

    /// Builder: set timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_secs = timeout.as_secs();
        self
    }

    /// Builder: set max retries.
    pub fn with_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Transition to Running.
    pub fn mark_running(&mut self) {
        self.state = TaskState::Running;
        self.started_at = Some(Utc::now());
    }

    /// Transition to Paused.
    pub fn mark_paused(&mut self) {
        self.state = TaskState::Paused;
    }

    /// Transition to Completed with output.
    pub fn mark_completed(&mut self, output: impl Into<String>) {
        self.state = TaskState::Completed;
        self.output = Some(output.into());
        self.completed_at = Some(Utc::now());
    }

    /// Transition to Failed with error.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.state = TaskState::Failed;
        self.error = Some(error.into());
        self.completed_at = Some(Utc::now());
    }

    /// Transition to Cancelled.
    pub fn mark_cancelled(&mut self) {
        self.state = TaskState::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    /// Transition to TimedOut.
    pub fn mark_timed_out(&mut self) {
        self.state = TaskState::TimedOut;
        self.error = Some("Task exceeded timeout".into());
        self.completed_at = Some(Utc::now());
    }

    /// Whether the task can be retried.
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Elapsed duration since started (or 0 if not started).
    pub fn elapsed(&self) -> Duration {
        match self.started_at {
            Some(started) => {
                let end = self.completed_at.unwrap_or_else(Utc::now);
                (end - started).to_std().unwrap_or_default()
            }
            None => Duration::ZERO,
        }
    }

    /// Whether the task has exceeded its timeout.
    pub fn is_timed_out(&self) -> bool {
        self.timeout_secs > 0 && self.elapsed().as_secs() > self.timeout_secs
    }
}

// ── Task filter for queries ─────────────────────────────────────────────────

/// Filter for querying tasks from the store.
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    pub state: Option<TaskState>,
    pub conversation_id: Option<String>,
    pub limit: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_lifecycle() {
        let mut task = AgentTask::new("Hello agent");
        assert_eq!(task.state, TaskState::Pending);
        assert!(!task.state.is_terminal());

        task.mark_running();
        assert_eq!(task.state, TaskState::Running);
        assert!(task.started_at.is_some());

        task.mark_completed("Done!");
        assert_eq!(task.state, TaskState::Completed);
        assert!(task.state.is_terminal());
        assert_eq!(task.output.as_deref(), Some("Done!"));
    }

    #[test]
    fn test_task_retry() {
        let task = AgentTask::new("test").with_retries(3);
        assert!(task.can_retry());
        assert_eq!(task.max_retries, 3);
    }

    #[test]
    fn test_task_timeout() {
        let task = AgentTask::new("test").with_timeout(Duration::from_secs(10));
        assert_eq!(task.timeout_secs, 10);
        // Not timed out yet (not started)
        assert!(!task.is_timed_out());
    }

    #[test]
    fn test_task_chaining() {
        let parent = AgentTask::new("parent");
        let child = AgentTask::new("child").with_parent(&parent.id);
        assert_eq!(child.parent_task_id.as_deref(), Some(parent.id.as_str()));
    }
}
