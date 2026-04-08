//! Node.js environment manager for sandboxed apps.
//!
//! Handles:
//! - Detect system Node.js and version
//! - Detect / prefer package managers: pnpm > yarn > npm
//! - Install deps from package.json (sandboxed npm cache)
//! - Build launch commands for Node.js apps
//! - Provide sandboxed env vars (NPM_CONFIG_CACHE, NODE_PATH, etc.)
//!
//! Cross-platform: Linux + macOS + Windows native (no WSL).

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Supported Node.js package managers, ranked by preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Pnpm,
    Yarn,
    Npm,
}

impl PackageManager {
    /// Binary name (cross-platform).
    pub fn bin(&self) -> &'static str {
        match self {
            Self::Pnpm => {
                if cfg!(windows) {
                    "pnpm.cmd"
                } else {
                    "pnpm"
                }
            }
            Self::Yarn => {
                if cfg!(windows) {
                    "yarn.cmd"
                } else {
                    "yarn"
                }
            }
            Self::Npm => {
                if cfg!(windows) {
                    "npm.cmd"
                } else {
                    "npm"
                }
            }
        }
    }

    /// Install command args.
    pub fn install_args(&self) -> Vec<&'static str> {
        match self {
            Self::Pnpm => vec!["install", "--frozen-lockfile"],
            Self::Yarn => vec!["install", "--frozen-lockfile"],
            Self::Npm => vec!["install"],
        }
    }

    /// Install command args (non-frozen, for first install).
    pub fn install_args_fresh(&self) -> Vec<&'static str> {
        match self {
            Self::Pnpm => vec!["install"],
            Self::Yarn => vec!["install"],
            Self::Npm => vec!["install"],
        }
    }

    /// Lock file name for detection.
    pub fn lockfile(&self) -> &'static str {
        match self {
            Self::Pnpm => "pnpm-lock.yaml",
            Self::Yarn => "yarn.lock",
            Self::Npm => "package-lock.json",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Npm => "npm",
        }
    }
}

/// Node.js environment for a single sandboxed app.
#[derive(Debug, Clone)]
pub struct NodeEnv {
    /// Path to the Node.js binary
    node_bin: PathBuf,
    /// Selected package manager
    pkg_manager: PackageManager,
    /// App workspace root (sandbox root)
    workspace: PathBuf,
}

/// Check if a binary is available on system PATH. Returns its path.
fn find_binary(name: &str) -> Option<PathBuf> {
    let cmd = if cfg!(windows) { "where" } else { "which" };
    Command::new(cmd).arg(name).output().ok().and_then(|o| {
        if o.status.success() {
            let path = String::from_utf8_lossy(&o.stdout)
                .trim()
                .lines()
                .next()?
                .to_string();
            if !path.is_empty() {
                Some(PathBuf::from(path))
            } else {
                None
            }
        } else {
            None
        }
    })
}

/// Get the version string of a binary.
fn binary_version(name: &str) -> Option<String> {
    Command::new(name)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            } else {
                None
            }
        })
}

/// Detect system Node.js. Returns (path, version) or error.
pub fn detect_node() -> Result<(PathBuf, String)> {
    let node_name = if cfg!(windows) { "node.exe" } else { "node" };
    let node_bin = find_binary(node_name).ok_or_else(|| {
        anyhow!("Node.js not found. Install from https://nodejs.org or via nvm/fnm")
    })?;
    let version = binary_version(&node_bin.to_string_lossy()).unwrap_or_else(|| "unknown".into());
    Ok((node_bin, version))
}

/// Detect the best package manager for a workspace.
/// Priority: lockfile presence > system availability (pnpm > yarn > npm).
pub fn detect_package_manager(workspace: &Path) -> PackageManager {
    // 1. Check lockfile presence
    if workspace.join("pnpm-lock.yaml").exists() {
        return PackageManager::Pnpm;
    }
    if workspace.join("yarn.lock").exists() {
        return PackageManager::Yarn;
    }
    if workspace.join("package-lock.json").exists() {
        return PackageManager::Npm;
    }

    // 2. Check system availability (prefer pnpm > yarn > npm)
    if find_binary(PackageManager::Pnpm.bin()).is_some() {
        return PackageManager::Pnpm;
    }
    if find_binary(PackageManager::Yarn.bin()).is_some() {
        return PackageManager::Yarn;
    }

    // Fallback: npm (always ships with Node.js)
    PackageManager::Npm
}

impl NodeEnv {
    /// Create a new Node.js environment for an app workspace.
    pub fn new(workspace: &Path) -> Result<Self> {
        let (node_bin, version) = detect_node()?;
        println!("  [node] Using: {} ({})", node_bin.display(), version);

        let pkg_manager = detect_package_manager(workspace);
        if let Some(pm_version) = binary_version(pkg_manager.bin()) {
            println!(
                "  [node] Package manager: {} ({})",
                pkg_manager.display_name(),
                pm_version
            );
        } else {
            println!("  [node] Package manager: {}", pkg_manager.display_name());
        }

        Ok(Self {
            node_bin,
            pkg_manager,
            workspace: workspace.to_path_buf(),
        })
    }

    /// Create with explicit package manager override.
    pub fn with_package_manager(workspace: &Path, pm: PackageManager) -> Result<Self> {
        let (node_bin, version) = detect_node()?;
        println!("  [node] Using: {} ({})", node_bin.display(), version);
        println!("  [node] Package manager: {} (forced)", pm.display_name());

        Ok(Self {
            node_bin,
            pkg_manager: pm,
            workspace: workspace.to_path_buf(),
        })
    }

    /// Install dependencies from package.json.
    /// Uses the detected/configured package manager.
    pub fn install_deps(&self) -> Result<()> {
        let package_json = self.workspace.join("package.json");
        if !package_json.exists() {
            println!("  [node] No package.json found, skipping deps");
            return Ok(());
        }

        println!(
            "  [node] Installing dependencies via {}...",
            self.pkg_manager.display_name()
        );

        let has_lockfile = self.workspace.join(self.pkg_manager.lockfile()).exists();
        let args = if has_lockfile {
            self.pkg_manager.install_args()
        } else {
            self.pkg_manager.install_args_fresh()
        };

        let status = Command::new(self.pkg_manager.bin())
            .args(&args)
            .current_dir(&self.workspace)
            .envs(
                self.env_vars()
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str())),
            )
            .status()
            .with_context(|| {
                format!(
                    "Failed to run {} {}",
                    self.pkg_manager.display_name(),
                    args.join(" ")
                )
            })?;

        if !status.success() {
            return Err(anyhow!(
                "{} install exited with code {}",
                self.pkg_manager.display_name(),
                status
            ));
        }

        println!("  [node] Dependencies installed successfully");
        Ok(())
    }

    /// Run an npm script (e.g. "build", "start").
    pub fn run_script(&self, script: &str) -> Result<std::process::ExitStatus> {
        let status = Command::new(self.pkg_manager.bin())
            .args(["run", script])
            .current_dir(&self.workspace)
            .envs(
                self.env_vars()
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str())),
            )
            .status()
            .with_context(|| {
                format!(
                    "Failed to run {} run {}",
                    self.pkg_manager.display_name(),
                    script
                )
            })?;
        Ok(status)
    }

    /// Build the launch command for a Node.js app.
    /// Resolves `node` in the command to the full path.
    pub fn build_launch_cmd(&self, app_cmd: &str) -> String {
        let node_path = self.node_bin.to_string_lossy();
        // Replace bare `node ` with full path
        let cmd = app_cmd.replace("node ", &format!("{} ", node_path));
        cmd
    }

    /// Get sandboxed environment variables for Node.js processes.
    /// Jails npm/pnpm/yarn caches inside the workspace.
    pub fn env_vars(&self) -> Vec<(String, String)> {
        let root = self.workspace.to_string_lossy().to_string();
        let sep = if cfg!(windows) { "\\" } else { "/" };

        let mut vars = vec![
            // Node.js binary location
            ("NODE_PATH".into(), format!("{}{}node_modules", root, sep)),
            // npm cache inside sandbox
            (
                "NPM_CONFIG_CACHE".into(),
                format!("{}{}.npm_cache", root, sep),
            ),
            // npm prefix (global installs go inside sandbox)
            (
                "NPM_CONFIG_PREFIX".into(),
                format!("{}{}.npm_global", root, sep),
            ),
            // Disable npm update checks (noisy in sandbox)
            ("NPM_CONFIG_UPDATE_NOTIFIER".into(), "false".into()),
            // Disable npm audit on install (slow in sandbox)
            ("NPM_CONFIG_AUDIT".into(), "false".into()),
            // Disable npm fund messages
            ("NPM_CONFIG_FUND".into(), "false".into()),
            // pnpm store inside sandbox
            ("PNPM_HOME".into(), format!("{}{}.pnpm", root, sep)),
            // yarn cache inside sandbox
            (
                "YARN_CACHE_FOLDER".into(),
                format!("{}{}.yarn_cache", root, sep),
            ),
            // Disable color for cleaner log capture
            ("NO_UPDATE_NOTIFIER".into(), "1".into()),
        ];

        // Prepend node_modules/.bin to PATH so locally installed binaries work
        let node_modules_bin = self.workspace.join("node_modules").join(".bin");
        if let Ok(existing_path) = std::env::var("PATH") {
            let path_sep = if cfg!(windows) { ";" } else { ":" };
            vars.push((
                "PATH".into(),
                format!(
                    "{}{}{}",
                    node_modules_bin.to_string_lossy(),
                    path_sep,
                    existing_path
                ),
            ));
        }

        vars
    }

    /// Check if node_modules exists.
    pub fn has_node_modules(&self) -> bool {
        self.workspace.join("node_modules").exists()
    }

    /// Node.js version string.
    pub fn node_version(&self) -> Option<String> {
        binary_version(&self.node_bin.to_string_lossy())
    }

    /// Package manager info.
    pub fn package_manager(&self) -> PackageManager {
        self.pkg_manager
    }

    /// Total size of node_modules (human readable).
    pub fn node_modules_size_human(&self) -> String {
        let nm = self.workspace.join("node_modules");
        if !nm.exists() {
            return "0".to_string();
        }
        fn dir_size(p: &Path) -> u64 {
            std::fs::read_dir(p)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| {
                            if e.path().is_dir() {
                                dir_size(&e.path())
                            } else {
                                e.metadata().ok().map(|m| m.len()).unwrap_or(0)
                            }
                        })
                        .sum()
                })
                .unwrap_or(0)
        }
        let bytes = dir_size(&nm);
        if bytes > 1_073_741_824 {
            format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
        } else if bytes > 1_048_576 {
            format!("{:.1} MB", bytes as f64 / 1_048_576.0)
        } else {
            format!("{:.0} KB", bytes as f64 / 1024.0)
        }
    }

    /// Get comprehensive node environment info for API responses.
    pub fn info(&self) -> serde_json::Value {
        serde_json::json!({
            "node_path": self.node_bin.to_string_lossy(),
            "node_version": self.node_version().unwrap_or_else(|| "unknown".into()),
            "package_manager": self.pkg_manager.display_name(),
            "package_manager_version": binary_version(self.pkg_manager.bin()),
            "has_node_modules": self.has_node_modules(),
            "node_modules_size": self.node_modules_size_human(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manager_lockfiles() {
        assert_eq!(PackageManager::Pnpm.lockfile(), "pnpm-lock.yaml");
        assert_eq!(PackageManager::Yarn.lockfile(), "yarn.lock");
        assert_eq!(PackageManager::Npm.lockfile(), "package-lock.json");
    }

    #[test]
    fn test_detect_package_manager_npm_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package-lock.json"), "{}").unwrap();
        assert_eq!(detect_package_manager(dir.path()), PackageManager::Npm);
    }

    #[test]
    fn test_detect_package_manager_pnpm_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(detect_package_manager(dir.path()), PackageManager::Pnpm);
    }

    #[test]
    fn test_detect_package_manager_yarn_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
        assert_eq!(detect_package_manager(dir.path()), PackageManager::Yarn);
    }

    #[test]
    fn test_env_vars_contain_npm_cache() {
        // Only run if node is available
        if detect_node().is_err() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let env = NodeEnv::new(dir.path()).unwrap();
        let vars = env.env_vars();
        assert!(vars.iter().any(|(k, _)| k == "NPM_CONFIG_CACHE"));
        assert!(vars.iter().any(|(k, _)| k == "NODE_PATH"));
        assert!(vars.iter().any(|(k, _)| k == "PNPM_HOME"));
    }
}
