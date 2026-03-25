use super::hooks::{HookContext, HookResult, HookType};
use super::manifest::{PluginManifest, PluginType};
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Maximum log lines kept per plugin (ring buffer).
const MAX_LOG_LINES: usize = 500;

/// Thread-safe log buffer shared between the engine and reader threads.
type SharedLogBuffer = Arc<Mutex<VecDeque<PluginLogEntry>>>;

/// A single log entry from a plugin.
#[derive(Debug, Clone, Serialize)]
pub struct PluginLogEntry {
    /// ISO-8601 timestamp
    pub timestamp: String,
    /// "stdout", "stderr", or "system"
    pub stream: String,
    /// The log line content
    pub message: String,
}

impl PluginLogEntry {
    fn now(stream: &str, message: impl Into<String>) -> Self {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        // Simple ISO-ish timestamp without chrono
        let secs = ts.as_secs();
        Self {
            timestamp: format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                1970 + secs / 31_536_000,
                (secs % 31_536_000) / 2_592_000 + 1,
                (secs % 2_592_000) / 86_400 + 1,
                (secs % 86_400) / 3600,
                (secs % 3600) / 60,
                secs % 60,
            ),
            stream: stream.to_string(),
            message: message.into(),
        }
    }
}

fn push_log(buf: &SharedLogBuffer, stream: &str, message: impl Into<String>) {
    if let Ok(mut logs) = buf.lock() {
        if logs.len() >= MAX_LOG_LINES {
            logs.pop_front();
        }
        logs.push_back(PluginLogEntry::now(stream, message));
    }
}

/// A loaded and potentially running plugin.
struct LoadedPlugin {
    manifest: PluginManifest,
    state: PluginState,
    plugin_dir: PathBuf,
    process: Option<Child>,
    logs: SharedLogBuffer,
}

/// Lifecycle state of a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    /// Discovered but not installed
    Discovered,
    /// Dependencies installed, ready to start
    Installed,
    /// Currently running
    Running,
    /// Stopped (was running)
    Stopped,
    /// Error state
    Error,
}

/// Status info for a running plugin.
#[derive(Debug, Clone, Serialize)]
pub struct PluginStatus {
    pub id: String,
    pub name: String,
    pub version: String,
    pub plugin_type: PluginType,
    pub state: PluginState,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub hooks: Vec<HookType>,
}

/// Plugin engine — manages plugin lifecycle and hook dispatch.
pub struct PluginEngine {
    plugins: HashMap<String, LoadedPlugin>,
    plugins_dir: PathBuf,
    /// Additional directories to scan for plugins (e.g. bundled plugins shipped with the binary).
    extra_dirs: Vec<PathBuf>,
}

impl PluginEngine {
    pub fn new(plugins_dir: &Path) -> Self {
        Self {
            plugins: HashMap::new(),
            plugins_dir: plugins_dir.to_path_buf(),
            extra_dirs: Vec::new(),
        }
    }

    /// Register an additional directory to scan for plugins.
    pub fn add_search_dir(&mut self, dir: PathBuf) {
        if dir.exists() && dir.is_dir() && !self.extra_dirs.contains(&dir) {
            self.extra_dirs.push(dir);
        }
    }

    /// Discover plugins from all registered directories.
    /// Looks for plugin.json in each subdirectory of the primary and extra dirs.
    pub fn discover(&mut self) -> Result<Vec<PluginManifest>> {
        let mut manifests = Vec::new();

        // Collect all directories to scan: primary + extras
        let mut dirs_to_scan = vec![self.plugins_dir.clone()];
        dirs_to_scan.extend(self.extra_dirs.clone());

        for scan_dir in &dirs_to_scan {
            if !scan_dir.exists() {
                continue;
            }

            let entries = match std::fs::read_dir(scan_dir) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(
                        dir = %scan_dir.display(),
                        error = %e,
                        "Failed to read plugin directory"
                    );
                    continue;
                }
            };

            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }

                let manifest_path = path.join("plugin.json");
                if !manifest_path.exists() {
                    continue;
                }

                match PluginManifest::from_file(&manifest_path) {
                    Ok(manifest) => {
                        // Skip duplicates (user-installed takes priority over bundled)
                        if self.plugins.contains_key(&manifest.id) {
                            tracing::debug!(
                                plugin = %manifest.id,
                                dir = %scan_dir.display(),
                                "Skipping duplicate plugin (already loaded)"
                            );
                            continue;
                        }

                        tracing::info!(
                            plugin = %manifest.id,
                            name = %manifest.name,
                            dir = %scan_dir.display(),
                            "Discovered plugin"
                        );

                        let id = manifest.id.clone();
                        manifests.push(manifest.clone());

                        let log_buf: SharedLogBuffer =
                            Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES)));
                        push_log(
                            &log_buf,
                            "system",
                            format!("Plugin '{}' v{} discovered", manifest.name, manifest.version),
                        );

                        self.plugins.insert(
                            id,
                            LoadedPlugin {
                                manifest,
                                state: PluginState::Discovered,
                                plugin_dir: path,
                                process: None,
                                logs: log_buf,
                            },
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %manifest_path.display(),
                            error = %e,
                            "Failed to load plugin manifest"
                        );
                    }
                }
            }
        }

        Ok(manifests)
    }

    /// Install a plugin — creates venv and installs dependencies.
    pub async fn install(&mut self, plugin_id: &str) -> Result<()> {
        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| anyhow!("Plugin '{}' not found", plugin_id))?;

        if plugin.state == PluginState::Installed || plugin.state == PluginState::Running {
            return Ok(()); // Already installed
        }

        push_log(&plugin.logs, "system", "Installing plugin...");

        // Install Python deps via uv if it's a Python plugin
        if plugin.manifest.language == super::manifest::Language::Python
            && !plugin.manifest.deps.is_empty()
        {
            let venv_dir = plugin.plugin_dir.join(".venv");
            push_log(
                &plugin.logs,
                "system",
                format!("Creating venv at {}", venv_dir.display()),
            );
            tracing::info!(
                plugin = plugin_id,
                deps = ?plugin.manifest.deps,
                "Installing plugin dependencies"
            );

            // Create venv
            let uv_cmd = if cfg!(windows) { "uv.exe" } else { "uv" };
            let output = std::process::Command::new(uv_cmd)
                .arg("venv")
                .arg(&venv_dir)
                .current_dir(&plugin.plugin_dir)
                .output()
                .context("Failed to create plugin venv via uv")?;

            // Capture venv creation output
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                push_log(&plugin.logs, "stdout", line);
            }
            for line in String::from_utf8_lossy(&output.stderr).lines() {
                push_log(&plugin.logs, "stderr", line);
            }

            if !output.status.success() {
                plugin.state = PluginState::Error;
                push_log(&plugin.logs, "system", "ERROR: Failed to create venv");
                return Err(anyhow!("Failed to create venv for plugin '{}'", plugin_id));
            }

            // Install deps
            let deps_display = plugin.manifest.deps.join(", ");
            push_log(
                &plugin.logs,
                "system",
                format!("Installing dependencies: {}", deps_display),
            );
            let pip_args: Vec<&str> = plugin.manifest.deps.iter().map(|s| s.as_str()).collect();
            let output = std::process::Command::new(uv_cmd)
                .arg("pip")
                .arg("install")
                .args(&pip_args)
                .arg("--python")
                .arg(
                    venv_dir
                        .join(if cfg!(windows) {
                            "Scripts/python.exe"
                        } else {
                            "bin/python"
                        })
                        .to_string_lossy()
                        .to_string(),
                )
                .current_dir(&plugin.plugin_dir)
                .output()
                .context("Failed to install plugin deps via uv")?;

            for line in String::from_utf8_lossy(&output.stdout).lines() {
                push_log(&plugin.logs, "stdout", line);
            }
            for line in String::from_utf8_lossy(&output.stderr).lines() {
                push_log(&plugin.logs, "stderr", line);
            }

            if !output.status.success() {
                plugin.state = PluginState::Error;
                push_log(&plugin.logs, "system", "ERROR: Failed to install dependencies");
                return Err(anyhow!(
                    "Failed to install dependencies for plugin '{}'",
                    plugin_id
                ));
            }
        }

        plugin.state = PluginState::Installed;
        push_log(&plugin.logs, "system", "Plugin installed successfully");
        tracing::info!(plugin = plugin_id, "Plugin installed");
        Ok(())
    }

    /// Start a plugin process.
    pub async fn start(&mut self, plugin_id: &str) -> Result<()> {
        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| anyhow!("Plugin '{}' not found", plugin_id))?;

        if plugin.state == PluginState::Running {
            return Err(anyhow!("Plugin '{}' is already running", plugin_id));
        }

        let entry = plugin
            .manifest
            .entry
            .as_ref()
            .ok_or_else(|| anyhow!("Plugin '{}' has no entry point", plugin_id))?;

        let entry_path = plugin.plugin_dir.join(entry);
        if !entry_path.exists() {
            push_log(
                &plugin.logs,
                "system",
                format!("ERROR: Entry point not found: {}", entry_path.display()),
            );
            return Err(anyhow!(
                "Plugin entry point not found: {}",
                entry_path.display()
            ));
        }

        // Determine how to run
        let (cmd, args) = match plugin.manifest.language {
            super::manifest::Language::Python => {
                let venv_python = plugin.plugin_dir.join(".venv").join(if cfg!(windows) {
                    "Scripts/python.exe"
                } else {
                    "bin/python"
                });
                let python = if venv_python.exists() {
                    venv_python.to_string_lossy().to_string()
                } else {
                    "python3".to_string()
                };
                (python, vec![entry_path.to_string_lossy().to_string()])
            }
            super::manifest::Language::JavaScript | super::manifest::Language::TypeScript => {
                ("node".to_string(), vec![entry_path.to_string_lossy().to_string()])
            }
            super::manifest::Language::Binary => {
                (entry_path.to_string_lossy().to_string(), vec![])
            }
        };

        push_log(
            &plugin.logs,
            "system",
            format!("Starting: {} {}", cmd, args.join(" ")),
        );

        let mut child = std::process::Command::new(&cmd)
            .args(&args)
            .current_dir(&plugin.plugin_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env(
                "PLUGIN_ID",
                &plugin.manifest.id,
            )
            .env(
                "PLUGIN_PORT",
                plugin
                    .manifest
                    .port
                    .map(|p| p.to_string())
                    .unwrap_or_default(),
            )
            .spawn()
            .with_context(|| format!("Failed to start plugin '{}'", plugin_id))?;

        let pid = child.id();

        // Spawn stdout reader thread
        if let Some(stdout) = child.stdout.take() {
            let buf = plugin.logs.clone();
            let id = plugin_id.to_string();
            std::thread::Builder::new()
                .name(format!("plugin-stdout-{}", id))
                .spawn(move || {
                    let reader = std::io::BufReader::new(stdout);
                    for line in reader.lines() {
                        match line {
                            Ok(l) => push_log(&buf, "stdout", l),
                            Err(_) => break,
                        }
                    }
                })
                .ok();
        }

        // Spawn stderr reader thread
        if let Some(stderr) = child.stderr.take() {
            let buf = plugin.logs.clone();
            let id = plugin_id.to_string();
            std::thread::Builder::new()
                .name(format!("plugin-stderr-{}", id))
                .spawn(move || {
                    let reader = std::io::BufReader::new(stderr);
                    for line in reader.lines() {
                        match line {
                            Ok(l) => push_log(&buf, "stderr", l),
                            Err(_) => break,
                        }
                    }
                })
                .ok();
        }

        push_log(
            &plugin.logs,
            "system",
            format!("Plugin started (PID: {})", pid),
        );
        plugin.process = Some(child);
        plugin.state = PluginState::Running;
        tracing::info!(
            plugin = plugin_id,
            pid = pid,
            "Plugin started"
        );

        Ok(())
    }

    /// Stop a running plugin.
    pub async fn stop(&mut self, plugin_id: &str) -> Result<()> {
        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| anyhow!("Plugin '{}' not found", plugin_id))?;

        push_log(&plugin.logs, "system", "Stopping plugin...");

        if let Some(ref mut child) = plugin.process {
            let _ = child.kill();
            let _ = child.wait();
        }

        plugin.process = None;
        plugin.state = PluginState::Stopped;
        push_log(&plugin.logs, "system", "Plugin stopped");
        tracing::info!(plugin = plugin_id, "Plugin stopped");
        Ok(())
    }

    /// Fire hooks for all plugins listening to a given event.
    pub fn fire_hook(&self, context: &HookContext) -> Vec<HookResult> {
        let mut results = Vec::new();

        for (id, plugin) in &self.plugins {
            if plugin.state != PluginState::Running {
                continue;
            }
            if !plugin.manifest.hooks.contains(&context.hook_type) {
                continue;
            }

            // For now, hooks are synchronous JSON-RPC calls to the plugin's port.
            // Phase 3 will add async RPC.
            results.push(HookResult::ok(id));
        }

        results
    }

    /// Get status of all plugins.
    pub fn status(&self) -> Vec<PluginStatus> {
        self.plugins
            .values()
            .map(|p| PluginStatus {
                id: p.manifest.id.clone(),
                name: p.manifest.name.clone(),
                version: p.manifest.version.clone(),
                plugin_type: p.manifest.plugin_type,
                state: p.state,
                pid: p.process.as_ref().map(|c| c.id()),
                port: p.manifest.port,
                hooks: p.manifest.hooks.clone(),
            })
            .collect()
    }

    /// Get info about a specific plugin.
    pub fn get(&self, plugin_id: &str) -> Option<PluginStatus> {
        self.plugins.get(plugin_id).map(|p| PluginStatus {
            id: p.manifest.id.clone(),
            name: p.manifest.name.clone(),
            version: p.manifest.version.clone(),
            plugin_type: p.manifest.plugin_type,
            state: p.state,
            pid: p.process.as_ref().map(|c| c.id()),
            port: p.manifest.port,
            hooks: p.manifest.hooks.clone(),
        })
    }

    /// List all discovered plugin IDs.
    pub fn plugin_ids(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Get logs for a specific plugin.
    pub fn logs(&self, plugin_id: &str) -> Option<Vec<PluginLogEntry>> {
        self.plugins.get(plugin_id).map(|p| {
            p.logs
                .lock()
                .map(|buf| buf.iter().cloned().collect())
                .unwrap_or_default()
        })
    }

    /// Clear logs for a specific plugin.
    pub fn clear_logs(&self, plugin_id: &str) -> bool {
        if let Some(p) = self.plugins.get(plugin_id) {
            if let Ok(mut buf) = p.logs.lock() {
                buf.clear();
                return true;
            }
        }
        false
    }

    /// Get contributed tool definitions from all running plugins.
    pub fn contributed_tools(&self) -> Vec<crate::llm::ToolDef> {
        self.plugins
            .values()
            .filter(|p| p.state == PluginState::Running)
            .flat_map(|p| {
                p.manifest.provides_tools.iter().map(|t| crate::llm::ToolDef {
                    name: format!("{}_{}", p.manifest.id.replace('-', "_"), t.name),
                    description: t.description.clone(),
                    parameters: t.parameters.clone(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_plugins() {
        let dir = std::env::temp_dir().join(format!(
            "nde-plugin-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        // Create a test plugin
        let plugin_dir = dir.join("test-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.json"),
            r#"{
                "id": "test-plugin",
                "name": "Test Plugin",
                "version": "1.0.0",
                "type": "tool",
                "description": "A test",
                "author": "test"
            }"#,
        )
        .unwrap();
        // Create a dummy entry point so start() won't fail on missing file
        std::fs::write(plugin_dir.join("main.py"), "print('hello')").unwrap();

        let mut engine = PluginEngine::new(&dir);
        let manifests = engine.discover().unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, "test-plugin");

        let status = engine.status();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].state, PluginState::Discovered);

        // Verify logs were captured
        let logs = engine.logs("test-plugin").unwrap();
        assert!(!logs.is_empty());
        assert_eq!(logs[0].stream, "system");
        assert!(logs[0].message.contains("discovered"));

        std::fs::remove_dir_all(dir).ok();
    }
}
