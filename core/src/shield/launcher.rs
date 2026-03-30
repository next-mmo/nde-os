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
    ///
    /// An optional `on_exit` callback is invoked when the browser process exits
    /// on its own (e.g. user closes the window). This lets the Tauri layer emit
    /// an event so the frontend can refresh.
    pub async fn launch_profile<F>(
        &self,
        profile_id: &str,
        url: Option<&str>,
        on_exit: Option<F>,
    ) -> Result<u16>
    where
        F: FnOnce(String) + Send + 'static,
    {
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
        cmd.env(
            "DISPLAY",
            std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into()),
        );

        // For Wayfern/Chromium, suppress first-run dialogs
        if profile.engine == BrowserEngine::Wayfern {
            cmd.env("CHROME_NO_SANDBOX", "1");
        }

        // Pipe stdout/stderr to null to avoid console noise
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        let mut child = cmd.spawn().with_context(|| {
            format!("Failed to spawn {} process", profile.engine.display_name())
        })?;

        let process_id = child
            .id()
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
            instances.insert(
                profile_id.to_string(),
                BrowserInstance {
                    profile_id: profile_id.to_string(),
                    process_id,
                    cdp_port,
                    engine: profile.engine.clone(),
                },
            );
        }

        // Update profile metadata
        profile_mgr.set_running(profile_id, process_id)?;

        // Spawn a background task that waits for the browser process to exit.
        // When camoufox is closed by the user, this task cleans up the instance
        // map + profile metadata and fires the on_exit callback.
        {
            let instances = Arc::clone(&self.instances);
            let base_dir = self.base_dir.clone();
            let pid = profile_id.to_string();
            tokio::spawn(async move {
                let exit_status = child.wait().await;
                tracing::info!(
                    "Browser process for profile '{}' exited: {:?}",
                    pid,
                    exit_status
                );

                // Remove from running instances (returns Some if this was
                // a natural exit, None if stop_profile already cleaned up)
                let was_natural_exit = {
                    let mut lock = instances.lock().await;
                    lock.remove(&pid).is_some()
                };

                // Only clean up + notify if this was a natural exit
                // (user closed camoufox). If stop_profile handled it,
                // the instance was already removed above.
                if was_natural_exit {
                    let pmgr = ProfileManager::new(&base_dir);
                    let _ = pmgr.set_stopped(&pid);

                    if let Some(cb) = on_exit {
                        cb(pid);
                    }
                }
            });
        }

        Ok(cdp_port)
    }

    /// Stop a running browser profile.
    ///
    /// If the browser already exited (e.g. user closed camoufox), this is a
    /// no-op: the background watcher already cleaned up the instance map.
    pub async fn stop_profile(&self, profile_id: &str) -> Result<()> {
        let instance = {
            let mut instances = self.instances.lock().await;
            instances.remove(profile_id)
        };

        if let Some(inst) = instance {
            tracing::info!(
                "Stopping {} (PID {}) for profile '{}'",
                inst.engine.display_name(),
                inst.process_id,
                profile_id
            );
            kill_process(inst.process_id);
        } else {
            tracing::info!(
                "Profile '{}' already exited, ensuring metadata is cleaned up",
                profile_id
            );
        }

        // Always ensure profile metadata reflects stopped state
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
            tracing::info!(
                "Stopping {} (PID {})",
                instance.engine.display_name(),
                instance.process_id
            );
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

/// Resolve "latest" to an actual version tag from GitHub releases.
pub async fn resolve_latest_version(engine: &BrowserEngine) -> Result<String> {
    match engine {
        BrowserEngine::Camoufox => {
            let client = reqwest::Client::builder()
                .user_agent("NDE-OS-Shield/1.0")
                .build()
                .context("Failed to build HTTP client")?;

            let resp = client
                .get("https://api.github.com/repos/daijro/camoufox/releases/latest")
                .send()
                .await
                .context("Failed to fetch latest Camoufox release from GitHub")?;

            if !resp.status().is_success() {
                anyhow::bail!("GitHub API returned status {}", resp.status());
            }

            let json: serde_json::Value = resp
                .json()
                .await
                .context("Failed to parse GitHub release JSON")?;

            let tag = json["tag_name"]
                .as_str()
                .context("Missing tag_name in GitHub release")?;

            // Strip leading 'v' if present: "v135.0.1-beta.24" -> "135.0.1-beta.24"
            let version = tag.strip_prefix('v').unwrap_or(tag);
            Ok(version.to_string())
        }
        BrowserEngine::Wayfern => {
            anyhow::bail!("Wayfern engine is not yet available for download. Coming soon.")
        }
    }
}

/// Get the download URL for an engine version on the current platform.
pub fn get_download_url(engine: &BrowserEngine, version: &str) -> Result<String> {
    let (os, arch) = get_platform_info();

    match engine {
        BrowserEngine::Camoufox => {
            // Real Camoufox release asset pattern (verified against GitHub):
            // camoufox-{version}-{os}.{arch}.zip
            // Example: camoufox-135.0.1-beta.24-win.x86_64.zip
            let os_name = match os.as_str() {
                "windows" => "win",
                "linux" => "lin",
                "macos" => "mac",
                _ => anyhow::bail!("Unsupported OS for Camoufox: {os}"),
            };

            let arch_name = match arch.as_str() {
                "x64" => "x86_64",
                "arm64" => "arm64",
                _ => anyhow::bail!("Unsupported architecture for Camoufox: {arch}"),
            };

            Ok(format!(
                "https://github.com/daijro/camoufox/releases/download/v{version}/camoufox-{version}-{os_name}.{arch_name}.zip"
            ))
        }
        BrowserEngine::Wayfern => {
            anyhow::bail!("Wayfern engine is not yet available for download. Coming soon.")
        }
    }
}

/// Returns metadata about available engines for the UI.
pub fn get_available_engines() -> Vec<AvailableEngine> {
    vec![
        AvailableEngine {
            engine: "camoufox".into(),
            name: "Camoufox".into(),
            description: "Firefox-based with native C++ fingerprint spoofing. Supports canvas, WebGL, audio, timezone, locale, WebRTC, and geolocation.".into(),
            available: true,
            icon: "🦊".into(),
        },
        AvailableEngine {
            engine: "wayfern".into(),
            name: "Wayfern".into(),
            description: "Chromium-based with patched canvas, WebGL, and navigator APIs at the C++ level. Coming soon.".into(),
            available: false,
            icon: "🌐".into(),
        },
    ]
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AvailableEngine {
    pub engine: String,
    pub name: String,
    pub description: String,
    pub available: bool,
    pub icon: String,
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
        let url = get_download_url(&BrowserEngine::Camoufox, "135.0.1-beta.24").unwrap();
        assert!(url.contains("camoufox"));
        assert!(url.contains("135.0.1-beta.24"));
        assert!(url.contains("github.com/daijro/camoufox"));
        assert!(url.ends_with(".zip"));
    }

    #[test]
    fn test_wayfern_download_not_available() {
        let result = get_download_url(&BrowserEngine::Wayfern, "133.0");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Coming soon"));
    }
}
