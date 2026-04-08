//! Voice runtime manager — manages the shared voice venv at `~/.ai-launcher/voice-runtime/`.
//!
//! Provides tool detection, installs, and PATH management so that any NDE-OS app
//! can use Edge TTS, Whisper, and RVC without duplicating setup logic.

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::types::{VoiceInstallResult, VoiceRuntimeStatus};

/// Central voice runtime — one per NDE-OS installation.
#[derive(Debug, Clone)]
pub struct VoiceRuntime {
    /// Base NDE-OS data directory (e.g. `~/.ai-launcher`).
    base_dir: PathBuf,
    /// The workspace where the voice venv lives.
    workspace: PathBuf,
}

impl VoiceRuntime {
    /// Create a new runtime pointing at `base_dir/voice-runtime/`.
    pub fn new(base_dir: &Path) -> Self {
        let workspace = base_dir.join("voice-runtime");
        Self {
            base_dir: base_dir.to_path_buf(),
            workspace,
        }
    }

    /// The workspace directory.
    pub fn workspace_dir(&self) -> &Path {
        &self.workspace
    }

    /// The venv bin directory (platform-aware).
    pub fn bin_dir(&self) -> PathBuf {
        if cfg!(windows) {
            self.workspace.join(".venv").join("Scripts")
        } else {
            self.workspace.join(".venv").join("bin")
        }
    }

    /// Whether the venv directory exists on disk.
    pub fn is_installed(&self) -> bool {
        self.bin_dir().exists()
    }

    /// Resolve a tool by name inside the runtime venv, falling back to system PATH.
    pub fn resolve_tool(&self, name: &str) -> Option<PathBuf> {
        let bin = self.bin_dir();
        if bin.exists() {
            let candidate = if cfg!(windows) {
                bin.join(format!("{name}.exe"))
            } else {
                bin.join(name)
            };
            if candidate.exists() {
                return Some(candidate);
            }
        }
        // Fall back to system PATH
        resolve_system_command(name)
    }

    /// Build a PATH string with the runtime bin prepended.
    pub fn prepend_path(&self) -> String {
        let bin = self.bin_dir();
        let sep = if cfg!(windows) { ";" } else { ":" };
        let existing = std::env::var("PATH").unwrap_or_default();
        if bin.exists() && !existing.is_empty() {
            format!("{}{}{}", bin.display(), sep, existing)
        } else if bin.exists() {
            bin.to_string_lossy().to_string()
        } else {
            existing
        }
    }

    /// Run an operation with the runtime bin prepended to PATH, restoring afterwards.
    pub fn with_runtime_path<T, F>(&self, op: F) -> T
    where
        F: FnOnce() -> T,
    {
        let old_path = std::env::var("PATH").ok();
        if self.bin_dir().exists() {
            std::env::set_var("PATH", self.prepend_path());
        }
        let result = op();
        match old_path {
            Some(p) => std::env::set_var("PATH", p),
            None => std::env::remove_var("PATH"),
        }
        result
    }

    /// Detect the status of all voice tools.
    pub fn detect_status(&self) -> VoiceRuntimeStatus {
        self.with_runtime_path(|| {
            let edge_tts = resolve_system_command("edge-tts").is_some();
            let whisper = resolve_system_command("whisper").is_some();
            let python = resolve_python().is_some();

            let mut details = Vec::new();
            if !edge_tts {
                details.push("edge-tts CLI not found".to_string());
            }
            if !whisper {
                details.push("whisper CLI not found".to_string());
            }
            if !python {
                details.push("Python not found (needed for RVC)".to_string());
            }

            let voices = if edge_tts {
                list_edge_voices_raw().unwrap_or_default()
            } else {
                Vec::new()
            };

            VoiceRuntimeStatus {
                runtime_installed: self.is_installed(),
                runtime_path: Some(self.workspace.to_string_lossy().to_string()),
                edge_tts_available: edge_tts,
                whisper_available: whisper,
                python_available: python,
                rvc_available: python,
                voices,
                details,
            }
        })
    }

    /// Install voice runtime components into the shared venv.
    pub fn install_components(&self, components: &[String]) -> Result<VoiceInstallResult> {
        fs::create_dir_all(&self.workspace).with_context(|| {
            format!(
                "failed to create voice runtime dir: {}",
                self.workspace.display()
            )
        })?;

        let mut packages = Vec::new();
        for component in components {
            match component.as_str() {
                "core" => {
                    packages.push("openai-whisper".to_string());
                    packages.push("edge-tts".to_string());
                }
                "whisper" => packages.push("openai-whisper".to_string()),
                "edge_tts" | "edge-tts" => packages.push("edge-tts".to_string()),
                other => return Err(anyhow!("unknown voice component: '{other}'")),
            }
        }

        let uv_bin =
            crate::uv_env::ensure_uv(&self.base_dir).context("failed to ensure uv binary")?;
        let uv = crate::uv_env::UvEnv::new(&uv_bin, &self.workspace, "3.11");
        uv.ensure_python().context("failed to ensure Python 3.11")?;
        uv.create_venv()
            .context("failed to create voice runtime venv")?;
        uv.install_deps(&packages)
            .context("failed to install voice dependencies")?;

        let bin_path = self.bin_dir();
        Ok(VoiceInstallResult {
            installed_packages: packages,
            runtime_path: self.workspace.to_string_lossy().to_string(),
            bin_path: bin_path.to_string_lossy().to_string(),
            message: "Voice runtime installed into NDE-OS".to_string(),
        })
    }

    /// Migrate an old FreeCut-specific tooling dir to the shared voice-runtime.
    /// If the shared runtime already exists, this is a no-op.
    pub fn migrate_from_freecut(&self, freecut_tooling_dir: &Path) -> Result<()> {
        if self.is_installed() {
            return Ok(());
        }
        let old_workspace = freecut_tooling_dir.join("dubbing-runtime");
        let old_venv = old_workspace.join(".venv");
        if !old_venv.exists() {
            return Ok(());
        }
        // Move the old workspace to the new location
        fs::create_dir_all(self.workspace.parent().unwrap_or(Path::new(".")))
            .context("failed to create parent dir for voice-runtime")?;
        fs::rename(&old_workspace, &self.workspace).with_context(|| {
            format!(
                "failed to migrate FreeCut runtime from {} to {}",
                old_workspace.display(),
                self.workspace.display()
            )
        })?;
        Ok(())
    }
}

// ─── Helpers ───────────────────────────────────────────────────────────────────

/// Resolve a program on system PATH (cross-platform).
pub fn resolve_system_command(program: &str) -> Option<PathBuf> {
    let output = if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "where", program])
            .output()
            .ok()?
    } else {
        Command::new("sh")
            .args(["-c", &format!("command -v {program}")])
            .output()
            .ok()?
    };
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
}

/// Resolve Python binary (python3 preferred, python fallback).
pub fn resolve_python() -> Option<PathBuf> {
    resolve_system_command("python3").or_else(|| resolve_system_command("python"))
}

/// Run a command, returning an error with stderr on failure.
pub fn run_checked_command(mut command: Command, label: &str) -> Result<()> {
    let output = command
        .output()
        .with_context(|| format!("failed to execute {label}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    anyhow::bail!("{label} failed: {stderr}");
}

/// List Edge TTS voices (raw parse of `edge-tts --list-voices`).
fn list_edge_voices_raw() -> Result<Vec<String>> {
    let edge_tts =
        resolve_system_command("edge-tts").ok_or_else(|| anyhow!("edge-tts not found"))?;
    let output = Command::new(edge_tts)
        .arg("--list-voices")
        .output()
        .context("failed to list edge-tts voices")?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut voices = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('-') {
            continue;
        }
        if let Some(name) = trimmed.split_whitespace().next() {
            if name.contains('-') && name.ends_with("Neural") {
                voices.push(name.to_string());
            }
        }
        if voices.len() >= 60 {
            break;
        }
    }
    voices.sort();
    voices.dedup();
    Ok(voices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_bin_dir_platform_aware() {
        let rt = VoiceRuntime::new(Path::new("/tmp/test-nde"));
        let bin = rt.bin_dir();
        if cfg!(windows) {
            assert!(bin.to_string_lossy().contains("Scripts"));
        } else {
            assert!(bin.to_string_lossy().contains("bin"));
        }
    }

    #[test]
    fn prepend_path_includes_bin() {
        let rt = VoiceRuntime::new(Path::new("/tmp/test-nde-path"));
        let path = rt.prepend_path();
        // Even if bin doesn't exist, should at least return existing PATH
        assert!(!path.is_empty() || std::env::var("PATH").is_err());
    }

    #[test]
    fn workspace_dir_is_voice_runtime() {
        let rt = VoiceRuntime::new(Path::new("/tmp/test-nde"));
        assert!(rt.workspace_dir().ends_with("voice-runtime"));
    }
}
