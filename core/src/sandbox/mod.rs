use anyhow::{anyhow, Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Cross-platform filesystem jail.
/// Works on both Linux and Windows without WSL.
#[derive(Debug, Clone)]
pub struct Sandbox {
    root: PathBuf,
    readonly_paths: Vec<PathBuf>,
    blocked_names: HashSet<String>,
}

impl Sandbox {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        fs::create_dir_all(root)
            .with_context(|| format!("Failed to create sandbox: {}", root.display()))?;

        let root = root
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize: {}", root.display()))?;

        // Platform-specific permissions
        Self::set_restrictive_permissions(&root)?;

        let mut blocked_names = HashSet::new();
        for name in &[".ssh", ".gnupg", ".bash_history", ".env", ".git"] {
            blocked_names.insert(name.to_string());
        }

        Ok(Self {
            root,
            readonly_paths: Vec::new(),
            blocked_names,
        })
    }

    /// Set directory permissions (owner-only access)
    #[cfg(unix)]
    fn set_restrictive_permissions(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        Ok(())
    }

    #[cfg(windows)]
    fn set_restrictive_permissions(path: &Path) -> Result<()> {
        // Use icacls to restrict to current user only
        let path_str = path.to_string_lossy();
        // Remove inherited permissions, grant full control to current user
        Command::new("icacls")
            .args([
                path_str.as_ref(),
                "/inheritance:r",
                "/grant:r",
                "%USERNAME%:(OI)(CI)F",
            ])
            .output()
            .ok(); // Best-effort on Windows
        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    fn set_restrictive_permissions(_path: &Path) -> Result<()> {
        Ok(())
    }

    pub fn allow_readonly(&mut self, path: impl AsRef<Path>) -> &mut Self {
        if let Ok(p) = path.as_ref().canonicalize() {
            self.readonly_paths.push(p);
        }
        self
    }

    /// Resolve a path and ensure it stays inside the jail.
    /// Blocks: path traversal, absolute escapes, symlink escapes.
    pub fn resolve(&self, path: &Path) -> Result<PathBuf> {
        let full = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        // Check blocked filenames
        if let Some(name) = full.file_name().and_then(|n| n.to_str()) {
            if self.blocked_names.contains(name) {
                return Err(anyhow!("Blocked filename: {}", name));
            }
        }

        // For existing paths: canonicalize resolves symlinks
        if full.exists() {
            let canonical = full
                .canonicalize()
                .with_context(|| format!("Failed to canonicalize: {}", full.display()))?;

            if canonical.starts_with(&self.root) {
                return Ok(canonical);
            }
            for ro in &self.readonly_paths {
                if canonical.starts_with(ro) {
                    return Ok(canonical);
                }
            }
            return Err(anyhow!(
                "Path escapes sandbox: {} -> {}",
                path.display(),
                canonical.display()
            ));
        }

        // For non-existing paths: verify parent chain
        if let Some(parent) = full.parent() {
            if parent.exists() {
                let cp = parent.canonicalize()?;
                if cp.starts_with(&self.root) {
                    return Ok(full);
                }
                return Err(anyhow!("Parent escapes sandbox: {}", cp.display()));
            }
        }

        // Walk up until we find an existing ancestor
        let mut check = full.clone();
        while let Some(parent) = check.parent() {
            if parent.exists() {
                let cp = parent.canonicalize()?;
                if cp.starts_with(&self.root) {
                    return Ok(full);
                }
                return Err(anyhow!("Ancestor escapes sandbox"));
            }
            check = parent.to_path_buf();
        }

        Err(anyhow!("Cannot verify path inside sandbox"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Create standard workspace subdirectories
    pub fn init_workspace(&self) -> Result<()> {
        for dir in &["data", "models", "outputs", "logs", "tmp", "config"] {
            fs::create_dir_all(self.root.join(dir))?;
        }
        let info = serde_json::json!({
            "sandbox_root": self.root.to_string_lossy(),
            "created_at": chrono::Utc::now().to_rfc3339(),
            "platform": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "readonly_paths": self.readonly_paths.iter()
                .map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>(),
        });
        fs::write(
            self.root.join(".sandbox_info"),
            serde_json::to_string_pretty(&info)?,
        )?;
        Ok(())
    }

    /// Generate sandboxed environment variables (cross-platform)
    pub fn env_vars(&self) -> Vec<(String, String)> {
        let root_str = self.root.to_string_lossy().to_string();
        let sep = if cfg!(windows) { "\\" } else { "/" };
        vec![
            ("SANDBOX_ROOT".into(), root_str.clone()),
            ("HOME".into(), root_str.clone()),
            ("USERPROFILE".into(), root_str.clone()), // Windows HOME equivalent
            ("TMPDIR".into(), format!("{}{}{}", root_str, sep, "tmp")),
            ("TEMP".into(), format!("{}{}{}", root_str, sep, "tmp")), // Windows
            ("TMP".into(), format!("{}{}{}", root_str, sep, "tmp")),  // Windows
            (
                "XDG_DATA_HOME".into(),
                format!("{}{}{}", root_str, sep, "data"),
            ),
            (
                "XDG_CONFIG_HOME".into(),
                format!("{}{}{}", root_str, sep, "config"),
            ),
            (
                "XDG_CACHE_HOME".into(),
                format!("{}{}{}", root_str, sep, "tmp"),
            ),
            ("APPDATA".into(), format!("{}{}{}", root_str, sep, "config")), // Windows
            (
                "LOCALAPPDATA".into(),
                format!("{}{}{}", root_str, sep, "data"),
            ), // Windows
            (
                "PIP_TARGET".into(),
                format!("{}{}{}", root_str, sep, "pip_packages"),
            ),
            ("PYTHONDONTWRITEBYTECODE".into(), "1".into()),
        ]
    }

    /// Verify sandbox blocks all escape methods. Returns structured result.
    pub fn verify(&self) -> SandboxVerifyResult {
        // Test platform-appropriate path traversal
        let path_traversal = if cfg!(windows) {
            self.resolve(Path::new("..\\..\\..\\Windows\\System32\\config\\SAM"))
                .is_err()
                && self.resolve(Path::new("../../../etc/passwd")).is_err()
        } else {
            self.resolve(Path::new("../../../etc/passwd")).is_err()
        };

        let absolute_escape = if cfg!(windows) {
            // Test with a path that won't be in readonly_paths
            self.resolve(Path::new("C:\\Users\\Public")).is_err()
        } else {
            self.resolve(Path::new("/etc/shadow")).is_err()
        };

        let symlink_escape = self.test_symlink_escape();
        let valid_path = self.resolve(Path::new("data/test.txt")).is_ok();

        SandboxVerifyResult {
            path_traversal_blocked: path_traversal,
            absolute_escape_blocked: absolute_escape,
            symlink_escape_blocked: symlink_escape,
            valid_path_works: valid_path,
            sandbox_root: self.root.to_string_lossy().into(),
            platform: std::env::consts::OS.to_string(),
        }
    }

    fn test_symlink_escape(&self) -> bool {
        let link_path = self.root.join("tmp").join("_escape_test");

        #[cfg(unix)]
        let created = std::os::unix::fs::symlink("/etc", &link_path).is_ok();

        #[cfg(windows)]
        let created = std::os::windows::fs::symlink_dir("C:\\Windows", &link_path).is_ok();

        #[cfg(not(any(unix, windows)))]
        let created = false;

        if created {
            let escaped = self.resolve(Path::new("tmp/_escape_test")).is_ok();
            fs::remove_file(&link_path).ok();
            fs::remove_dir(&link_path).ok();
            !escaped // blocked = true
        } else {
            true // can't create symlinks = safe
        }
    }

    pub fn disk_usage(&self) -> Result<u64> {
        fn dir_size(path: &Path) -> Result<u64> {
            let mut total = 0u64;
            if path.is_dir() {
                for entry in fs::read_dir(path)? {
                    let entry = entry?;
                    let ft = entry.file_type()?;
                    if ft.is_dir() {
                        total += dir_size(&entry.path())?;
                    } else if ft.is_file() {
                        total += entry.metadata()?.len();
                    }
                }
            }
            Ok(total)
        }
        dir_size(&self.root)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxVerifyResult {
    pub path_traversal_blocked: bool,
    pub absolute_escape_blocked: bool,
    pub symlink_escape_blocked: bool,
    pub valid_path_works: bool,
    pub sandbox_root: String,
    pub platform: String,
}
