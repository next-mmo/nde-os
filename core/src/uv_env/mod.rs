//! UV-based Python environment manager.
//!
//! Handles:
//! - Bootstrap uv binary (download if missing)
//! - Install specific Python versions via `uv python install`
//! - Create per-app venvs via `uv venv`
//! - Install pip deps via `uv pip install`
//! - Run commands inside the venv
//!
//! Cross-platform: Linux + Windows native (no WSL).

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// UV environment for a single app.
#[derive(Debug, Clone)]
pub struct UvEnv {
    /// Path to the uv binary
    uv_bin: PathBuf,
    /// App workspace root
    workspace: PathBuf,
    /// Path to the venv inside workspace
    venv_path: PathBuf,
    /// Python version for this app
    python_version: String,
}

/// Where we store the bundled uv binary
fn uv_home(base_dir: &Path) -> PathBuf {
    base_dir.join(".uv")
}

/// Name of the uv binary
fn uv_binary_name() -> &'static str {
    if cfg!(windows) { "uv.exe" } else { "uv" }
}

/// Check if uv is available on system PATH
fn find_system_uv() -> Option<PathBuf> {
    let cmd = if cfg!(windows) { "where" } else { "which" };
    Command::new(cmd)
        .arg("uv")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let path = String::from_utf8_lossy(&o.stdout).trim().lines().next()?.to_string();
                if !path.is_empty() { Some(PathBuf::from(path)) } else { None }
            } else {
                None
            }
        })
}

/// Bootstrap: ensure uv is available. Returns path to uv binary.
/// Priority: 1) bundled in base_dir/.uv/ 2) system PATH 3) download
pub fn ensure_uv(base_dir: &Path) -> Result<PathBuf> {
    let uv_dir = uv_home(base_dir);
    let bundled = uv_dir.join(uv_binary_name());

    // 1. Check bundled
    if bundled.exists() {
        return Ok(bundled);
    }

    // 2. Check system
    if let Some(system_uv) = find_system_uv() {
        return Ok(system_uv);
    }

    // 3. Download uv
    println!("  [uv] uv not found, downloading...");
    fs::create_dir_all(&uv_dir)?;
    download_uv(&uv_dir)?;

    if bundled.exists() {
        Ok(bundled)
    } else {
        Err(anyhow!(
            "Failed to bootstrap uv. Install manually: https://docs.astral.sh/uv/getting-started/installation/"
        ))
    }
}

/// Download uv using the official installer
fn download_uv(dest_dir: &Path) -> Result<()> {
    if cfg!(windows) {
        // Windows: use PowerShell to download
        let script = format!(
            "irm https://astral.sh/uv/install.ps1 | iex; Move-Item $env:USERPROFILE\\.local\\bin\\uv.exe '{}'",
            dest_dir.join("uv.exe").to_string_lossy()
        );
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .context("Failed to run PowerShell for uv install")?;
        if !output.status.success() {
            // Fallback: direct download
            let url = "https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-pc-windows-msvc.zip";
            println!("  [uv] Trying direct download from GitHub...");
            let dl = Command::new("powershell")
                .args(["-NoProfile", "-Command", &format!(
                    "Invoke-WebRequest -Uri '{}' -OutFile '{}/uv.zip'; Expand-Archive -Path '{}/uv.zip' -DestinationPath '{}' -Force",
                    url, dest_dir.to_string_lossy(), dest_dir.to_string_lossy(), dest_dir.to_string_lossy()
                )])
                .output()
                .context("Failed to download uv")?;
            if !dl.status.success() {
                return Err(anyhow!("Could not download uv. Install manually: https://docs.astral.sh/uv/"));
            }
        }
    } else {
        // Linux/macOS: use curl + sh
        let output = Command::new("sh")
            .args(["-c", &format!(
                "curl -LsSf https://astral.sh/uv/install.sh | UV_INSTALL_DIR='{}' sh",
                dest_dir.to_string_lossy()
            )])
            .output()
            .context("Failed to run uv installer")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("uv install failed: {}", stderr));
        }
    }
    Ok(())
}

impl UvEnv {
    /// Create a new UV environment for an app.
    pub fn new(uv_bin: &Path, workspace: &Path, python_version: &str) -> Self {
        Self {
            uv_bin: uv_bin.to_path_buf(),
            workspace: workspace.to_path_buf(),
            venv_path: workspace.join(".venv"),
            python_version: python_version.to_string(),
        }
    }

    /// Run a uv command, returning output
    fn uv_cmd(&self, args: &[&str]) -> Result<std::process::Output> {
        Command::new(&self.uv_bin)
            .args(args)
            .current_dir(&self.workspace)
            .env("UV_CACHE_DIR", self.workspace.join(".uv_cache"))
            .output()
            .with_context(|| format!("Failed to run: uv {}", args.join(" ")))
    }

    /// Ensure the required Python version is available via uv
    pub fn ensure_python(&self) -> Result<String> {
        println!("  [uv] Ensuring Python {}...", self.python_version);
        let output = self.uv_cmd(&["python", "install", &self.python_version])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Not fatal — Python might already be on system
            println!("  [uv] Note: {}", stderr.trim());
        }

        // Find the installed Python
        let find = self.uv_cmd(&["python", "find", &self.python_version])?;
        let python_path = String::from_utf8_lossy(&find.stdout).trim().to_string();
        if python_path.is_empty() {
            // Fallback to system python
            let fallback = if cfg!(windows) { "python" } else { "python3" };
            println!("  [uv] Using system: {}", fallback);
            Ok(fallback.to_string())
        } else {
            println!("  [uv] Found: {}", python_path);
            Ok(python_path)
        }
    }

    /// Create a venv for this app
    pub fn create_venv(&self) -> Result<()> {
        if self.venv_path.exists() {
            println!("  [uv] Venv already exists");
            return Ok(());
        }

        println!("  [uv] Creating venv with Python {}...", self.python_version);
        let output = self.uv_cmd(&[
            "venv",
            &self.venv_path.to_string_lossy(),
            "--python", &self.python_version,
        ])?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Try without --python flag (use whatever's available)
            println!("  [uv] Retrying without specific Python version...");
            let retry = self.uv_cmd(&["venv", &self.venv_path.to_string_lossy()])?;
            if !retry.status.success() {
                return Err(anyhow!("Failed to create venv: {}", stderr));
            }
        }
        println!("  [uv] Venv created at {}", self.venv_path.display());
        Ok(())
    }

    /// Install pip dependencies into the venv
    pub fn install_deps(&self, deps: &[String]) -> Result<()> {
        if deps.is_empty() {
            println!("  [uv] No dependencies to install");
            return Ok(());
        }

        println!("  [uv] Installing {} package(s)...", deps.len());
        let venv_str = self.venv_path.to_string_lossy().to_string();
        let mut args = vec!["pip", "install", "--python", &venv_str];
        let dep_refs: Vec<&str> = deps.iter().map(|s| s.as_str()).collect();
        args.extend_from_slice(&dep_refs);

        let output = self.uv_cmd(&args)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("uv pip install failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if !line.trim().is_empty() {
                println!("  [uv] {}", line.trim());
            }
        }
        println!("  [uv] All dependencies installed");
        Ok(())
    }

    /// Install from a requirements.txt file
    pub fn install_requirements(&self, req_file: &Path) -> Result<()> {
        if !req_file.exists() {
            return Ok(());
        }
        println!("  [uv] Installing from {}...", req_file.display());
        let output = self.uv_cmd(&[
            "pip", "install",
            "--python", &self.venv_path.to_string_lossy(),
            "-r", &req_file.to_string_lossy(),
        ])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("uv pip install -r failed: {}", stderr));
        }
        Ok(())
    }

    /// Get the path to the Python binary inside the venv
    pub fn python_bin(&self) -> PathBuf {
        if cfg!(windows) {
            self.venv_path.join("Scripts").join("python.exe")
        } else {
            self.venv_path.join("bin").join("python")
        }
    }

    /// Get the path to pip inside the venv
    pub fn pip_bin(&self) -> PathBuf {
        if cfg!(windows) {
            self.venv_path.join("Scripts").join("pip.exe")
        } else {
            self.venv_path.join("bin").join("pip")
        }
    }

    /// Get the activate script path
    pub fn activate_script(&self) -> PathBuf {
        if cfg!(windows) {
            self.venv_path.join("Scripts").join("activate.bat")
        } else {
            self.venv_path.join("bin").join("activate")
        }
    }

    /// Build the launch command that runs inside the venv
    pub fn build_launch_cmd(&self, app_cmd: &str) -> String {
        let python = self.python_bin();
        // Replace python3/python in the command with the venv python
        let cmd = app_cmd
            .replace("python3 ", &format!("{} ", python.to_string_lossy()))
            .replace("python ", &format!("{} ", python.to_string_lossy()));
        cmd
    }

    /// Environment variables for the venv
    pub fn env_vars(&self) -> Vec<(String, String)> {
        let mut vars = vec![
            ("VIRTUAL_ENV".into(), self.venv_path.to_string_lossy().into()),
        ];

        // Prepend venv bin to PATH
        let bin_dir = if cfg!(windows) {
            self.venv_path.join("Scripts")
        } else {
            self.venv_path.join("bin")
        };
        if let Ok(existing_path) = std::env::var("PATH") {
            let sep = if cfg!(windows) { ";" } else { ":" };
            vars.push(("PATH".into(), format!("{}{}{}", bin_dir.to_string_lossy(), sep, existing_path)));
        } else {
            vars.push(("PATH".into(), bin_dir.to_string_lossy().into()));
        }

        vars
    }

    /// Get uv version string
    pub fn uv_version(&self) -> Option<String> {
        let output = Command::new(&self.uv_bin)
            .arg("--version")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let version = if !stdout.is_empty() { stdout } else { stderr };

        if version.is_empty() {
            None
        } else {
            Some(version)
        }
    }

    /// Check if venv exists
    pub fn has_venv(&self) -> bool {
        self.venv_path.exists() && self.python_bin().exists()
    }

    /// Get venv size
    pub fn venv_size_human(&self) -> String {
        if !self.venv_path.exists() {
            return "0".to_string();
        }
        fn dir_size(p: &Path) -> u64 {
            fs::read_dir(p).ok().map(|entries| {
                entries.filter_map(|e| e.ok()).map(|e| {
                    let m = e.metadata().ok();
                    if e.path().is_dir() { dir_size(&e.path()) }
                    else { m.map(|m| m.len()).unwrap_or(0) }
                }).sum()
            }).unwrap_or(0)
        }
        let bytes = dir_size(&self.venv_path);
        if bytes > 1_073_741_824 { format!("{:.1} GB", bytes as f64 / 1_073_741_824.0) }
        else if bytes > 1_048_576 { format!("{:.1} MB", bytes as f64 / 1_048_576.0) }
        else { format!("{:.0} KB", bytes as f64 / 1024.0) }
    }
}
