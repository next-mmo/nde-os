use crate::manifest::{AppManifest, AppStatus, InstalledApp, SourceType, StoreUploadRequest, StoreUploadResult, ValidationError};
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

    // ── Store Upload ─────────────────────────────────────────────────────

    /// Upload an app to the store.
    /// Accepts folder, zip, or git url. Validates, stages, reads manifest.json,
    /// performs a trial install, and on success copies to manifests/.
    pub fn upload_to_store(&self, req: &StoreUploadRequest) -> Result<StoreUploadResult> {
        let mut log: Vec<String> = Vec::new();

        // ── 1. Validate input fields ────────────────────────────────────
        log.push("[upload] Validating input...".into());
        let validation_errors = Self::validate_upload(req);
        if !validation_errors.is_empty() {
            return Ok(StoreUploadResult {
                accepted: false,
                app_id: None,
                app_name: None,
                validation_errors,
                install_log: log,
            });
        }
        log.push("[upload] Input validation passed".into());

        // ── 2. Stage content into a temp dir ────────────────────────────
        let staging_dir = self.base_dir.join("_staging");
        fs::create_dir_all(&staging_dir)?;
        // Use a unique name per upload
        let stamp = chrono::Utc::now().timestamp_millis();
        let stage = staging_dir.join(format!("upload_{}", stamp));
        fs::create_dir_all(&stage)?;

        log.push(format!("[upload] Staging content to {:?}...", stage));
        if let Err(e) = Self::stage_content(req, &stage) {
            // Clean up on staging failure
            fs::remove_dir_all(&stage).ok();
            return Ok(StoreUploadResult {
                accepted: false,
                app_id: None,
                app_name: None,
                validation_errors: vec![ValidationError {
                    field: "source".into(),
                    message: format!("Failed to stage content: {}", e),
                }],
                install_log: log,
            });
        }
        log.push("[upload] Content staged successfully".into());

        // ── 3. Read and parse manifest.json ─────────────────────────────
        let manifest_path = stage.join("manifest.json");
        if !manifest_path.exists() {
            fs::remove_dir_all(&stage).ok();
            return Ok(StoreUploadResult {
                accepted: false,
                app_id: None,
                app_name: None,
                validation_errors: vec![ValidationError {
                    field: "manifest.json".into(),
                    message: "No manifest.json found in uploaded content".into(),
                }],
                install_log: log,
            });
        }

        let manifest_content = fs::read_to_string(&manifest_path)?;
        let manifest: AppManifest = match serde_json::from_str(&manifest_content) {
            Ok(m) => m,
            Err(e) => {
                fs::remove_dir_all(&stage).ok();
                return Ok(StoreUploadResult {
                    accepted: false,
                    app_id: None,
                    app_name: None,
                    validation_errors: vec![ValidationError {
                        field: "manifest.json".into(),
                        message: format!("Invalid manifest.json: {}", e),
                    }],
                    install_log: log,
                });
            }
        };

        let app_id = manifest.id.clone();
        let app_name = manifest.name.clone();
        log.push(format!("[upload] Parsed manifest: id={}, name={}", app_id, app_name));

        // ── 4. Validate manifest fields ─────────────────────────────────
        let mut manifest_errors = Vec::new();
        if app_id.is_empty() {
            manifest_errors.push(ValidationError {
                field: "manifest.id".into(),
                message: "Manifest id must not be empty".into(),
            });
        }
        if manifest.name.is_empty() {
            manifest_errors.push(ValidationError {
                field: "manifest.name".into(),
                message: "Manifest name must not be empty".into(),
            });
        }
        if manifest.launch_cmd.is_empty() {
            manifest_errors.push(ValidationError {
                field: "manifest.launch_cmd".into(),
                message: "Manifest launch_cmd must not be empty".into(),
            });
        }
        if manifest.port == 0 {
            manifest_errors.push(ValidationError {
                field: "manifest.port".into(),
                message: "Manifest port must be > 0".into(),
            });
        }
        if !manifest_errors.is_empty() {
            fs::remove_dir_all(&stage).ok();
            return Ok(StoreUploadResult {
                accepted: false,
                app_id: Some(app_id),
                app_name: Some(app_name),
                validation_errors: manifest_errors,
                install_log: log,
            });
        }

        // ── 5. Copy staged files to manifests/{app_id} ──────────────────
        let current_dir = std::env::current_dir().unwrap_or_else(|_| self.base_dir.clone());
        let manifests_dir = current_dir.join("manifests").join(&app_id);
        fs::create_dir_all(&manifests_dir)?;
        Self::copy_dir_recursive(&stage, &manifests_dir)?;
        log.push(format!("[upload] Copied to manifests/{}", app_id));

        // ── 6. Trial install ────────────────────────────────────────────
        log.push("[upload] Starting trial install...".into());
        match self.install(&manifest) {
            Ok(()) => {
                log.push("[upload] Trial install succeeded ✓".into());
            }
            Err(e) => {
                log.push(format!("[upload] Trial install FAILED: {}", e));
                // Clean up the failed install
                self.uninstall(&app_id).ok();
                // Remove from manifests/
                fs::remove_dir_all(&manifests_dir).ok();
                // Clean up staging
                fs::remove_dir_all(&stage).ok();
                return Ok(StoreUploadResult {
                    accepted: false,
                    app_id: Some(app_id),
                    app_name: Some(app_name),
                    validation_errors: vec![ValidationError {
                        field: "install".into(),
                        message: format!("Trial install failed: {}", e),
                    }],
                    install_log: log,
                });
            }
        }

        // ── 7. Clean up staging ─────────────────────────────────────────
        fs::remove_dir_all(&stage).ok();
        log.push("[upload] Upload accepted and app installed!".into());

        Ok(StoreUploadResult {
            accepted: true,
            app_id: Some(app_id),
            app_name: Some(app_name),
            validation_errors: vec![],
            install_log: log,
        })
    }

    /// Validate upload request fields before staging
    pub fn validate_upload(req: &StoreUploadRequest) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        match req.source_type {
            SourceType::Folder => {
                match &req.source_path {
                    None => errors.push(ValidationError {
                        field: "source_path".into(),
                        message: "source_path is required for folder upload".into(),
                    }),
                    Some(p) => {
                        let path = Path::new(p);
                        if !path.exists() {
                            errors.push(ValidationError {
                                field: "source_path".into(),
                                message: format!("Path does not exist: {}", p),
                            });
                        } else if !path.is_dir() {
                            errors.push(ValidationError {
                                field: "source_path".into(),
                                message: format!("Path is not a directory: {}", p),
                            });
                        } else if !path.join("manifest.json").exists() {
                            errors.push(ValidationError {
                                field: "source_path".into(),
                                message: "Directory does not contain manifest.json".into(),
                            });
                        }
                    }
                }
            }
            SourceType::Zip => {
                match &req.source_path {
                    None => errors.push(ValidationError {
                        field: "source_path".into(),
                        message: "source_path is required for zip upload".into(),
                    }),
                    Some(p) => {
                        let path = Path::new(p);
                        if !path.exists() {
                            errors.push(ValidationError {
                                field: "source_path".into(),
                                message: format!("File does not exist: {}", p),
                            });
                        } else if !path.is_file() {
                            errors.push(ValidationError {
                                field: "source_path".into(),
                                message: format!("Path is not a file: {}", p),
                            });
                        } else {
                            // Check extension
                            let ext = path.extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("");
                            if ext.to_lowercase() != "zip" {
                                errors.push(ValidationError {
                                    field: "source_path".into(),
                                    message: format!("File must have .zip extension, got: .{}", ext),
                                });
                            }
                        }
                    }
                }
            }
            SourceType::GitUrl => {
                match &req.git_url {
                    None => errors.push(ValidationError {
                        field: "git_url".into(),
                        message: "git_url is required for git_url upload".into(),
                    }),
                    Some(url) => {
                        let url_lower = url.to_lowercase();
                        // Must be http(s)
                        if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
                            errors.push(ValidationError {
                                field: "git_url".into(),
                                message: "git_url must start with http:// or https://".into(),
                            });
                        }
                        // Must look like a git repo URL
                        let known_hosts = ["github.com", "gitlab.com", "bitbucket.org", "codeberg.org"];
                        let is_known_host = known_hosts.iter().any(|h| url_lower.contains(h));
                        let has_git_ext = url_lower.ends_with(".git");
                        if !is_known_host && !has_git_ext {
                            errors.push(ValidationError {
                                field: "git_url".into(),
                                message: "git_url must end with .git or be from a known host (github.com, gitlab.com, bitbucket.org, codeberg.org)".into(),
                            });
                        }
                    }
                }
            }
        }

        errors
    }

    /// Stage content into the staging directory based on source type
    fn stage_content(req: &StoreUploadRequest, stage_dir: &Path) -> Result<()> {
        match req.source_type {
            SourceType::Folder => {
                let src = Path::new(req.source_path.as_ref().unwrap());
                println!("  [upload] Copying folder {:?} → {:?}", src, stage_dir);
                Self::copy_dir_recursive(src, stage_dir)?;
            }
            SourceType::Zip => {
                let zip_path = Path::new(req.source_path.as_ref().unwrap());
                println!("  [upload] Extracting zip {:?} → {:?}", zip_path, stage_dir);
                Self::extract_zip(zip_path, stage_dir)?;
            }
            SourceType::GitUrl => {
                let url = req.git_url.as_ref().unwrap();
                println!("  [upload] Cloning {} → {:?}", url, stage_dir);
                Self::clone_git_repo(url, stage_dir)?;
            }
        }
        Ok(())
    }

    /// Extract a ZIP file using platform-native tools
    fn extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
        let zip_str = zip_path.to_string_lossy().to_string();
        let dest_str = dest.to_string_lossy().to_string();

        let status = if cfg!(windows) {
            Command::new("powershell")
                .args([
                    "-NoProfile", "-Command",
                    &format!(
                        "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                        zip_str, dest_str
                    ),
                ])
                .status()
                .with_context(|| "Failed to run powershell Expand-Archive")?
        } else {
            Command::new("unzip")
                .args(["-o", &zip_str, "-d", &dest_str])
                .status()
                .with_context(|| "Failed to run unzip")?
        };

        if !status.success() {
            return Err(anyhow!("ZIP extraction failed with exit code: {}", status));
        }

        // Some ZIPs extract into a single subdirectory — flatten if so
        Self::flatten_single_subdir(dest)?;
        Ok(())
    }

    /// Clone a git repository (shallow clone for speed)
    fn clone_git_repo(url: &str, dest: &Path) -> Result<()> {
        let status = Command::new("git")
            .args(["clone", "--depth", "1", url, &dest.to_string_lossy()])
            .status()
            .with_context(|| format!("Failed to run git clone for {}", url))?;

        if !status.success() {
            return Err(anyhow!("git clone failed with exit code: {}", status));
        }

        // Remove .git directory from clone (not needed in sandbox)
        let dot_git = dest.join(".git");
        if dot_git.exists() {
            fs::remove_dir_all(&dot_git).ok();
        }

        Ok(())
    }

    /// If a directory contains exactly one subdirectory and nothing else,
    /// move its contents up (common with ZIP archives)
    fn flatten_single_subdir(dir: &Path) -> Result<()> {
        let entries: Vec<_> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .collect();

        if entries.len() == 1 && entries[0].file_type().map(|f| f.is_dir()).unwrap_or(false) {
            let sub = entries[0].path();
            // Move all children of sub into dir
            for child in fs::read_dir(&sub)? {
                let child = child?;
                let target = dir.join(child.file_name());
                fs::rename(child.path(), &target)?;
            }
            fs::remove_dir_all(&sub).ok();
        }

        Ok(())
    }
}

