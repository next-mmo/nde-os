use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

// ─── Data types ────────────────────────────────────────────────────

/// Information about an available Android Virtual Device.
#[derive(Debug, Clone, Serialize)]
pub struct AvdInfo {
    pub name: String,
}

/// Status of a connected ADB device.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum DeviceStatus {
    Online,
    Offline,
    Booting,
    Unauthorized,
}

impl std::fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceStatus::Online => write!(f, "online"),
            DeviceStatus::Offline => write!(f, "offline"),
            DeviceStatus::Booting => write!(f, "booting"),
            DeviceStatus::Unauthorized => write!(f, "unauthorized"),
        }
    }
}

/// A connected ADB device (emulator or physical).
#[derive(Debug, Clone, Serialize)]
pub struct AdbDevice {
    pub serial: String,
    pub status: DeviceStatus,
    /// The AVD name, if this is an emulator and we can determine it.
    pub avd_name: Option<String>,
}

impl AdbDevice {
    /// Check if this device is an emulator (standard AVD, LDPlayer, etc).
    /// Standard AVDs use "emulator-XXXX", LDPlayer uses "127.0.0.1:555X" or "localhost:555X".
    pub fn is_emulator(&self) -> bool {
        self.serial.starts_with("emulator-")
            || self.serial.starts_with("127.0.0.1:")
            || self.serial.starts_with("localhost:")
    }

    /// Get a short display name for this device.
    pub fn display_name(&self) -> String {
        if let Some(ref avd) = self.avd_name {
            format!("{} ({})", avd, self.serial)
        } else {
            self.serial.clone()
        }
    }
}

// ─── Emulator Manager ──────────────────────────────────────────────

/// Manages Android emulator instances via `adb` and the Android SDK `emulator` tool.
///
/// Auto-detects binaries from PATH or ANDROID_HOME. All subprocess calls use
/// cross-platform execution (`cmd /C` on Windows, direct on Unix).
pub struct EmulatorManager {
    adb_path: PathBuf,
    emulator_path: PathBuf,
    /// Directory for storing screenshots pulled from devices.
    screenshots_dir: PathBuf,
    /// Maximum number of concurrent emulator instances (default: 2).
    max_concurrent: usize,
}

impl EmulatorManager {
    /// Create a new EmulatorManager, auto-detecting adb and emulator binaries.
    pub fn new(base_dir: &Path) -> Result<Self> {
        let adb_path = detect_adb().context(
            "Could not find 'adb'. Install Android SDK Platform-Tools or set ANDROID_HOME.",
        )?;
        let emulator_path = detect_emulator().context(
            "Could not find 'emulator'. Install Android SDK Emulator or set ANDROID_HOME.",
        )?;

        let screenshots_dir = base_dir.join("shield-screenshots");
        std::fs::create_dir_all(&screenshots_dir).ok();

        Ok(Self {
            adb_path,
            emulator_path,
            screenshots_dir,
            max_concurrent: 2,
        })
    }

    /// Create with explicit paths (for testing or manual configuration).
    pub fn with_paths(
        adb_path: PathBuf,
        emulator_path: PathBuf,
        base_dir: &Path,
    ) -> Self {
        let screenshots_dir = base_dir.join("shield-screenshots");
        std::fs::create_dir_all(&screenshots_dir).ok();
        Self {
            adb_path,
            emulator_path,
            screenshots_dir,
            max_concurrent: 2,
        }
    }

    /// Set max concurrent emulators.
    pub fn set_max_concurrent(&mut self, max: usize) {
        self.max_concurrent = max;
    }

    // ── AVD Listing ────────────────────────────────────────────────

    /// List all available Android Virtual Devices.
    /// Parses the output of `emulator -list-avds`.
    pub fn list_avds(&self) -> Result<Vec<AvdInfo>> {
        let output = Command::new(&self.emulator_path)
            .arg("-list-avds")
            .output()
            .context("Failed to run 'emulator -list-avds'")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let avds = parse_avd_list(&stdout);
        Ok(avds)
    }

    // ── Device Listing ─────────────────────────────────────────────

    /// List all connected ADB devices (emulators + physical).
    /// Parses `adb devices -l` output.
    pub fn list_devices(&self) -> Result<Vec<AdbDevice>> {
        let output = Command::new(&self.adb_path)
            .args(["devices", "-l"])
            .output()
            .context("Failed to run 'adb devices'")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let devices = parse_device_list(&stdout);
        Ok(devices)
    }

    /// Get only emulator devices (AVD, LDPlayer, and network emulators).
    pub fn list_emulators(&self) -> Result<Vec<AdbDevice>> {
        Ok(self
            .list_devices()?
            .into_iter()
            .filter(|d| d.is_emulator())
            .collect())
    }

    /// Connect to a device over TCP/IP (for LDPlayer, Nox, etc).
    /// LDPlayer default: 127.0.0.1:5555 (first instance), 5557, 5559...
    /// Nox default: 127.0.0.1:62001
    pub fn adb_connect(&self, address: &str) -> Result<()> {
        let output = Command::new(&self.adb_path)
            .args(["connect", address])
            .output()
            .with_context(|| format!("Failed to run 'adb connect {}'", address))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("connected") {
            tracing::info!("ADB connected to {}", address);
            Ok(())
        } else {
            anyhow::bail!("Failed to connect to {}: {}", address, stdout.trim())
        }
    }

    // ── Emulator Lifecycle ─────────────────────────────────────────

    /// Launch an Android emulator by AVD name.
    /// Returns immediately after spawning — use `wait_for_boot` to wait.
    pub fn launch_avd(&self, avd_name: &str) -> Result<()> {
        // Check max concurrent
        let running = self.list_emulators().unwrap_or_default();
        if running.len() >= self.max_concurrent {
            anyhow::bail!(
                "Maximum concurrent emulators ({}) reached. Stop one first with /emulator_stop.",
                self.max_concurrent
            );
        }

        // Verify AVD exists
        let avds = self.list_avds()?;
        if !avds.iter().any(|a| a.name == avd_name) {
            anyhow::bail!(
                "AVD '{}' not found. Available: {}",
                avd_name,
                avds.iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        // Spawn emulator process (detached, no-window on Windows)
        let mut cmd = Command::new(&self.emulator_path);
        cmd.args(["-avd", avd_name, "-no-snapshot-load"]);

        // Detach from console
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            const DETACHED_PROCESS: u32 = 0x00000008;
            cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
        }

        cmd.spawn()
            .with_context(|| format!("Failed to launch emulator AVD '{}'", avd_name))?;

        tracing::info!("Launched Android emulator: {}", avd_name);
        Ok(())
    }

    /// Stop a running device by its serial.
    /// For standard emulators: sends `emu kill`.
    /// For network devices (LDPlayer, Nox): disconnects via `adb disconnect`.
    /// For USB devices: this is a no-op (can't power off a physical phone).
    pub fn stop_device(&self, serial: &str) -> Result<()> {
        if serial.starts_with("emulator-") {
            // Standard AVD emulator — use emu kill
            let output = Command::new(&self.adb_path)
                .args(["-s", serial, "emu", "kill"])
                .output()
                .with_context(|| format!("Failed to stop emulator '{}'", serial))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Failed to stop emulator '{}': {}", serial, stderr.trim());
            }
            tracing::info!("Stopped emulator: {}", serial);
        } else if serial.contains(':') {
            // Network device (LDPlayer, Nox, remote) — disconnect
            let output = Command::new(&self.adb_path)
                .args(["disconnect", serial])
                .output()
                .with_context(|| format!("Failed to disconnect '{}'", serial))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Failed to disconnect '{}': {}", serial, stderr.trim());
            }
            tracing::info!("Disconnected network device: {}", serial);
        } else {
            anyhow::bail!(
                "Cannot stop a USB device '{}'. Unplug it or use Android settings.",
                serial
            );
        }
        Ok(())
    }

    /// Check if a device has finished booting.
    pub fn is_device_ready(&self, serial: &str) -> bool {
        let output = Command::new(&self.adb_path)
            .args(["-s", serial, "shell", "getprop", "sys.boot_completed"])
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.trim() == "1"
            }
            Err(_) => false,
        }
    }

    /// Wait for a device to finish booting (polls every 2s, up to timeout_secs).
    pub fn wait_for_boot(&self, serial: &str, timeout_secs: u32) -> Result<()> {
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_secs(timeout_secs as u64);

        while std::time::Instant::now() < deadline {
            if self.is_device_ready(serial) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }

        anyhow::bail!(
            "Emulator '{}' did not finish booting within {}s",
            serial,
            timeout_secs
        )
    }

    // ── Device Operations ──────────────────────────────────────────

    /// Set HTTP proxy on a running emulator via ADB shell settings.
    pub fn configure_proxy(
        &self,
        serial: &str,
        host: &str,
        port: u16,
    ) -> Result<()> {
        let proxy_val = format!("{}:{}", host, port);

        let output = Command::new(&self.adb_path)
            .args([
                "-s", serial, "shell", "settings", "put", "global",
                "http_proxy", &proxy_val,
            ])
            .output()
            .context("Failed to set proxy on emulator")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to set proxy: {}", stderr.trim());
        }

        tracing::info!("Set proxy {}:{} on {}", host, port, serial);
        Ok(())
    }

    /// Clear proxy settings on a running emulator.
    pub fn clear_proxy(&self, serial: &str) -> Result<()> {
        let output = Command::new(&self.adb_path)
            .args([
                "-s", serial, "shell", "settings", "put", "global",
                "http_proxy", ":0",
            ])
            .output()
            .context("Failed to clear proxy on emulator")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to clear proxy: {}", stderr.trim());
        }

        Ok(())
    }

    /// Open a URL in the emulator's default browser.
    pub fn open_url(&self, serial: &str, url: &str) -> Result<()> {
        let output = Command::new(&self.adb_path)
            .args([
                "-s", serial, "shell", "am", "start",
                "-a", "android.intent.action.VIEW",
                "-d", url,
            ])
            .output()
            .with_context(|| format!("Failed to open URL '{}' on '{}'", url, serial))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to open URL: {}", stderr.trim());
        }

        tracing::info!("Opened URL '{}' on {}", url, serial);
        Ok(())
    }

    /// Take a screenshot from the emulator and save locally.
    /// Returns the local file path.
    pub fn take_screenshot(&self, serial: &str) -> Result<PathBuf> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("{}_{}.png", serial, timestamp);
        let local_path = self.screenshots_dir.join(&filename);
        let device_path = "/sdcard/nde_screenshot.png";

        // Capture on device
        let cap_output = Command::new(&self.adb_path)
            .args(["-s", serial, "shell", "screencap", "-p", device_path])
            .output()
            .context("Failed to capture screenshot on device")?;

        if !cap_output.status.success() {
            let stderr = String::from_utf8_lossy(&cap_output.stderr);
            anyhow::bail!("screencap failed: {}", stderr.trim());
        }

        // Pull to local
        let pull_output = Command::new(&self.adb_path)
            .args([
                "-s", serial, "pull", device_path,
                &local_path.to_string_lossy(),
            ])
            .output()
            .context("Failed to pull screenshot from device")?;

        if !pull_output.status.success() {
            let stderr = String::from_utf8_lossy(&pull_output.stderr);
            anyhow::bail!("adb pull failed: {}", stderr.trim());
        }

        // Remove temp file on device
        let _ = Command::new(&self.adb_path)
            .args(["-s", serial, "shell", "rm", device_path])
            .output();

        tracing::info!("Screenshot saved: {}", local_path.display());
        Ok(local_path)
    }

    /// Get the screenshot directory path.
    pub fn screenshots_dir(&self) -> &Path {
        &self.screenshots_dir
    }
}

// ─── Binary Detection ──────────────────────────────────────────────

/// Search for a binary by name in the system PATH.
fn find_in_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var("PATH").ok()?;
    let separator = if cfg!(windows) { ';' } else { ':' };

    for dir in path_var.split(separator) {
        let candidate = PathBuf::from(dir).join(name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Detect `adb` binary from PATH, ANDROID_HOME, or default install locations.
pub fn detect_adb() -> Option<PathBuf> {
    let adb_name = if cfg!(windows) { "adb.exe" } else { "adb" };

    // Try PATH first
    if let Some(path) = find_in_path(adb_name) {
        return Some(path);
    }

    // Try ANDROID_HOME / ANDROID_SDK_ROOT
    for env_key in &["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(sdk) = std::env::var(env_key) {
            let candidate = PathBuf::from(&sdk)
                .join("platform-tools")
                .join(adb_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // Windows: try default install location
    #[cfg(windows)]
    {
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            let candidate = PathBuf::from(localappdata)
                .join("Android")
                .join("Sdk")
                .join("platform-tools")
                .join("adb.exe");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Detect `emulator` binary from PATH, ANDROID_HOME, or default install locations.
pub fn detect_emulator() -> Option<PathBuf> {
    let emu_name = if cfg!(windows) {
        "emulator.exe"
    } else {
        "emulator"
    };

    if let Some(path) = find_in_path(emu_name) {
        return Some(path);
    }

    for env_key in &["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(sdk) = std::env::var(env_key) {
            let candidate = PathBuf::from(&sdk).join("emulator").join(emu_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    #[cfg(windows)]
    {
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            let candidate = PathBuf::from(localappdata)
                .join("Android")
                .join("Sdk")
                .join("emulator")
                .join("emulator.exe");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Check whether Android SDK tools are available.
pub fn is_android_sdk_available() -> bool {
    detect_adb().is_some() && detect_emulator().is_some()
}

/// Check whether at least `adb` is available (sufficient for USB devices and LDPlayer).
pub fn is_adb_available() -> bool {
    detect_adb().is_some()
}

// ─── Output Parsers ────────────────────────────────────────────────

/// Parse `emulator -list-avds` output into AvdInfo structs.
/// Each line is an AVD name. Blank lines and lines starting with warning
/// prefixes are skipped.
fn parse_avd_list(stdout: &str) -> Vec<AvdInfo> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("INFO")
                && !line.starts_with("WARNING")
                && !line.starts_with("ERROR")
                && !line.starts_with("Parsing")
        })
        .map(|name| AvdInfo {
            name: name.to_string(),
        })
        .collect()
}

/// Parse `adb devices -l` output into AdbDevice structs.
/// Example output:
/// ```text
/// List of devices attached
/// emulator-5554          device product:sdk_gphone64_arm64 model:sdk_gphone64_arm64 transport_id:1
/// ```
fn parse_device_list(stdout: &str) -> Vec<AdbDevice> {
    stdout
        .lines()
        .filter(|line| !line.starts_with("List of") && !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                return None;
            }

            let serial = parts[0].to_string();
            let status_str = parts[1];

            let status = match status_str {
                "device" => DeviceStatus::Online,
                "offline" => DeviceStatus::Offline,
                "unauthorized" => DeviceStatus::Unauthorized,
                _ => DeviceStatus::Booting,
            };

            // Try to extract AVD name from "model:xxx" tag
            let avd_name = parts.iter().find_map(|p| {
                p.strip_prefix("model:").map(|m| m.to_string())
            });

            Some(AdbDevice {
                serial,
                status,
                avd_name,
            })
        })
        .collect()
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_avd_list_basic() {
        let output = "Pixel_7_API_34\nPixel_6a_API_33\n";
        let avds = parse_avd_list(output);
        assert_eq!(avds.len(), 2);
        assert_eq!(avds[0].name, "Pixel_7_API_34");
        assert_eq!(avds[1].name, "Pixel_6a_API_33");
    }

    #[test]
    fn test_parse_avd_list_with_warnings() {
        let output = "\
WARNING: unexpected token
INFO: some info
Pixel_7_API_34
Parsing something...
Pixel_6a_API_33
";
        let avds = parse_avd_list(output);
        assert_eq!(avds.len(), 2);
        assert_eq!(avds[0].name, "Pixel_7_API_34");
        assert_eq!(avds[1].name, "Pixel_6a_API_33");
    }

    #[test]
    fn test_parse_avd_list_empty() {
        let output = "";
        let avds = parse_avd_list(output);
        assert!(avds.is_empty());
    }

    #[test]
    fn test_parse_device_list_basic() {
        let output = "\
List of devices attached
emulator-5554          device product:sdk_gphone64_arm64 model:sdk_gphone64_arm64 transport_id:1
emulator-5556          offline
";
        let devices = parse_device_list(output);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].serial, "emulator-5554");
        assert_eq!(devices[0].status, DeviceStatus::Online);
        assert_eq!(
            devices[0].avd_name.as_deref(),
            Some("sdk_gphone64_arm64")
        );
        assert!(devices[0].is_emulator());

        assert_eq!(devices[1].serial, "emulator-5556");
        assert_eq!(devices[1].status, DeviceStatus::Offline);
        assert!(devices[1].is_emulator());
    }

    #[test]
    fn test_parse_device_list_physical() {
        let output = "\
List of devices attached
ABCD1234               device product:redfin model:Pixel_5 transport_id:2
";
        let devices = parse_device_list(output);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].serial, "ABCD1234");
        assert!(!devices[0].is_emulator());
    }

    #[test]
    fn test_parse_device_list_empty() {
        let output = "List of devices attached\n\n";
        let devices = parse_device_list(output);
        assert!(devices.is_empty());
    }

    #[test]
    fn test_parse_device_list_unauthorized() {
        let output = "\
List of devices attached
emulator-5554          unauthorized transport_id:1
";
        let devices = parse_device_list(output);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].status, DeviceStatus::Unauthorized);
    }

    #[test]
    fn test_device_status_display() {
        assert_eq!(format!("{}", DeviceStatus::Online), "online");
        assert_eq!(format!("{}", DeviceStatus::Offline), "offline");
        assert_eq!(format!("{}", DeviceStatus::Booting), "booting");
        assert_eq!(format!("{}", DeviceStatus::Unauthorized), "unauthorized");
    }
}
