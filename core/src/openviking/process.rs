//! Manages the OpenViking server as a sidecar subprocess.
//!
//! Auto-installs `openviking` via `uv pip install`, writes config,
//! starts the server, and monitors health. Falls back gracefully
//! if Python/uv is unavailable.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};

use super::client::VikingClient;
use super::config::VikingConfig;
use crate::uv_env;

/// Manages the OpenViking server lifecycle.
pub struct VikingProcess {
    config: VikingConfig,
    data_dir: PathBuf,
    child: Option<Child>,
}

impl VikingProcess {
    /// Create a new process manager (does NOT start the server).
    pub fn new(config: VikingConfig, data_dir: &Path) -> Self {
        Self {
            config,
            data_dir: data_dir.to_path_buf(),
            child: None,
        }
    }

    /// Install OpenViking via uv/pip if not already available.
    pub async fn ensure_installed(&self) -> Result<bool> {
        // Check if openviking-server is already on PATH
        let check = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "where openviking-server"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
        } else {
            Command::new("sh")
                .args(["-c", "which openviking-server"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
        };

        if let Ok(status) = check {
            if status.success() {
                tracing::info!("openviking-server already installed");
                return Ok(true);
            }
        }

        // Resolve uv binary via the existing bootstrapper (bundled → system → download)
        let uv_bin = match uv_env::ensure_uv(&self.data_dir) {
            Ok(path) => {
                tracing::info!("Using uv at: {}", path.display());
                path
            }
            Err(e) => {
                tracing::warn!("Cannot find or download uv: {}", e);
                // Fall through to pip-only fallback below
                return self.pip_fallback_install().await;
            }
        };

        // Install via uv with the resolved binary path
        tracing::info!("Installing OpenViking via uv...");
        let install_result = Command::new(&uv_bin)
            .args(["pip", "install", "--system", "openviking", "--upgrade"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        match install_result {
            Ok(output) if output.status.success() => {
                tracing::info!("OpenViking installed successfully via uv");
                Ok(true)
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("uv install failed ({}): {}", output.status, stderr);
                self.pip_fallback_install().await
            }
            Err(e) => {
                tracing::warn!("Cannot run uv binary: {}", e);
                self.pip_fallback_install().await
            }
        }
    }

    /// Fallback: install via system pip.
    async fn pip_fallback_install(&self) -> Result<bool> {
        tracing::info!("Trying pip fallback...");
        let pip_result = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "pip install openviking --upgrade"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
        } else {
            Command::new("sh")
                .args(["-c", "pip install openviking --upgrade"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
        };

        match pip_result {
            Ok(output) if output.status.success() => {
                tracing::info!("OpenViking installed successfully via pip");
                Ok(true)
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("pip install failed ({}): {}", output.status, stderr);
                Ok(false)
            }
            Err(e) => {
                tracing::warn!("Cannot run pip: {}", e);
                Ok(false)
            }
        }
    }

    /// Write config files and start the OpenViking server process.
    pub async fn start(&mut self) -> Result<()> {
        if self.child.is_some() {
            tracing::info!("OpenViking server already running");
            return Ok(());
        }

        // Write config files
        let conf_dir = self.data_dir.join(".openviking");
        self.config
            .write_server_conf(&conf_dir)
            .context("Failed to write ov.conf")?;
        self.config
            .write_client_conf(&conf_dir)
            .context("Failed to write ovcli.conf")?;

        let conf_path = conf_dir.join("ov.conf");

        // Start the server — pass --config and --port as CLI args
        let conf_str = conf_path.to_string_lossy().to_string();
        let port_str = self.config.port.to_string();

        tracing::info!(
            "OpenViking launch: python -m openviking_cli.server_bootstrap --config {} --port {} --host 127.0.0.1",
            conf_str, port_str
        );

        // First, do a quick test run to check the module is importable.
        // Call python directly — avoids cmd /C quote-escaping issues on Windows.
        let test_output = Command::new("python")
            .args(["-c", "import openviking_cli.server_bootstrap; print('ok')"])
            .output()
            .await;

        match &test_output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                tracing::info!("OpenViking import test: status={}, stdout={}, stderr={}", out.status, stdout.trim(), stderr.trim());
                if !out.status.success() {
                    return Err(anyhow::anyhow!(
                        "OpenViking module not importable: {}", stderr.trim()
                    ));
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Cannot run python: {}", e));
            }
        }

        // Spawn the server process directly — no shell wrapper needed.
        let child = Command::new("python")
            .args([
                "-m", "openviking_cli.server_bootstrap",
                "--config", &conf_str,
                "--port", &port_str,
                "--host", "127.0.0.1",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to spawn openviking server process")?;

        self.child = Some(child);
        tracing::info!("OpenViking server spawned on port {} (waiting for health...)", self.config.port);

        // Wait for health check (up to 30 seconds)
        let client = VikingClient::new(&self.config.base_url());
        for i in 0..60 {
            // Check if process exited prematurely
            let mut crashed = false;
            if let Some(child) = &mut self.child {
                if let Ok(Some(_)) = child.try_wait() {
                    crashed = true;
                }
            }
            if crashed {
                let mut stderr_msg = String::new();
                if let Some(mut dead_child) = self.child.take() {
                    if let Some(mut stderr) = dead_child.stderr.take() {
                        use tokio::io::AsyncReadExt;
                        let _ = stderr.read_to_string(&mut stderr_msg).await;
                    }
                    let status = dead_child.try_wait().ok().flatten();
                    let code = status.map(|s| s.to_string()).unwrap_or_else(|| "unknown".into());
                    tracing::error!("OpenViking crashed (status {}): {}", code, stderr_msg.trim());
                    return Err(anyhow::anyhow!(
                        "OpenViking server process exited with {}: {}",
                        code, stderr_msg.trim()
                    ));
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if client.health().await.unwrap_or(false) {
                tracing::info!("OpenViking server healthy after {}ms", (i + 1) * 500);
                return Ok(());
            }
        }

        tracing::warn!("OpenViking server started but health check timed out after 30s");
        Ok(())
    }

    /// Stop the server gracefully.
    pub async fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            tracing::info!("Stopping OpenViking server...");
            let _ = child.kill().await;
        }
    }

    /// Check if the managed process is still running.
    pub fn is_running(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(None) => true, // Still running
                _ => {
                    self.child = None;
                    false
                }
            }
        } else {
            false
        }
    }

    /// Get the config reference.
    pub fn config(&self) -> &VikingConfig {
        &self.config
    }

    /// Check synchronously if `openviking-server` is available on the system.
    /// Used by the service registry for detection without an async runtime.
    pub fn is_installed_sync(&self) -> bool {
        let check = if cfg!(windows) {
            std::process::Command::new("cmd")
                .args(["/C", "where openviking-server"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
        } else {
            std::process::Command::new("sh")
                .args(["-c", "which openviking-server"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
        };
        check.map(|s| s.success()).unwrap_or(false)
    }
}

impl Drop for VikingProcess {
    fn drop(&mut self) {
        // Best-effort synchronous kill
        if let Some(child) = &mut self.child {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_creation() {
        let dir = tempfile::tempdir().unwrap();
        let config = VikingConfig::default();
        let process = VikingProcess::new(config, dir.path());
        assert!(!process.config().base_url().is_empty());
    }

    #[test]
    fn test_is_running_without_start() {
        let dir = tempfile::tempdir().unwrap();
        let config = VikingConfig::default();
        let mut process = VikingProcess::new(config, dir.path());
        assert!(!process.is_running());
    }
}
