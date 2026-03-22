use crate::manifest::{AppManifest, AppStatus, InstalledApp};
use crate::sandbox::Sandbox;
use crate::uv_env::{self, UvEnv};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

pub struct AppManager {
    base_dir: PathBuf,
    uv_bin: PathBuf,
    running: Arc<Mutex<HashMap<String, Child>>>,
    registry: Arc<Mutex<HashMap<String, InstalledApp>>>,
}

impl Clone for AppManager {
    fn clone(&self) -> Self {
        Self {
            base_dir: self.base_dir.clone(),
            uv_bin: self.uv_bin.clone(),
            running: Arc::clone(&self.running),
            registry: Arc::clone(&self.registry),
        }
    }
}

impl AppManager {
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir)?;

        // Bootstrap uv
        let uv_bin = uv_env::ensure_uv(&base_dir)?;
        println!("  [uv] Using: {}", uv_bin.display());

        let registry_path = base_dir.join("registry.json");
        let registry: HashMap<String, InstalledApp> = if registry_path.exists() {
            serde_json::from_str(&fs::read_to_string(&registry_path)?).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(Self {
            base_dir,
            uv_bin,
            running: Arc::new(Mutex::new(HashMap::new())),
            registry: Arc::new(Mutex::new(registry)),
        })
    }

    fn app_workspace(&self, app_id: &str) -> PathBuf {
        self.base_dir.join(app_id).join("workspace")
    }

    fn create_sandbox(&self, app_id: &str) -> Result<Sandbox> {
        let workspace = self.app_workspace(app_id);
        let mut sandbox = Sandbox::new(&workspace)?;

        if cfg!(unix) {
            sandbox.allow_readonly("/usr");
            sandbox.allow_readonly("/lib");
            sandbox.allow_readonly("/lib64");
            sandbox.allow_readonly("/etc/ssl");
        }
        if cfg!(windows) {
            sandbox.allow_readonly("C:\\Windows\\System32");
        }

        sandbox.init_workspace()?;
        Ok(sandbox)
    }

    fn uv_for_app(&self, app_id: &str, python_version: &str) -> UvEnv {
        UvEnv::new(&self.uv_bin, &self.app_workspace(app_id), python_version)
    }

    /// Install an app: sandbox + uv venv + dependencies
    pub fn install(&self, manifest: &AppManifest) -> Result<()> {
        {
            let reg = self.registry.lock().unwrap();
            if reg.contains_key(&manifest.id) {
                return Err(anyhow!("App '{}' is already installed", manifest.id));
            }
        }

        // 1. Create sandbox
        println!("\n  [1/4] Creating sandbox...");
        let sandbox = self.create_sandbox(&manifest.id)?;
        let verify = sandbox.verify();
        if !verify.path_traversal_blocked || !verify.absolute_escape_blocked {
            return Err(anyhow!("Sandbox security verification failed"));
        }
        println!("  [sandbox] Verified secure");

        // Copy app source files from manifests/ directory if available
        let current_dir = std::env::current_dir().unwrap_or_default();
        let possible_dirs = vec![
            current_dir.join("manifests").join(&manifest.id),
            current_dir.join("..").join("manifests").join(&manifest.id),
            current_dir.join("..").join("..").join("manifests").join(&manifest.id),
        ];

        let mut found = false;
        for dir in possible_dirs {
            if dir.exists() {
                println!("  [install] Copying app files from {:?}...", dir);
                Self::copy_app_files(&dir, sandbox.root())?;
                found = true;
                break;
            }
        }
        
        if !found {
            println!("  [install] Warning: No local manifest directory found to copy files from.");
        }

        // 2. Setup uv + Python (best-effort — sandbox is the security layer)
        println!("  [2/4] Setting up Python environment...");
        let uv = self.uv_for_app(&manifest.id, &manifest.python_version);
        if let Err(e) = uv.ensure_python() {
            println!("  [uv] Python setup skipped: {} (will use system Python)", e);
        }

        // 3. Create venv (best-effort)
        println!("  [3/4] Creating virtual environment...");
        let has_venv = match uv.create_venv() {
            Ok(()) => true,
            Err(e) => {
                println!("  [uv] Venv creation skipped: {} (will use system Python)", e);
                false
            }
        };

        // 4. Install python deps
        if has_venv && !manifest.pip_deps.is_empty() {
            println!("  [4/5] Installing dependencies via uv...");
            if let Err(e) = uv.install_deps(&manifest.pip_deps) {
                println!("  [uv] Dep install issue: {} (app may need manual setup)", e);
            }
            let req_file = sandbox.root().join("requirements.txt");
            uv.install_requirements(&req_file).ok();
        } else if !manifest.pip_deps.is_empty() {
            println!("  [4/5] Skipping deps (no venv) — install manually or re-run with network");
        } else {
            println!("  [4/5] No Python dependencies needed");
        }

        // 5. Install Node.js deps if package.json exists
        let package_json = sandbox.root().join("package.json");
        if package_json.exists() {
            println!("  [5/5] Found package.json, running npm install...");
            let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };
            let npm_run = std::process::Command::new(npm_cmd)
                .arg("install")
                .current_dir(sandbox.root())
                .status();
            match npm_run {
                Ok(status) if !status.success() => println!("  [npm] install exited with {}", status),
                Err(e) => println!("  [npm] install failed: {}", e),
                _ => println!("  [npm] installed successfully"),
            }
        } else {
            println!("  [5/5] No package.json found");
        }

        let installed = InstalledApp {
            manifest: manifest.clone(),
            status: AppStatus::Installed,
            workspace: sandbox.root().to_string_lossy().into(),
            installed_at: Some(chrono::Utc::now().to_rfc3339()),
            last_run: None,
        };

        {
            let mut reg = self.registry.lock().unwrap();
            reg.insert(manifest.id.clone(), installed);
        }
        self.save_registry()?;
        println!("  [done] {} installed", manifest.name);
        Ok(())
    }

    pub fn verify_sandbox(&self, app_id: &str) -> Result<crate::sandbox::SandboxVerifyResult> {
        let workspace = self.app_workspace(app_id);
        let sandbox = Sandbox::new(&workspace)?;
        sandbox.init_workspace()?;
        Ok(sandbox.verify())
    }

    /// Launch app inside sandbox with uv venv activated
    pub fn launch(&self, app_id: &str) -> Result<(u32, u16)> {
        let app = {
            let reg = self.registry.lock().unwrap();
            reg.get(app_id)
                .ok_or_else(|| anyhow!("App '{}' not installed", app_id))?
                .clone()
        };

        {
            let running = self.running.lock().unwrap();
            if running.contains_key(app_id) {
                return Err(anyhow!("App '{}' is already running", app_id));
            }
        }

        let workspace = PathBuf::from(&app.workspace);
        let sandbox = Sandbox::new(&workspace)?;
        let uv = self.uv_for_app(app_id, &app.manifest.python_version);

        // Build environment: sandbox vars + venv vars
        let mut env_vars = sandbox.env_vars();
        env_vars.extend(uv.env_vars());
        env_vars.extend(app.manifest.env.clone());

        // Build the launch command using venv Python
        let launch_cmd = if uv.has_venv() {
            uv.build_launch_cmd(&app.manifest.launch_cmd)
        } else {
            app.manifest.launch_cmd.clone()
        };

        // Spawn
        let child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", &launch_cmd])
                .current_dir(sandbox.root())
                .envs(env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .with_context(|| format!("Failed to launch: {}", launch_cmd))?
        } else {
            Command::new("sh")
                .args(["-c", &launch_cmd])
                .current_dir(sandbox.root())
                .envs(env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .with_context(|| format!("Failed to launch: {}", launch_cmd))?
        };

        let pid = child.id();
        let port = app.manifest.port;

        {
            let mut running = self.running.lock().unwrap();
            running.insert(app_id.to_string(), child);
        }
        {
            let mut reg = self.registry.lock().unwrap();
            if let Some(entry) = reg.get_mut(app_id) {
                entry.status = AppStatus::Running { pid, port };
                entry.last_run = Some(chrono::Utc::now().to_rfc3339());
            }
        }
        self.save_registry()?;
        Ok((pid, port))
    }

    pub fn stop(&self, app_id: &str) -> Result<()> {
        {
            let mut running = self.running.lock().unwrap();
            if let Some(mut child) = running.remove(app_id) {
                let pid = child.id();
                
                #[cfg(windows)]
                {
                    std::process::Command::new("taskkill")
                        .args(&["/F", "/T", "/PID", &pid.to_string()])
                        .output()
                        .ok();
                }
                
                #[cfg(not(windows))]
                {
                    child.kill().ok();
                }
                
                child.wait().ok();
            } else {
                return Err(anyhow!("App '{}' is not running", app_id));
            }
        }
        {
            let mut reg = self.registry.lock().unwrap();
            if let Some(entry) = reg.get_mut(app_id) {
                entry.status = AppStatus::Installed;
            }
        }
        self.save_registry()?;
        Ok(())
    }

    pub fn uninstall(&self, app_id: &str) -> Result<()> {
        self.stop(app_id).ok();
        let app_dir = self.base_dir.join(app_id);
        if app_dir.exists() {
            fs::remove_dir_all(&app_dir)?;
        }
        {
            let mut reg = self.registry.lock().unwrap();
            reg.remove(app_id);
        }
        self.save_registry()?;
        Ok(())
    }

    pub fn list_apps(&self) -> Vec<InstalledApp> {
        self.registry.lock().unwrap().values().cloned().collect()
    }

    pub fn get_app(&self, app_id: &str) -> Option<InstalledApp> {
        self.registry.lock().unwrap().get(app_id).cloned()
    }

    pub fn catalog(&self) -> Vec<AppManifest> {
        vec![
            AppManifest::sample_node_fullstack(),
            AppManifest::sample_counter(),
            AppManifest::stable_diffusion(),
            AppManifest::ollama()
        ]
    }

    pub fn disk_usage(&self, app_id: &str) -> Result<u64> {
        Sandbox::new(&self.app_workspace(app_id))?.disk_usage()
    }

    pub fn running_count(&self) -> usize { self.running.lock().unwrap().len() }
    pub fn total_count(&self) -> usize { self.registry.lock().unwrap().len() }
    pub fn base_dir(&self) -> &Path { &self.base_dir }
    pub fn uv_bin(&self) -> &Path { &self.uv_bin }

    /// Get uv info for the system endpoint
    pub fn uv_info(&self) -> serde_json::Value {
        let uv = UvEnv::new(&self.uv_bin, &self.base_dir, "3");
        serde_json::json!({
            "uv_path": self.uv_bin.to_string_lossy(),
            "uv_version": uv.uv_version().unwrap_or_else(|| "unknown".into()),
        })
    }

    fn save_registry(&self) -> Result<()> {
        let reg = self.registry.lock().unwrap();
        fs::write(self.base_dir.join("registry.json"), serde_json::to_string_pretty(&*reg)?)?;
        Ok(())
    }

    /// Copy app source files from manifest dir to workspace (skips manifest.json)
    fn copy_app_files(src: &Path, dst: &Path) -> Result<()> {
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Skip the manifest metadata
            if name_str == "manifest.json" {
                continue;
            }
            let target = dst.join(&name);
            if entry.file_type()?.is_dir() {
                Self::copy_dir_recursive(&entry.path(), &target)?;
            } else {
                fs::copy(entry.path(), &target)?;
                println!("  [install] Copied {}", name_str);
            }
        }
        Ok(())
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let target = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                Self::copy_dir_recursive(&entry.path(), &target)?;
            } else {
                fs::copy(entry.path(), &target)?;
            }
        }
        Ok(())
    }
}
