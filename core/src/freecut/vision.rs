use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use std::process::Command;

pub struct VisionRuntime {
    base_dir: PathBuf,
}

impl VisionRuntime {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
        }
    }

    pub fn is_installed(&self) -> bool {
        let venv = self.base_dir.join(".venv_vision");
        venv.exists() && (venv.join("bin").join("rembg").exists() || venv.join("Scripts").join("rembg.exe").exists())
    }

    pub fn install(&self) -> Result<()> {
        let uv = crate::voice::runtime::resolve_system_command("uv")
            .or_else(|| {
                let local = self.base_dir.join("uv");
                if local.exists() { Some(local) } else { None }
            })
            .or_else(|| {
                let local_exe = self.base_dir.join("uv.exe");
                if local_exe.exists() { Some(local_exe) } else { None }
            })
            .context("uv not found. Please install the uv package manager.")?;

        let venv_dir = self.base_dir.join(".venv_vision");
        
        // Create venv
        let status = Command::new(&uv)
            .args(["venv", venv_dir.to_str().unwrap()])
            .status()
            .context("Failed to create vision venv via uv")?;
        
        if !status.success() {
            anyhow::bail!("Failed to create vision venv");
        }

        // Install rembg via uv pip
        let status = Command::new(&uv)
            .env("VIRTUAL_ENV", venv_dir.to_str().unwrap())
            .args(["pip", "install", "rembg[cli]"])
            .status()
            .context("Failed to install rembg via uv pip")?;

        if !status.success() {
            anyhow::bail!("Failed to install rembg in vision venv");
        }

        Ok(())
    }

    pub fn remove_background(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        if !self.is_installed() {
            anyhow::bail!("AI Vision runtime is not installed. Please install it from the Service Hub.");
        }

        let venv_dir = self.base_dir.join(".venv_vision");
        let rembg_bin = if cfg!(windows) {
            venv_dir.join("Scripts").join("rembg.exe")
        } else {
            venv_dir.join("bin").join("rembg")
        };

        // Suppress onnxruntime warnings by setting env
        let output = Command::new(rembg_bin)
            .env("REMDBG", "0")
            .arg("i")
            .arg(input_path.to_str().unwrap())
            .arg(output_path.to_str().unwrap())
            .output()
            .context("Failed to run rembg")?;

        if !output.status.success() {
            anyhow::bail!("Rembg failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }
}
