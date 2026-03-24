use crate::state::AppState;
use ai_launcher_core::sandbox::Sandbox;
use serde::Serialize;
use std::path::PathBuf;
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

/// Build a Sandbox rooted at `base_dir` with readonly access to the user home.
/// All mutating operations (create/delete/rename) are jailed to `base_dir`.
/// Read-only browsing is allowed under $HOME via `allow_readonly`.
fn build_sandbox(base_dir: &std::path::Path) -> Result<Sandbox, String> {
    let mut sb = Sandbox::new(base_dir).map_err(|e| format!("Sandbox init error: {e}"))?;

    // Allow read-only browsing of the user home directory
    if let Some(home) = home_dir_path() {
        sb.allow_readonly(&home);
    }

    Ok(sb)
}

/// Validate a path through the core Sandbox jail.
/// Returns the resolved (canonicalized, jail-verified) path.
fn sandbox_resolve(sandbox: &Sandbox, raw: &str) -> Result<PathBuf, String> {
    let p = std::path::Path::new(raw);
    sandbox
        .resolve(p)
        .map_err(|e| format!("Sandbox blocked: {e}"))
}

#[tauri::command]
pub async fn list_directory(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    let base_dir = state.base_dir.clone();

    let dir = if path.is_empty() {
        home_dir_path().ok_or_else(|| "Cannot determine home directory".to_string())?
    } else {
        PathBuf::from(&path)
    };

    tauri::async_runtime::spawn_blocking(move || {
        // Validate through sandbox (read is allowed for home + base_dir)
        let sandbox = build_sandbox(&base_dir)?;
        let resolved = sandbox_resolve(&sandbox, &dir.to_string_lossy())?;

        let read = std::fs::read_dir(&resolved)
            .map_err(|e| format!("Cannot read {}: {}", resolved.display(), e))?;

        let mut entries: Vec<FileEntry> = Vec::new();
        for item in read {
            let item = match item {
                Ok(i) => i,
                Err(_) => continue,
            };
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

        // Directories first, then alphabetical
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

fn home_dir_path() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    } else {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

#[tauri::command]
pub async fn get_home_dir() -> Result<String, String> {
    home_dir_path()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Cannot determine home directory".to_string())
}

#[tauri::command]
pub async fn open_file(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let sandbox = build_sandbox(&base_dir)?;
        let resolved = sandbox_resolve(&sandbox, &path)?;
        open::that(&resolved).map_err(|e| format!("Cannot open {}: {}", resolved.display(), e))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Mutating: jailed to base_dir ONLY (sandbox root).
#[tauri::command]
pub async fn create_folder(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let sandbox = build_sandbox(&base_dir)?;
        // For mutating ops, the path MUST resolve inside sandbox root (not readonly)
        let resolved = sandbox_resolve(&sandbox, &path)?;
        if !resolved.starts_with(&base_dir) {
            return Err(format!(
                "Write denied: path is outside sandbox root ({})",
                base_dir.display()
            ));
        }
        std::fs::create_dir(&resolved)
            .map_err(|e| format!("Cannot create folder {}: {}", resolved.display(), e))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Mutating: jailed to base_dir ONLY.
#[tauri::command]
pub async fn delete_entry(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let sandbox = build_sandbox(&base_dir)?;
        let resolved = sandbox_resolve(&sandbox, &path)?;
        if !resolved.starts_with(&base_dir) {
            return Err(format!(
                "Delete denied: path is outside sandbox root ({})",
                base_dir.display()
            ));
        }
        let meta = std::fs::metadata(&resolved)
            .map_err(|e| format!("Cannot stat {}: {}", resolved.display(), e))?;
        if meta.is_dir() {
            std::fs::remove_dir_all(&resolved)
                .map_err(|e| format!("Cannot delete directory {}: {}", resolved.display(), e))
        } else {
            std::fs::remove_file(&resolved)
                .map_err(|e| format!("Cannot delete file {}: {}", resolved.display(), e))
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Mutating: jailed to base_dir ONLY.
#[tauri::command]
pub async fn rename_entry(
    state: State<'_, AppState>,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let sandbox = build_sandbox(&base_dir)?;
        let old = sandbox_resolve(&sandbox, &old_path)?;
        let new = sandbox_resolve(&sandbox, &new_path)?;
        if !old.starts_with(&base_dir) || !new.starts_with(&base_dir) {
            return Err(format!(
                "Rename denied: paths must be inside sandbox root ({})",
                base_dir.display()
            ));
        }
        std::fs::rename(&old, &new).map_err(|e| {
            format!(
                "Cannot rename {} to {}: {}",
                old.display(),
                new.display(),
                e
            )
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let p = std::env::temp_dir().join(format!("ai-launcher-fs-test-{stamp}"));
        fs::create_dir_all(&p).expect("create temp dir");
        p
    }

    #[test]
    fn sandbox_blocks_sensitive_filenames() {
        let dir = temp_dir();
        let sb = Sandbox::new(&dir).unwrap();
        // .ssh, .gnupg, .env, .git are blocked by the core sandbox
        assert!(sb.resolve(std::path::Path::new(".ssh")).is_err());
        assert!(sb.resolve(std::path::Path::new(".env")).is_err());
        assert!(sb.resolve(std::path::Path::new(".git")).is_err());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn sandbox_blocks_traversal() {
        let dir = temp_dir();
        let sb = Sandbox::new(&dir).unwrap();
        assert!(sb.resolve(std::path::Path::new("../../etc/passwd")).is_err());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn sandbox_allows_valid_child() {
        let dir = temp_dir();
        let sb = Sandbox::new(&dir).unwrap();
        fs::write(dir.join("hello.txt"), b"world").unwrap();
        assert!(sb.resolve(std::path::Path::new("hello.txt")).is_ok());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn get_home_dir_returns_path() {
        let result = tauri::async_runtime::block_on(get_home_dir()).expect("should get home");
        assert!(!result.is_empty());
    }
}
