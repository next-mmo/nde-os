use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

// ─── Data Types ────────────────────────────────────────────────────

/// A single LDPlayer emulator instance as reported by `ldconsole list2`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdInstance {
    /// LDPlayer internal index (0-based).
    pub index: u32,
    /// Instance name.
    pub name: String,
    /// Top-level window handle (HWND).
    pub top_hwnd: i64,
    /// Bind window handle.
    pub bind_hwnd: i64,
    /// Whether the instance is currently running.
    pub is_running: bool,
    /// PID of the instance process (-1 if not running).
    pub pid: i64,
    /// VBox PID (-1 if not running).
    pub vbox_pid: i64,
}

/// Settings for creating or modifying an LDPlayer instance.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LdInstanceSettings {
    /// Number of CPU cores (1-4).
    #[serde(default)]
    pub cpu: Option<u32>,
    /// Memory in MB (512, 1024, 2048, 4096, 8192).
    #[serde(default)]
    pub memory: Option<u32>,
    /// Resolution as "width,height,dpi" (e.g. "1280,720,240").
    #[serde(default)]
    pub resolution: Option<String>,
}

/// Detection result for LDPlayer installation.
#[derive(Debug, Clone, Serialize)]
pub struct LdPlayerDetection {
    /// Whether ldconsole.exe was found.
    pub available: bool,
    /// Full path to ldconsole.exe (if found).
    pub ldconsole_path: Option<String>,
    /// Detected LDPlayer version directory name (e.g. "LDPlayer9").
    pub version_dir: Option<String>,
}

// ─── LDPlayer Manager ──────────────────────────────────────────────

/// Manages LDPlayer emulator instances via the `ldconsole.exe` CLI.
///
/// Auto-detects the ldconsole binary from common installation paths
/// and Windows registry. All commands are wrapped as safe Rust methods
/// returning `anyhow::Result`.
pub struct LdPlayerManager {
    ldconsole_path: PathBuf,
}

impl LdPlayerManager {
    /// Create a new manager by auto-detecting `ldconsole.exe`.
    pub fn new() -> Result<Self> {
        let ldconsole_path = detect_ldconsole().context(
            "Could not find LDPlayer 'ldconsole.exe'. \
             Install LDPlayer or set its directory in PATH.",
        )?;
        Ok(Self { ldconsole_path })
    }

    /// Create with an explicit path (for testing or manual configuration).
    pub fn with_path(ldconsole_path: PathBuf) -> Self {
        Self { ldconsole_path }
    }

    /// Get the detected ldconsole path.
    pub fn ldconsole_path(&self) -> &Path {
        &self.ldconsole_path
    }

    // ── Listing ────────────────────────────────────────────────────

    /// List all LDPlayer instances using `ldconsole list2`.
    ///
    /// `list2` returns CSV-like lines:
    /// `index,name,top_hwnd,bind_hwnd,is_running,pid,vbox_pid`
    pub fn list_instances(&self) -> Result<Vec<LdInstance>> {
        let output = self.run_command(&["list2"])?;
        let instances = parse_list2_output(&output);
        Ok(instances)
    }

    /// Check if a specific instance is running.
    pub fn is_running(&self, name: &str) -> Result<bool> {
        let output = self.run_command(&["isrunning", "--name", name])?;
        Ok(output.trim() == "running")
    }

    /// Check if a specific instance is running by index.
    pub fn is_running_by_index(&self, index: u32) -> Result<bool> {
        let output = self.run_command(&["isrunning", "--index", &index.to_string()])?;
        Ok(output.trim() == "running")
    }

    // ── Lifecycle ──────────────────────────────────────────────────

    /// Launch an instance by name.
    pub fn launch(&self, name: &str) -> Result<()> {
        let output = self.run_command(&["launch", "--name", name])?;
        if output.contains("error") || output.contains("Error") {
            anyhow::bail!("Failed to launch LDPlayer '{}': {}", name, output.trim());
        }
        tracing::info!("Launched LDPlayer instance: {}", name);
        Ok(())
    }

    /// Launch an instance by index.
    pub fn launch_by_index(&self, index: u32) -> Result<()> {
        let output = self.run_command(&["launch", "--index", &index.to_string()])?;
        if output.contains("error") || output.contains("Error") {
            anyhow::bail!(
                "Failed to launch LDPlayer index {}: {}",
                index,
                output.trim()
            );
        }
        tracing::info!("Launched LDPlayer instance index: {}", index);
        Ok(())
    }

    /// Quit (stop) an instance by name.
    pub fn quit(&self, name: &str) -> Result<()> {
        self.run_command(&["quit", "--name", name])?;
        tracing::info!("Quit LDPlayer instance: {}", name);
        Ok(())
    }

    /// Quit (stop) an instance by index.
    pub fn quit_by_index(&self, index: u32) -> Result<()> {
        self.run_command(&["quit", "--index", &index.to_string()])?;
        tracing::info!("Quit LDPlayer instance index: {}", index);
        Ok(())
    }

    /// Quit all running instances.
    pub fn quit_all(&self) -> Result<()> {
        self.run_command(&["quitall"])?;
        tracing::info!("Quit all LDPlayer instances");
        Ok(())
    }

    // ── CRUD ───────────────────────────────────────────────────────

    /// Create a new instance with the given name.
    pub fn create_instance(&self, name: &str) -> Result<()> {
        self.run_command(&["add", "--name", name])?;
        tracing::info!("Created LDPlayer instance: {}", name);
        Ok(())
    }

    /// Clone (copy) an existing instance by source name.
    pub fn clone_instance(&self, new_name: &str, from_name: &str) -> Result<()> {
        self.run_command(&["copy", "--name", new_name, "--from", from_name])?;
        tracing::info!(
            "Cloned LDPlayer instance '{}' from '{}'",
            new_name,
            from_name
        );
        Ok(())
    }

    /// Clone by source index.
    pub fn clone_instance_by_index(&self, new_name: &str, from_index: u32) -> Result<()> {
        self.run_command(&[
            "copy",
            "--name",
            new_name,
            "--from",
            &from_index.to_string(),
        ])?;
        tracing::info!(
            "Cloned LDPlayer instance '{}' from index {}",
            new_name,
            from_index
        );
        Ok(())
    }

    /// Remove (delete) an instance by name.
    /// The instance must not be running.
    pub fn remove_instance(&self, name: &str) -> Result<()> {
        // Safety check
        if self.is_running(name).unwrap_or(false) {
            anyhow::bail!(
                "Cannot remove LDPlayer instance '{}' while it is running. Stop it first.",
                name
            );
        }
        self.run_command(&["remove", "--name", name])?;
        tracing::info!("Removed LDPlayer instance: {}", name);
        Ok(())
    }

    /// Remove by index.
    pub fn remove_instance_by_index(&self, index: u32) -> Result<()> {
        if self.is_running_by_index(index).unwrap_or(false) {
            anyhow::bail!(
                "Cannot remove LDPlayer instance index {} while it is running. Stop it first.",
                index
            );
        }
        self.run_command(&["remove", "--index", &index.to_string()])?;
        tracing::info!("Removed LDPlayer instance index: {}", index);
        Ok(())
    }

    // ── Configuration ──────────────────────────────────────────────

    /// Modify instance settings (CPU, memory, resolution).
    pub fn modify_instance(&self, name: &str, settings: &LdInstanceSettings) -> Result<()> {
        let mut args: Vec<String> = vec!["modify".into(), "--name".into(), name.into()];

        if let Some(cpu) = settings.cpu {
            args.push("--cpu".into());
            args.push(cpu.to_string());
        }
        if let Some(memory) = settings.memory {
            args.push("--memory".into());
            args.push(memory.to_string());
        }
        if let Some(ref resolution) = settings.resolution {
            args.push("--resolution".into());
            args.push(resolution.clone());
        }

        // Only run if there are settings to change
        if args.len() <= 3 {
            return Ok(()); // No settings to apply
        }

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.run_command(&args_refs)?;
        tracing::info!("Modified LDPlayer instance '{}': {:?}", name, settings);
        Ok(())
    }

    // ── Internal ───────────────────────────────────────────────────

    /// Run an ldconsole command and return stdout as a string.
    fn run_command(&self, args: &[&str]) -> Result<String> {
        let output = Command::new(&self.ldconsole_path)
            .args(args)
            .output()
            .with_context(|| {
                format!(
                    "Failed to run ldconsole {}",
                    args.first().unwrap_or(&"(empty)")
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() && !stderr.is_empty() {
            anyhow::bail!(
                "ldconsole {} failed: {}",
                args.first().unwrap_or(&"(empty)"),
                stderr.trim()
            );
        }

        Ok(stdout)
    }
}

// ─── Detection ─────────────────────────────────────────────────────

/// Common LDPlayer installation directories on Windows.
const LDPLAYER_DIRS: &[&str] = &[
    "LDPlayer",
    "LDPlayer9",
    "LDPlayer4",
    "LDPlayer5",
];

/// Detect `ldconsole.exe` from common installation paths.
///
/// Search order:
/// 1. PATH environment variable
/// 2. `ProgramFiles` / `ProgramFiles(x86)` common locations
/// 3. Drive root common locations (C:\, D:\)
pub fn detect_ldconsole() -> Option<PathBuf> {
    let exe_name = "ldconsole.exe";

    // 1. Check PATH
    if let Some(path) = find_in_path(exe_name) {
        return Some(path);
    }

    // 2. Check ProgramFiles locations
    for env_key in &["ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(base) = std::env::var(env_key) {
            for dir in LDPLAYER_DIRS {
                let candidate = PathBuf::from(&base).join(dir).join(exe_name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    // 3. Check common drive roots
    for drive in &["C:", "D:", "E:"] {
        for dir in LDPLAYER_DIRS {
            let candidate = PathBuf::from(drive).join(dir).join(exe_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // 4. Check user-specific locations
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        for dir in LDPLAYER_DIRS {
            let candidate = PathBuf::from(&localappdata).join(dir).join(exe_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Check whether LDPlayer's `ldconsole.exe` is available on this system.
pub fn is_ldplayer_available() -> bool {
    detect_ldconsole().is_some()
}

/// Get full detection info for LDPlayer installation.
pub fn detect_ldplayer() -> LdPlayerDetection {
    match detect_ldconsole() {
        Some(path) => {
            let version_dir = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string());
            LdPlayerDetection {
                available: true,
                ldconsole_path: Some(path.to_string_lossy().to_string()),
                version_dir,
            }
        }
        None => LdPlayerDetection {
            available: false,
            ldconsole_path: None,
            version_dir: None,
        },
    }
}

// ─── Helpers ───────────────────────────────────────────────────────

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var("PATH").ok()?;
    for dir in path_var.split(';') {
        let candidate = PathBuf::from(dir).join(name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Parse the output of `ldconsole list2`.
///
/// Each line: `index,name,top_hwnd,bind_hwnd,is_running,pid,vbox_pid`
/// Example: `0,LDPlayer,0,0,0,-1,-1`
fn parse_list2_output(stdout: &str) -> Vec<LdInstance> {
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 7 {
                return None;
            }

            let index = parts[0].trim().parse::<u32>().ok()?;
            let name = parts[1].trim().to_string();
            let top_hwnd = parts[2].trim().parse::<i64>().unwrap_or(0);
            let bind_hwnd = parts[3].trim().parse::<i64>().unwrap_or(0);
            let is_running = parts[4].trim() == "1";
            let pid = parts[5].trim().parse::<i64>().unwrap_or(-1);
            let vbox_pid = parts[6].trim().parse::<i64>().unwrap_or(-1);

            Some(LdInstance {
                index,
                name,
                top_hwnd,
                bind_hwnd,
                is_running,
                pid,
                vbox_pid,
            })
        })
        .collect()
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_list2_basic() {
        let output = "0,LDPlayer,0,0,0,-1,-1\n1,LDPlayer-1,12345,67890,1,1234,5678\n";
        let instances = parse_list2_output(output);

        assert_eq!(instances.len(), 2);

        assert_eq!(instances[0].index, 0);
        assert_eq!(instances[0].name, "LDPlayer");
        assert!(!instances[0].is_running);
        assert_eq!(instances[0].pid, -1);

        assert_eq!(instances[1].index, 1);
        assert_eq!(instances[1].name, "LDPlayer-1");
        assert!(instances[1].is_running);
        assert_eq!(instances[1].pid, 1234);
        assert_eq!(instances[1].vbox_pid, 5678);
    }

    #[test]
    fn test_parse_list2_empty() {
        let output = "";
        let instances = parse_list2_output(output);
        assert!(instances.is_empty());
    }

    #[test]
    fn test_parse_list2_whitespace_lines() {
        let output = "  \n0,Test,0,0,0,-1,-1\n  \n";
        let instances = parse_list2_output(output);
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].name, "Test");
    }

    #[test]
    fn test_parse_list2_malformed_line() {
        let output = "not,enough,fields\n0,OK,0,0,0,-1,-1\n";
        let instances = parse_list2_output(output);
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].name, "OK");
    }

    #[test]
    fn test_detection_info_when_unavailable() {
        // detect_ldplayer should still return a valid struct on any platform
        let info = detect_ldplayer();
        // We can't guarantee LDPlayer is installed, so just verify struct shape
        assert!(!info.available || info.ldconsole_path.is_some());
    }

    #[test]
    fn test_ld_instance_settings_default() {
        let settings = LdInstanceSettings::default();
        assert!(settings.cpu.is_none());
        assert!(settings.memory.is_none());
        assert!(settings.resolution.is_none());
    }
}
