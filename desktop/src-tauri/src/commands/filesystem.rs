/// Filesystem commands for the File Explorer.
///
/// Security model — ALL operations are jailed to `base_dir`:
///   - **Reads** (`list_directory`, `get_home_dir`): Only list contents inside `base_dir`.
///   - **Writes** (`create_folder`, `delete_entry`, `rename_entry`): Jailed to `base_dir`.
///   - **Open** (`open_file`): Only opens files inside `base_dir`.
///
/// Every path is validated through the core `Sandbox::resolve()` which enforces:
///   • Path canonicalization (blocks symlink escapes)
///   • Traversal detection (`../../` blocked)
///   • Blocked filenames (`.ssh`, `.gnupg`, `.bash_history`, `.env`, `.git`)
///   • Absolute path escapes (must stay inside sandbox root)
///
/// This follows the same pattern as `AppManager::create_sandbox()` in `core/`.
use crate::state::AppState;
use ai_launcher_core::sandbox::Sandbox;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::State;

#[derive(Serialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<String>,
}

// ── Sandbox helpers ─────────────────────────────────────────────────────

/// Create a `Sandbox` rooted at `base_dir`, following the same pattern
/// as `AppManager::create_sandbox()` in `core/app_manager/mod.rs:57-72`.
fn create_explorer_sandbox(base_dir: &Path) -> Result<Sandbox, String> {
    let sandbox = Sandbox::new(base_dir).map_err(|e| format!("Sandbox init: {e}"))?;
    // Create standard workspace subdirectories (data, models, outputs, logs, tmp, config)
    // just like AppManager::create_sandbox() does in core/app_manager/mod.rs:71
    sandbox
        .init_workspace()
        .map_err(|e| format!("Workspace init: {e}"))?;
    Ok(sandbox)
}

/// Validate any path through the sandbox jail.
/// Returns the resolved (canonicalized, jail-verified) path.
fn sandbox_resolve(sandbox: &Sandbox, raw: &str) -> Result<PathBuf, String> {
    let p = Path::new(raw);
    sandbox
        .resolve(p)
        .map_err(|e| format!("Access denied — {e}"))
}

// ── Commands ────────────────────────────────────────────────────────────

/// List directory contents. Path must be inside `base_dir`.
/// If `path` is empty, lists the sandbox root itself.
#[tauri::command]
pub async fn list_directory(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let sandbox = create_explorer_sandbox(&base_dir)?;

        let dir = if path.is_empty() {
            sandbox.root().to_path_buf()
        } else {
            sandbox_resolve(&sandbox, &path)?
        };

        let read = std::fs::read_dir(&dir)
            .map_err(|e| format!("Cannot read {}: {e}", dir.display()))?;

        let mut entries: Vec<FileEntry> = Vec::new();
        for item in read.flatten() {
            let meta = match item.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| {
                    chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_default()
                });

            entries.push(FileEntry {
                name: item.file_name().to_string_lossy().to_string(),
                path: item.path().to_string_lossy().to_string(),
                is_dir: meta.is_dir(),
                size: meta.len(),
                modified,
            });
        }

        // Directories first, then case-insensitive alphabetical
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(entries)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Returns the sandbox root (base_dir) as the "home" directory.
#[tauri::command]
pub async fn get_home_dir(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.base_dir.to_string_lossy().to_string())
}

/// Open a file with the OS default application. File must be inside `base_dir`.
#[tauri::command]
pub async fn open_file(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let sandbox = create_explorer_sandbox(&base_dir)?;
        let resolved = sandbox_resolve(&sandbox, &path)?;
        open::that(&resolved).map_err(|e| format!("Cannot open {}: {e}", resolved.display()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Create a folder. Jailed to `base_dir` via core `Sandbox::resolve()`.
#[tauri::command]
pub async fn create_folder(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let sandbox = create_explorer_sandbox(&base_dir)?;
        let resolved = sandbox_resolve(&sandbox, &path)?;
        std::fs::create_dir(&resolved)
            .map_err(|e| format!("Cannot create folder {}: {e}", resolved.display()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Delete a file or directory. Jailed to `base_dir` via core `Sandbox::resolve()`.
#[tauri::command]
pub async fn delete_entry(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let sandbox = create_explorer_sandbox(&base_dir)?;
        let resolved = sandbox_resolve(&sandbox, &path)?;

        let meta = std::fs::metadata(&resolved)
            .map_err(|e| format!("Cannot stat {}: {e}", resolved.display()))?;
        if meta.is_dir() {
            std::fs::remove_dir_all(&resolved)
                .map_err(|e| format!("Cannot delete dir {}: {e}", resolved.display()))
        } else {
            std::fs::remove_file(&resolved)
                .map_err(|e| format!("Cannot delete file {}: {e}", resolved.display()))
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Rename a file or directory. Both old and new paths are jailed to `base_dir`.
#[tauri::command]
pub async fn rename_entry(
    state: State<'_, AppState>,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let sandbox = create_explorer_sandbox(&base_dir)?;
        let old = sandbox_resolve(&sandbox, &old_path)?;
        let new = sandbox_resolve(&sandbox, &new_path)?;
        std::fs::rename(&old, &new)
            .map_err(|e| format!("Cannot rename {} → {}: {e}", old.display(), new.display()))
    })
    .await
    .map_err(|e| e.to_string())?
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ai_launcher_core::sandbox::Sandbox;
    use std::fs;

    fn temp_dir() -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let p = std::env::temp_dir().join(format!("nde-fs-test-{stamp}"));
        fs::create_dir_all(&p).expect("create temp dir");
        p
    }

    // ── Core sandbox contract tests ─────────────────────────────────

    #[test]
    fn sandbox_blocks_sensitive_filenames() {
        let dir = temp_dir();
        let sb = Sandbox::new(&dir).unwrap();
        assert!(sb.resolve(Path::new(".ssh")).is_err());
        assert!(sb.resolve(Path::new(".env")).is_err());
        assert!(sb.resolve(Path::new(".git")).is_err());
        assert!(sb.resolve(Path::new(".gnupg")).is_err());
        assert!(sb.resolve(Path::new(".bash_history")).is_err());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn sandbox_blocks_traversal() {
        let dir = temp_dir();
        let sb = Sandbox::new(&dir).unwrap();
        assert!(sb.resolve(Path::new("../../etc/passwd")).is_err());
        assert!(sb.resolve(Path::new("..\\..\\Windows")).is_err());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn sandbox_blocks_absolute_escape() {
        let dir = temp_dir();
        let sb = Sandbox::new(&dir).unwrap();
        let escape = if cfg!(windows) {
            "C:\\Windows\\System32"
        } else {
            "/etc/passwd"
        };
        assert!(sb.resolve(Path::new(escape)).is_err());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn sandbox_allows_valid_child() {
        let dir = temp_dir();
        let sb = Sandbox::new(&dir).unwrap();
        fs::write(dir.join("hello.txt"), b"world").unwrap();
        assert!(sb.resolve(Path::new("hello.txt")).is_ok());
        fs::remove_dir_all(dir).ok();
    }

    // ── Jailed read/write validation ────────────────────────────────

    #[test]
    fn read_denied_outside_jail() {
        let dir = temp_dir();
        let sb = create_explorer_sandbox(&dir).unwrap();
        let outside = std::env::temp_dir();
        let result = sandbox_resolve(&sb, &outside.to_string_lossy());
        assert!(result.is_err(), "read outside jail must fail");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn write_denied_outside_jail() {
        let dir = temp_dir();
        let sb = create_explorer_sandbox(&dir).unwrap();
        let outside = std::env::temp_dir().join("outside.txt");
        let result = sandbox_resolve(&sb, &outside.to_string_lossy());
        assert!(result.is_err(), "write outside jail must fail");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn access_allowed_inside_jail() {
        let dir = temp_dir();
        let sb = create_explorer_sandbox(&dir).unwrap();
        let inside = dir.join("allowed-folder");
        let result = sandbox_resolve(&sb, &inside.to_string_lossy());
        assert!(result.is_ok(), "access inside jail should succeed");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn blocked_filenames_inside_jail() {
        let dir = temp_dir();
        let sb = create_explorer_sandbox(&dir).unwrap();
        let blocked = dir.join(".ssh");
        let result = sandbox_resolve(&sb, &blocked.to_string_lossy());
        assert!(
            result.is_err(),
            ".ssh inside jail should still be blocked by core sandbox"
        );
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn list_sandbox_root_works() {
        let dir = temp_dir();
        fs::write(dir.join("test.txt"), b"hello").unwrap();
        fs::create_dir(dir.join("subdir")).unwrap();

        let sb = create_explorer_sandbox(&dir).unwrap();
        let read = std::fs::read_dir(sb.root()).unwrap();
        let count: usize = read.count();
        assert!(count >= 2, "should see files in sandbox root");
        fs::remove_dir_all(dir).ok();
    }
}
