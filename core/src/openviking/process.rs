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

        // Try to install via uv (fast) first, then pip
        tracing::info!("Installing OpenViking via uv...");
        let install_cmd = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "uv pip install openviking --upgrade"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .status()
                .await
        } else {
            Command::new("sh")
                .args(["-c", "uv pip install openviking --upgrade"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .status()
                .await
        };

        match install_cmd {
            Ok(status) if status.success() => {
                tracing::info!("OpenViking installed successfully via uv");
                Ok(true)
            }
            _ => {
                // Fallback to pip
                tracing::warn!("uv install failed, trying pip...");
                let pip_cmd = if cfg!(windows) {
                    Command::new("cmd")
                        .args(["/C", "pip install openviking --upgrade"])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .status()
                        .await
                } else {
                    Command::new("sh")
                        .args(["-c", "pip install openviking --upgrade"])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .status()
                        .await
                };

                match pip_cmd {
                    Ok(status) if status.success() => {
                        tracing::info!("OpenViking installed successfully via pip");
                        Ok(true)
                    }
                    Ok(status) => {
                        tracing::warn!("pip install exited with {}", status);
                        Ok(false)
                    }
                    Err(e) => {
                        tracing::warn!("Cannot install OpenViking: {}", e);
                        Ok(false)
                    }
                }
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

        // Start the server
        let child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "openviking-server"])
                .env("OPENVIKING_CONFIG_FILE", &conf_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .context("Failed to start openviking-server")?
        } else {
            Command::new("sh")
                .args(["-c", "openviking-server"])
                .env("OPENVIKING_CONFIG_FILE", &conf_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .context("Failed to start openviking-server")?
        };

        self.child = Some(child);
        tracing::info!("OpenViking server started on port {}", self.config.port);

        // Wait for health check (up to 15 seconds)
        let client = VikingClient::new(&self.config.base_url());
        for i in 0..30 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if client.health().await.unwrap_or(false) {
                tracing::info!("OpenViking server healthy after {}ms", (i + 1) * 500);
                return Ok(());
            }
        }

        tracing::warn!("OpenViking server started but health check timed out after 15s");
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
