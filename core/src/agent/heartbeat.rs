//! Background heartbeat monitor for 24/7 agent operation.
//!
//! The heartbeat:
//! - Emits periodic `AgentEvent::Heartbeat` for each running task
//! - Detects stuck tasks (no progress for >60s)
//! - Auto-recovers stale tasks by cancelling and re-queuing
//! - Runs periodic cleanup of old completed tasks
//! - Tracks per-task memory via sysinfo

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use super::protocol::AgentEvent;
use super::store::TaskStore;

/// Tracks the last activity time for each running task.
#[derive(Debug)]
struct TaskActivity {
    last_event: Instant,
    iterations: u32,
    tokens_used: u64,
    started_at: Instant,
}

/// Configuration for the heartbeat monitor.
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Heartbeat emission interval.
    pub interval: Duration,
    /// If no event for this duration, mark task as stale.
    pub stale_threshold: Duration,
    /// Run cleanup every N heartbeat cycles.
    pub cleanup_every_cycles: u32,
    /// Delete completed tasks older than N days.
    pub cleanup_age_days: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(10),
            stale_threshold: Duration::from_secs(60),
            cleanup_every_cycles: 360, // every hour at 10s interval
            cleanup_age_days: 7,
        }
    }
}

/// Handle returned when starting the heartbeat monitor.
/// Drop this to stop the background task.
pub struct HeartbeatHandle {
    cancel: CancellationToken,
    _task: tokio::task::JoinHandle<()>,
}

impl HeartbeatHandle {
    /// Stop the heartbeat monitor.
    pub fn stop(&self) {
        self.cancel.cancel();
    }
}

/// Shared state for tracking task activity.
pub struct HeartbeatTracker {
    activities: Mutex<HashMap<String, TaskActivity>>,
}

impl HeartbeatTracker {
    pub fn new() -> Self {
        Self {
            activities: Mutex::new(HashMap::new()),
        }
    }

    /// Register a new task for tracking.
    pub async fn register_task(&self, task_id: &str) {
        let mut map = self.activities.lock().await;
        map.insert(
            task_id.to_string(),
            TaskActivity {
                last_event: Instant::now(),
                iterations: 0,
                tokens_used: 0,
                started_at: Instant::now(),
            },
        );
    }

    /// Record activity for a task (called when events are emitted).
    pub async fn record_activity(&self, task_id: &str, iterations: u32, tokens_used: u64) {
        let mut map = self.activities.lock().await;
        if let Some(activity) = map.get_mut(task_id) {
            activity.last_event = Instant::now();
            activity.iterations = iterations;
            activity.tokens_used = tokens_used;
        }
    }

    /// Remove a task from tracking (called when task completes).
    pub async fn unregister_task(&self, task_id: &str) {
        let mut map = self.activities.lock().await;
        map.remove(task_id);
    }

    /// Get stale task IDs (no activity for longer than threshold).
    async fn get_stale_tasks(&self, threshold: Duration) -> Vec<String> {
        let map = self.activities.lock().await;
        map.iter()
            .filter(|(_, activity)| activity.last_event.elapsed() > threshold)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get current snapshot for heartbeat emission.
    async fn snapshot(&self) -> Vec<(String, u64, u32, u64)> {
        let map = self.activities.lock().await;
        map.iter()
            .map(|(id, a)| {
                (
                    id.clone(),
                    a.started_at.elapsed().as_secs(),
                    a.iterations,
                    a.tokens_used,
                )
            })
            .collect()
    }
}

/// Start the heartbeat monitor as a background tokio task.
pub fn start_heartbeat(
    config: HeartbeatConfig,
    tracker: Arc<HeartbeatTracker>,
    store: Arc<TaskStore>,
    event_tx: broadcast::Sender<AgentEvent>,
    stale_tx: mpsc::Sender<String>,
) -> HeartbeatHandle {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    let task = tokio::spawn(async move {
        let mut cycle = 0u32;
        let mut sys = sysinfo::System::new();

        loop {
            tokio::select! {
                _ = tokio::time::sleep(config.interval) => {},
                _ = cancel_clone.cancelled() => break,
            }

            cycle += 1;

            // Refresh system info for memory tracking
            sys.refresh_memory();
            let process_memory = sys.used_memory();

            // Emit heartbeat for each tracked task
            let tasks = tracker.snapshot().await;
            for (task_id, uptime_secs, iterations, tokens_used) in &tasks {
                let event = AgentEvent::heartbeat(
                    task_id,
                    *uptime_secs,
                    *iterations,
                    *tokens_used,
                    process_memory,
                );
                let _ = event_tx.send(event);
            }

            // Detect stale tasks
            let stale = tracker.get_stale_tasks(config.stale_threshold).await;
            for task_id in stale {
                tracing::warn!(task_id = %task_id, "Detected stale task — requesting recovery");
                let _ = stale_tx.send(task_id).await;
            }

            // Periodic cleanup
            if cycle % config.cleanup_every_cycles == 0 {
                match store.cleanup_old(config.cleanup_age_days) {
                    Ok(deleted) if deleted > 0 => {
                        tracing::info!(deleted, "Cleaned up old tasks");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Task cleanup failed");
                    }
                    _ => {}
                }
            }
        }

        tracing::info!("Heartbeat monitor stopped");
    });

    HeartbeatHandle {
        cancel,
        _task: task,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tracker_register_unregister() {
        let tracker = HeartbeatTracker::new();
        tracker.register_task("task-1").await;
        tracker.register_task("task-2").await;

        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot.len(), 2);

        tracker.unregister_task("task-1").await;
        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot.len(), 1);
    }

    #[tokio::test]
    async fn test_tracker_activity() {
        let tracker = HeartbeatTracker::new();
        tracker.register_task("task-1").await;
        tracker.record_activity("task-1", 5, 1000).await;

        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot[0].2, 5); // iterations
        assert_eq!(snapshot[0].3, 1000); // tokens
    }

    #[tokio::test]
    async fn test_stale_detection() {
        let tracker = HeartbeatTracker::new();
        tracker.register_task("task-1").await;

        // Not stale with a long threshold
        let stale = tracker.get_stale_tasks(Duration::from_secs(60)).await;
        assert!(stale.is_empty());

        // Stale with zero threshold
        let stale = tracker.get_stale_tasks(Duration::from_nanos(1)).await;
        assert_eq!(stale.len(), 1);
    }
}
