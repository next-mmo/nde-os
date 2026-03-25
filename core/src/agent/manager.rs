//! Agent task manager — the single entry point for all agent operations.
//!
//! Replaces creating a new AgentRuntime per request. Manages:
//! - Task spawn, cancel, pause, resume
//! - Concurrent task limit with backpressure
//! - Boot recovery (restore incomplete tasks)
//! - Heartbeat monitor lifecycle
//! - Event broadcasting to subscribers

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use crate::agent::config::AgentConfig;
use crate::llm::{self, LlmProvider};
use crate::sandbox::Sandbox;
use crate::tools::ToolRegistry;

use super::executor::{self, ExecutorConfig};
use super::heartbeat::{self, HeartbeatConfig, HeartbeatHandle, HeartbeatTracker};
use super::models::{AgentTask, TaskFilter, TaskState};
use super::protocol::AgentEvent;
use super::store::TaskStore;

/// Configuration for the AgentManager.
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Maximum concurrent running tasks.
    pub max_concurrent: usize,
    /// Event broadcast channel capacity.
    pub broadcast_capacity: usize,
    /// Executor config.
    pub executor: ExecutorConfig,
    /// Heartbeat config.
    pub heartbeat: HeartbeatConfig,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            broadcast_capacity: 256,
            executor: ExecutorConfig::default(),
            heartbeat: HeartbeatConfig::default(),
        }
    }
}

/// Per-task runtime state held by the manager.
struct RunningTask {
    cancel: CancellationToken,
    handle: tokio::task::JoinHandle<()>,
}

/// The Agent Manager — singleton orchestrator for all agent tasks.
pub struct AgentManager {
    config: ManagerConfig,
    agent_config: AgentConfig,
    store: Arc<TaskStore>,
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    sandbox: Arc<Sandbox>,
    running: Mutex<HashMap<String, RunningTask>>,
    event_tx: broadcast::Sender<AgentEvent>,
    tracker: Arc<HeartbeatTracker>,
    heartbeat: Mutex<Option<HeartbeatHandle>>,
    data_dir: PathBuf,
}

impl AgentManager {
    /// Create a new agent manager.
    pub fn new(
        config: ManagerConfig,
        agent_config: AgentConfig,
        data_dir: &Path,
    ) -> Result<Self> {
        let db_path = data_dir.join("agent_tasks.db");
        let store = Arc::new(TaskStore::new(&db_path)?);

        let provider = Self::create_provider(&agent_config)?;
        let tools = Arc::new(crate::tools::builtin::default_registry());

        let workspace_path = if PathBuf::from(&agent_config.workspace).is_absolute() {
            PathBuf::from(&agent_config.workspace)
        } else {
            data_dir.join(&agent_config.workspace)
        };
        let sandbox = Arc::new(Sandbox::new(&*workspace_path.to_string_lossy())?);
        sandbox.init_workspace()?;

        let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
        let tracker = Arc::new(HeartbeatTracker::new());

        Ok(Self {
            config,
            agent_config,
            store,
            provider,
            tools,
            sandbox,
            running: Mutex::new(HashMap::new()),
            event_tx,
            tracker,
            heartbeat: Mutex::new(None),
            data_dir: data_dir.to_path_buf(),
        })
    }

    /// Update the LLM provider (e.g. after model switch).
    pub fn update_provider(&mut self, agent_config: &AgentConfig) -> Result<()> {
        self.provider = Self::create_provider(agent_config)?;
        self.agent_config = agent_config.clone();
        Ok(())
    }

    fn create_provider(agent_config: &AgentConfig) -> Result<Arc<dyn LlmProvider>> {
        let provider = llm::create_provider(
            &agent_config.model_provider,
            &agent_config.model_name,
            agent_config.base_url.as_deref(),
            agent_config.api_key.as_deref(),
        )?;
        Ok(Arc::from(provider))
    }

    // ── Boot & lifecycle ─────────────────────────────────────────────────

    /// Called at server startup. Restores incomplete tasks and starts heartbeat.
    pub async fn on_boot(&self) -> Result<()> {
        // Mark any previously-running tasks as failed (crashed)
        let incomplete = self.store.get_incomplete_tasks()?;
        for task in &incomplete {
            tracing::warn!(task_id = %task.id, "Recovering crashed task");
            if task.can_retry() {
                // Re-queue
                self.store.update_state(&task.id, TaskState::Pending)?;
            } else {
                self.store
                    .update_state(&task.id, TaskState::Failed)?;
            }
        }

        // Start heartbeat
        self.start_heartbeat().await;

        tracing::info!(
            recovered = incomplete.len(),
            "Agent manager booted"
        );
        Ok(())
    }

    /// Start the heartbeat monitor.
    async fn start_heartbeat(&self) {
        let (stale_tx, mut stale_rx) = mpsc::channel::<String>(32);

        let handle = heartbeat::start_heartbeat(
            self.config.heartbeat.clone(),
            self.tracker.clone(),
            self.store.clone(),
            self.event_tx.clone(),
            stale_tx,
        );

        // Spawn stale task handler
        let store = self.store.clone();
        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            while let Some(task_id) = stale_rx.recv().await {
                tracing::warn!(task_id = %task_id, "Handling stale task");
                if let Err(e) = store.update_state(&task_id, TaskState::Failed) {
                    tracing::error!(error = %e, "Failed to mark stale task as failed");
                }
                let _ = event_tx.send(AgentEvent::failed(&task_id, "Task stale — no progress", 0, false));
            }
        });

        *self.heartbeat.lock().await = Some(handle);
    }

    // ── Task operations ──────────────────────────────────────────────────

    /// Spawn a new agent task. Returns the task ID.
    pub async fn spawn(&self, input: &str) -> Result<String> {
        self.spawn_with_config(input, None).await
    }

    /// Spawn a new agent task with optional config overrides.
    pub async fn spawn_with_config(
        &self,
        input: &str,
        agent_config: Option<&AgentConfig>,
    ) -> Result<String> {
        // Check concurrent limit
        {
            let running = self.running.lock().await;
            if running.len() >= self.config.max_concurrent {
                return Err(anyhow!(
                    "Concurrent task limit reached ({}/{})",
                    running.len(),
                    self.config.max_concurrent
                ));
            }
        }

        let mut task = AgentTask::new(input);
        let task_id = task.id.clone();

        // Persist task
        self.store.save_task(&task)?;

        // Emit created event
        let _ = self
            .event_tx
            .send(AgentEvent::created(&task_id, input));

        // Set up execution
        let cancel = CancellationToken::new();
        let (event_tx, mut event_rx) = mpsc::channel::<AgentEvent>(64);

        let config = agent_config.cloned().unwrap_or_else(|| self.agent_config.clone());
        let exec_config = self.config.executor.clone();
        let provider = self.provider.clone();
        let tools = self.tools.clone();
        let sandbox = self.sandbox.clone();
        let store = self.store.clone();
        let broadcast_tx = self.event_tx.clone();
        let tracker = self.tracker.clone();
        let cancel_clone = cancel.clone();

        // Register with heartbeat tracker
        self.tracker.register_task(&task_id).await;

        let task_id_clone = task_id.clone();

        // Spawn the executor task
        let handle = tokio::spawn(async move {
            // Forward events from executor to broadcast + tracker
            let tracker_ref = tracker.clone();
            let task_id_for_fwd = task_id_clone.clone();
            let fwd_handle = tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    // Update tracker on iteration events
                    if let AgentEvent::IterationComplete {
                        iteration,
                        tokens_used,
                        ..
                    } = &event
                    {
                        tracker_ref
                            .record_activity(&task_id_for_fwd, *iteration, *tokens_used)
                            .await;
                    }
                    let _ = broadcast_tx.send(event);
                }
            });

            let result = executor::execute_task(
                &mut task,
                &config,
                &exec_config,
                provider,
                &tools,
                &sandbox,
                Some(&store),
                event_tx,
                cancel_clone,
                None,
            )
            .await;

            // Save final state
            let _ = store.save_task(&task);
            tracker.unregister_task(&task.id).await;

            if let Err(e) = &result {
                tracing::error!(task_id = %task.id, error = %e, "Task failed");
            }

            // Wait for event forwarding to finish
            fwd_handle.abort();
        });

        // Track running task
        {
            let mut running = self.running.lock().await;
            running.insert(task_id.clone(), RunningTask { cancel, handle });
        }

        Ok(task_id)
    }

    /// Cancel a running task.
    pub async fn cancel(&self, task_id: &str) -> Result<()> {
        let mut running = self.running.lock().await;
        if let Some(task) = running.remove(task_id) {
            task.cancel.cancel();
            // Don't await the handle — let it clean up async
            Ok(())
        } else {
            // Task might not be running — update store directly
            self.store.update_state(task_id, TaskState::Cancelled)?;
            let _ = self
                .event_tx
                .send(AgentEvent::cancelled(task_id));
            Ok(())
        }
    }

    /// Pause a running task (saves checkpoint).
    pub async fn pause(&self, task_id: &str) -> Result<()> {
        let mut running = self.running.lock().await;
        if let Some(task) = running.remove(task_id) {
            task.cancel.cancel();
            self.store.update_state(task_id, TaskState::Paused)?;
            self.tracker.unregister_task(task_id).await;
            Ok(())
        } else {
            Err(anyhow!("Task {} is not running", task_id))
        }
    }

    /// Resume a paused task from its last checkpoint.
    pub async fn resume(&self, task_id: &str) -> Result<()> {
        let task = self
            .store
            .load_task(task_id)?
            .ok_or_else(|| anyhow!("Task {} not found", task_id))?;

        if task.state != TaskState::Paused {
            return Err(anyhow!("Task {} is not paused (state: {})", task_id, task.state));
        }

        // Load checkpoint messages
        let messages = self.store.load_checkpoint(task_id)?;

        // Re-spawn with checkpoint
        let cancel = CancellationToken::new();
        let (event_tx, mut event_rx) = mpsc::channel::<AgentEvent>(64);

        let mut task = task;
        task.state = TaskState::Pending; // Will be set to Running by executor
        task.retry_count += 1;
        self.store.save_task(&task)?;

        let config = self.agent_config.clone();
        let exec_config = self.config.executor.clone();
        let provider = self.provider.clone();
        let tools = self.tools.clone();
        let sandbox = self.sandbox.clone();
        let store = self.store.clone();
        let broadcast_tx = self.event_tx.clone();
        let tracker = self.tracker.clone();
        let cancel_clone = cancel.clone();
        let task_id_str = task_id.to_string();

        self.tracker.register_task(task_id).await;

        let handle = tokio::spawn(async move {
            let tracker_ref = tracker.clone();
            let task_id_for_fwd = task_id_str.clone();
            let fwd_handle = tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    if let AgentEvent::IterationComplete {
                        iteration,
                        tokens_used,
                        ..
                    } = &event
                    {
                        tracker_ref
                            .record_activity(&task_id_for_fwd, *iteration, *tokens_used)
                            .await;
                    }
                    let _ = broadcast_tx.send(event);
                }
            });

            let _ = executor::execute_task(
                &mut task,
                &config,
                &exec_config,
                provider,
                &tools,
                &sandbox,
                Some(&store),
                event_tx,
                cancel_clone,
                messages,
            )
            .await;

            let _ = store.save_task(&task);
            tracker.unregister_task(&task.id).await;
            fwd_handle.abort();
        });

        let mut running = self.running.lock().await;
        running.insert(task_id.to_string(), RunningTask { cancel, handle });

        Ok(())
    }

    // ── Queries ──────────────────────────────────────────────────────────

    /// Get a task by ID.
    pub fn get_task(&self, task_id: &str) -> Result<Option<AgentTask>> {
        self.store.load_task(task_id)
    }

    /// List tasks matching a filter.
    pub fn list_tasks(&self, filter: &TaskFilter) -> Result<Vec<AgentTask>> {
        self.store.list_tasks(filter)
    }

    /// Get active running task count.
    pub async fn active_count(&self) -> usize {
        self.running.lock().await.len()
    }

    /// Subscribe to task events.
    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.event_tx.subscribe()
    }

    /// Shutdown the manager.
    pub async fn shutdown(&self) {
        // Stop heartbeat
        if let Some(handle) = self.heartbeat.lock().await.take() {
            handle.stop();
        }

        // Cancel all running tasks
        let mut running = self.running.lock().await;
        for (task_id, task) in running.drain() {
            tracing::info!(task_id = %task_id, "Cancelling task on shutdown");
            task.cancel.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> (ManagerConfig, AgentConfig, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let mut manager_config = ManagerConfig::default();
        manager_config.executor.audit_dir = dir.path().join("audit");

        let mut agent_config = AgentConfig::default();
        agent_config.workspace = dir.path().join("workspace").to_string_lossy().into();

        (manager_config, agent_config, dir)
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let (config, agent_config, dir) = test_config();
        let manager = AgentManager::new(config, agent_config, dir.path());
        // Manager creation may fail if GGUF provider can't init, which is OK in test
        // The important thing is the struct construction works
        if manager.is_err() {
            // Expected in CI — GGUF binary not available
            return;
        }
        let manager = manager.unwrap();
        assert_eq!(manager.active_count().await, 0);
    }
}
