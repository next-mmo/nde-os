use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;

use super::manifest::{ActorManager, ActorRuntime};
use super::storage::RunStorage;

// ─── Run Status ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunStatus {
    Running,
    Succeeded,
    Failed,
    Aborted,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunStatus::Running => write!(f, "RUNNING"),
            RunStatus::Succeeded => write!(f, "SUCCEEDED"),
            RunStatus::Failed => write!(f, "FAILED"),
            RunStatus::Aborted => write!(f, "ABORTED"),
        }
    }
}

// ─── Actor Run ─────────────────────────────────────────────────────

/// Represents a single execution of an actor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorRun {
    pub run_id: String,
    pub actor_id: String,
    pub status: RunStatus,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub dataset_items: usize,
    pub exit_code: Option<i32>,
}

/// Internal tracking of a running actor process.
struct RunningActor {
    run_id: String,
    actor_id: String,
    process_id: u32,
    started_at: u64,
}

// ─── Actor Runner ──────────────────────────────────────────────────

/// Executes actors locally using Shield Browser as the browser backend.
///
/// Lifecycle:
/// 1. Load and validate manifest
/// 2. Validate input against schema (apply defaults)
/// 3. Create run storage (dataset, KV store, log)
/// 4. Set environment variables (NDE_ACTOR=1, CDP endpoint, input path, etc.)
/// 5. Spawn the actor process (Python via uv / Node via npx)
/// 6. Monitor stdout/stderr → log file
/// 7. On exit: record status, finalize storage
pub struct ActorRunner {
    base_dir: PathBuf,
    running: Arc<Mutex<HashMap<String, RunningActor>>>,
}

impl ActorRunner {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn actors_dir(&self) -> PathBuf {
        self.base_dir.join("actors")
    }

    /// Run an actor with the given input JSON.
    /// Returns the run details including the run_id.
    pub async fn run_actor(
        &self,
        actor_id: &str,
        mut input: serde_json::Value,
    ) -> Result<ActorRun> {
        let actor_mgr = ActorManager::new(&self.base_dir);
        let manifest = actor_mgr.get_actor(actor_id)?;

        // Validate and apply defaults
        manifest.input_schema.apply_defaults(&mut input);
        manifest
            .input_schema
            .validate(&input)
            .with_context(|| format!("Input validation failed for actor '{}'", actor_id))?;

        // Generate run ID
        let run_id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Create run storage
        let storage = RunStorage::create(&self.actors_dir(), actor_id, &run_id)?;
        storage.store_input(&input)?;

        // Write input to a temp file the actor process can read
        let input_path = storage.run_dir.join("input.json");
        let input_json = serde_json::to_string_pretty(&input)?;
        std::fs::write(&input_path, &input_json)?;

        // Build the actor command
        let actor_dir = actor_mgr.actor_dir(actor_id);
        let entry_path = actor_dir.join(manifest.runtime.entry());

        let mut cmd = build_actor_command(&manifest.runtime, &entry_path, &actor_dir)?;

        // Set NDE-OS actor environment variables
        cmd.env("NDE_ACTOR", "1");
        cmd.env("NDE_ACTOR_ID", actor_id);
        cmd.env("NDE_RUN_ID", &run_id);
        cmd.env("NDE_INPUT_PATH", &input_path);
        cmd.env(
            "NDE_DATASET_PATH",
            storage.dataset.path().to_string_lossy().to_string(),
        );
        cmd.env(
            "NDE_KV_STORE_PATH",
            storage.kv_store.dir().to_string_lossy().to_string(),
        );
        cmd.env(
            "NDE_LOG_PATH",
            &storage.log_path.to_string_lossy().to_string(),
        );

        // Browser/Shield config
        if manifest.browser.headless {
            cmd.env("NDE_HEADLESS", "1");
        }
        if let Some(ref profile_id) = manifest.browser.profile_id {
            cmd.env("NDE_SHIELD_PROFILE", profile_id);
        }
        cmd.env("NDE_MAX_PAGES", manifest.browser.pages.to_string());

        // Redirect stdout/stderr to log file
        let log_file =
            std::fs::File::create(&storage.log_path).context("Failed to create actor log file")?;
        let log_err = log_file
            .try_clone()
            .context("Failed to clone log file handle")?;

        cmd.stdout(std::process::Stdio::from(log_file));
        cmd.stderr(std::process::Stdio::from(log_err));

        // Detach on Windows
        #[cfg(windows)]
        {
            use tokio::process::Command as TokioCommand;
            // Creation flags aren't directly exposed on tokio::process::Command without casting to std cmd
            // Let's use the underlying std::process::Command representation if needed,
            // but for portability let's use the wrapper methods if applicable.
            // Tokio Command doesn't have creation_flags natively, we need to import std::os::windows::process::CommandExt
            // and use it on the tokio command builder if it supports it, wait, tokio Command allows standard ext traits.
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        // Spawn the actor process
        let child = cmd.spawn().with_context(|| {
            format!(
                "Failed to spawn actor '{}' ({})",
                actor_id,
                entry_path.display()
            )
        })?;

        let process_id = child.id().context("Failed to get actor process ID")?;

        tracing::info!(
            "Actor '{}' started: run_id={}, PID={}",
            actor_id,
            run_id,
            process_id
        );

        // Track the running process
        {
            let mut running = self.running.lock().await;
            running.insert(
                run_id.clone(),
                RunningActor {
                    run_id: run_id.clone(),
                    actor_id: actor_id.to_string(),
                    process_id,
                    started_at: now,
                },
            );
        }

        // Spawn a background task to wait for completion
        {
            let running = Arc::clone(&self.running);
            let base_dir = self.base_dir.clone();
            let rid = run_id.clone();
            let aid = actor_id.to_string();
            let mut child = child;

            tokio::spawn(async move {
                let exit_status = child.wait().await;
                let finished_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let exit_code = exit_status.as_ref().ok().and_then(|s| s.code());

                let status = match exit_code {
                    Some(0) => RunStatus::Succeeded,
                    _ => RunStatus::Failed,
                };

                tracing::info!(
                    "Actor '{}' run '{}' finished: status={}, exit_code={:?}",
                    aid,
                    rid,
                    status,
                    exit_code
                );

                // Remove from running set
                {
                    let mut lock = running.lock().await;
                    lock.remove(&rid);
                }

                // Write run metadata
                let run_meta = ActorRun {
                    run_id: rid.clone(),
                    actor_id: aid,
                    status,
                    started_at: 0, // Will be set from the RunningActor
                    finished_at: Some(finished_at),
                    dataset_items: 0, // Will be computed on read
                    exit_code,
                };

                let actors_dir = base_dir.join("actors");
                let meta_path = actors_dir
                    .join(&run_meta.actor_id)
                    .join("runs")
                    .join(&rid)
                    .join("run_meta.json");

                if let Ok(json) = serde_json::to_string_pretty(&run_meta) {
                    let _ = std::fs::write(&meta_path, json);
                }
            });
        }

        Ok(ActorRun {
            run_id,
            actor_id: actor_id.to_string(),
            status: RunStatus::Running,
            started_at: now,
            finished_at: None,
            dataset_items: 0,
            exit_code: None,
        })
    }

    /// Stop a running actor by run_id.
    pub async fn stop_actor(&self, run_id: &str) -> Result<()> {
        let actor = {
            let mut running = self.running.lock().await;
            running.remove(run_id)
        };

        match actor {
            Some(a) => {
                tracing::info!("Stopping actor '{}' (PID {})", a.actor_id, a.process_id);
                kill_process(a.process_id);

                // Write aborted status
                let meta_path = self
                    .actors_dir()
                    .join(&a.actor_id)
                    .join("runs")
                    .join(run_id)
                    .join("run_meta.json");

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let run_meta = ActorRun {
                    run_id: run_id.to_string(),
                    actor_id: a.actor_id,
                    status: RunStatus::Aborted,
                    started_at: a.started_at,
                    finished_at: Some(now),
                    dataset_items: 0,
                    exit_code: None,
                };

                if let Ok(json) = serde_json::to_string_pretty(&run_meta) {
                    let _ = std::fs::write(&meta_path, json);
                }

                Ok(())
            }
            None => {
                anyhow::bail!("No running actor with run_id '{}'", run_id);
            }
        }
    }

    /// List currently running actors.
    pub async fn list_running(&self) -> Vec<ActorRun> {
        let running = self.running.lock().await;
        running
            .values()
            .map(|a| ActorRun {
                run_id: a.run_id.clone(),
                actor_id: a.actor_id.clone(),
                status: RunStatus::Running,
                started_at: a.started_at,
                finished_at: None,
                dataset_items: 0,
                exit_code: None,
            })
            .collect()
    }

    /// List all runs for a specific actor (reads from disk).
    pub fn list_runs(&self, actor_id: &str) -> Result<Vec<ActorRun>> {
        let runs_dir = self.actors_dir().join(actor_id).join("runs");
        if !runs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut runs = Vec::new();
        for entry in std::fs::read_dir(&runs_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let meta_path = entry.path().join("run_meta.json");
                if meta_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&meta_path) {
                        if let Ok(mut run) = serde_json::from_str::<ActorRun>(&content) {
                            // Update dataset count
                            let ds_path = entry.path().join("dataset.jsonl");
                            if ds_path.exists() {
                                if let Ok(storage) = super::storage::ActorDataset::new(ds_path) {
                                    run.dataset_items = storage.count().unwrap_or(0);
                                }
                            }
                            runs.push(run);
                        }
                    }
                } else {
                    // Running (no meta yet) — check if in running set
                    let run_id = entry.file_name().to_string_lossy().to_string();
                    // We'll just skip these — they'll appear from list_running()
                    let _ = run_id;
                }
            }
        }

        // Sort by started_at descending
        runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(runs)
    }

    /// Get details for a specific run.
    pub fn get_run(&self, actor_id: &str, run_id: &str) -> Result<ActorRun> {
        let meta_path = self
            .actors_dir()
            .join(actor_id)
            .join("runs")
            .join(run_id)
            .join("run_meta.json");

        if meta_path.exists() {
            let content = std::fs::read_to_string(&meta_path)?;
            let mut run: ActorRun = serde_json::from_str(&content)?;

            // Update dataset count
            let ds_path = self
                .actors_dir()
                .join(actor_id)
                .join("runs")
                .join(run_id)
                .join("dataset.jsonl");
            if let Ok(ds) = super::storage::ActorDataset::new(ds_path) {
                run.dataset_items = ds.count().unwrap_or(0);
            }

            Ok(run)
        } else {
            anyhow::bail!("Run '{}' not found for actor '{}'", run_id, actor_id);
        }
    }

    /// Stop all running actors (cleanup on shutdown).
    pub async fn stop_all(&self) -> Result<()> {
        let actors: Vec<RunningActor> = {
            let mut running = self.running.lock().await;
            running.drain().map(|(_, v)| v).collect()
        };

        for a in actors {
            tracing::info!("Stopping actor '{}' (PID {})", a.actor_id, a.process_id);
            kill_process(a.process_id);
        }

        Ok(())
    }
}

// ─── Command Building ──────────────────────────────────────────────

/// Build the subprocess command for an actor based on its runtime.
fn build_actor_command(
    runtime: &ActorRuntime,
    entry_path: &Path,
    working_dir: &Path,
) -> Result<TokioCommand> {
    let mut cmd = match runtime {
        ActorRuntime::Python { .. } => {
            // Use Python from uv-managed venv or system PATH
            let python = if cfg!(windows) { "python" } else { "python3" };
            let mut c = TokioCommand::new(python);
            c.arg(entry_path);
            c
        }
        ActorRuntime::Node { .. } => {
            let node = "node";
            let mut c = TokioCommand::new(node);
            c.arg(entry_path);
            c
        }
    };

    cmd.current_dir(working_dir);

    Ok(cmd)
}

// ─── Cross-Platform Process Kill ───────────────────────────────────

fn kill_process(pid: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F", "/T"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_status_display() {
        assert_eq!(format!("{}", RunStatus::Running), "RUNNING");
        assert_eq!(format!("{}", RunStatus::Succeeded), "SUCCEEDED");
        assert_eq!(format!("{}", RunStatus::Failed), "FAILED");
        assert_eq!(format!("{}", RunStatus::Aborted), "ABORTED");
    }

    #[test]
    fn test_actor_run_serialization() {
        let run = ActorRun {
            run_id: "abc-123".to_string(),
            actor_id: "my-scraper".to_string(),
            status: RunStatus::Succeeded,
            started_at: 1000,
            finished_at: Some(2000),
            dataset_items: 42,
            exit_code: Some(0),
        };

        let json = serde_json::to_string(&run).unwrap();
        let parsed: ActorRun = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.run_id, "abc-123");
        assert_eq!(parsed.status, RunStatus::Succeeded);
        assert_eq!(parsed.dataset_items, 42);
    }

    #[test]
    fn test_build_python_command() {
        let runtime = ActorRuntime::Python {
            version: "3.11".to_string(),
            pip_deps: vec![],
            entry: "src/main.py".to_string(),
        };
        let entry = PathBuf::from("/actors/test/src/main.py");
        let working = PathBuf::from("/actors/test");

        let cmd = build_actor_command(&runtime, &entry, &working);
        assert!(cmd.is_ok());
    }

    #[test]
    fn test_build_node_command() {
        let runtime = ActorRuntime::Node {
            version: "20".to_string(),
            npm_deps: vec![],
            entry: "src/main.js".to_string(),
        };
        let entry = PathBuf::from("/actors/test/src/main.js");
        let working = PathBuf::from("/actors/test");

        let cmd = build_actor_command(&runtime, &entry, &working);
        assert!(cmd.is_ok());
    }
}
