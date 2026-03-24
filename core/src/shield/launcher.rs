use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;

use super::browser::{self, BrowserEngine, ProxyConfig};
use super::engine::EngineManager;
use super::profile::{ProfileManager, ShieldProfile};

// ─── Running Browser Instance ──────────────────────────────────────

#[derive(Debug)]
pub struct BrowserInstance {
    pub profile_id: String,
    pub process_id: u32,
    pub cdp_port: u16,
    pub engine: BrowserEngine,
}

// ─── Browser Launcher ──────────────────────────────────────────────

/// Manages launching and stopping browser instances.
pub struct BrowserLauncher {
    base_dir: PathBuf,
    instances: Arc<Mutex<HashMap<String, BrowserInstance>>>,
}

impl BrowserLauncher {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
            instances: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Find a free TCP port for CDP debugging.
    async fn find_free_port() -> Result<u16> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        drop(listener);
        Ok(port)
    }

    /// Launch a browser profile. Returns the CDP port.
    pub async fn launch_profile(
        &self,
        profile_id: &str,
        url: Option<&str>,
    ) -> Result<u16> {
        let profile_mgr = ProfileManager::new(&self.base_dir);
        let engine_mgr = EngineManager::new(&self.base_dir);
        let profile = profile_mgr.get_profile(profile_id)?;

        // Check if already running
        {
            let instances = self.instances.lock().await;
            if instances.contains_key(profile_id) {
                anyhow::bail!("Profile '{}' is already running", profile.name);
            }
        }

        // Find the engine executable
        let executable = engine_mgr.get_executable(&profile.engine, &profile.engine_version)?;

        // Find a free CDP port
        let cdp_port = Self::find_free_port().await?;

        // Build profile data directory
        let profile_data_dir = profile.data_dir(profile_mgr.profiles_dir());
        std::fs::create_dir_all(&profile_data_dir)
            .context("Failed to create profile data directory")?;

        // Build launch args
        let args = browser::build_launch_args(
            &profile.engine,
            &profile_data_dir,
            profile.proxy.as_ref(),
            url,
            Some(cdp_port),
            false,
        );

        tracing::info!(
            "Launching {} for profile '{}' on CDP port {}",
            profile.engine.display_name(),
            profile.name,
            cdp_port
        );

        // Build command
        let mut cmd = TokioCommand::new(&executable);
        cmd.args(&args);

        // Set environment for sandboxing
        cmd.env("DISPLAY", std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into()));

        // For Wayfern/Chromium, suppress first-run dialogs
        if profile.engine == BrowserEngine::Wayfern {
            cmd.env("CHROME_NO_SANDBOX", "1");
        }

        // Pipe stdout/stderr to null to avoid console noise
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        let child = cmd.spawn()
            .with_context(|| format!("Failed to spawn {} process", profile.engine.display_name()))?;

        let process_id = child.id()
            .context("Failed to get process ID of spawned browser")?;

        tracing::info!(
            "{} launched with PID {} for profile '{}'",
            profile.engine.display_name(),
            process_id,
            profile.name
        );

        // Record the running instance
        {
            let mut instances = self.instances.lock().await;
            instances.insert(profile_id.to_string(), BrowserInstance {
                profile_id: profile_id.to_string(),
                process_id,
                cdp_port,
                engine: profile.engine.clone(),
            });
        }

        // Update profile metadata
        profile_mgr.set_running(profile_id, process_id)?;

        Ok(cdp_port)
    }

    /// Stop a running browser profile.
    pub async fn stop_profile(&self, profile_id: &str) -> Result<()> {
        let instance = {
            let mut instances = self.instances.lock().await;
            instances.remove(profile_id)
                .context(format!("Profile '{profile_id}' is not running"))?
        };

        tracing::info!(
            "Stopping {} (PID {}) for profile '{}'",
            instance.engine.display_name(),
            instance.process_id,
            profile_id
        );

        kill_process(instance.process_id);

        // Update profile metadata
        let profile_mgr = ProfileManager::new(&self.base_dir);
        profile_mgr.set_stopped(profile_id)?;

        Ok(())
    }

    /// Get list of currently running profile IDs.
    pub async fn running_profiles(&self) -> Vec<String> {
        let instances = self.instances.lock().await;
        instances.keys().cloned().collect()
    }

    /// Check if a specific profile is running.
    pub async fn is_running(&self, profile_id: &str) -> bool {
        let instances = self.instances.lock().await;
        instances.contains_key(profile_id)
    }

    /// Get CDP port for a running profile.
    pub async fn get_cdp_port(&self, profile_id: &str) -> Option<u16> {
        let instances = self.instances.lock().await;
        instances.get(profile_id).map(|i| i.cdp_port)
    }

    /// Stop all running browser instances (for cleanup on app exit).
    pub async fn stop_all(&self) -> Result<()> {
        let instances: Vec<(String, BrowserInstance)> = {
            let mut lock = self.instances.lock().await;
            lock.drain().collect()
        };

        let profile_mgr = ProfileManager::new(&self.base_dir);

        for (id, instance) in instances {
            tracing::info!("Stopping {} (PID {})", instance.engine.display_name(), instance.process_id);
            kill_process(instance.process_id);
            let _ = profile_mgr.set_stopped(&id);
        }

        Ok(())
    }
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

// ─── Download URL Helpers ──────────────────────────────────────────

/// Get the download URL for an engine version on the current platform.
pub fn get_download_url(engine: &BrowserEngine, version: &str) -> Result<String> {
    let (os, arch) = get_platform_info();

    match engine {
        BrowserEngine::Camoufox => {
            let (os_name, arch_name) = match (os.as_str(), arch.as_str()) {
                ("windows", "x64") => ("win", "x86_64"),
                ("windows", "arm64") => ("win", "arm64"),
                ("linux", "x64") => ("lin", "x86_64"),
                ("linux", "arm64") => ("lin", "arm64"),
                ("macos", "x64") => ("mac", "x86_64"),
                ("macos", "arm64") => ("mac", "arm64"),
                _ => anyhow::bail!("Unsupported platform for Camoufox: {os}/{arch}"),
            };

            // Camoufox GitHub releases: https://github.com/daijro/camoufox/releases
            Ok(format!(
                "https://github.com/daijro/camoufox/releases/download/v{version}/camoufox-{version}-{os_name}.{arch_name}.zip"
            ))
        }
        BrowserEngine::Wayfern => {
            let platform_key = format!("{os}-{arch}");
            let ext = match os.as_str() {
                "windows" => "zip",
                "macos" => "dmg",
                "linux" => "tar.xz",
                _ => "zip",
            };

            // Wayfern downloads from download.wayfern.com
            Ok(format!(
                "https://download.wayfern.com/wayfern-{version}-{platform_key}.{ext}"
            ))
        }
    }
}

fn get_platform_info() -> (String, String) {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "unknown"
    };

    (os.to_string(), arch.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camoufox_download_url() {
        let url = get_download_url(&BrowserEngine::Camoufox, "132.0.2").unwrap();
        assert!(url.contains("camoufox"));
        assert!(url.contains("132.0.2"));
        assert!(url.contains("github.com/daijro/camoufox"));
    }

    #[test]
    fn test_wayfern_download_url() {
        let url = get_download_url(&BrowserEngine::Wayfern, "133.0.6943.2").unwrap();
        assert!(url.contains("wayfern"));
        assert!(url.contains("133.0.6943.2"));
        assert!(url.contains("download.wayfern.com"));
    }
}
