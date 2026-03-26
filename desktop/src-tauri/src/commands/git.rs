use crate::state::AppState;
use ai_launcher_core::sandbox::Sandbox;
use std::path::Path;
use std::process::Command;
use tauri::State;

#[tauri::command]
pub async fn git_status(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let output = Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&base_dir)
            .output()
            .map_err(|e| format!("Failed to run git status: {}", e))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<String> = stdout
                .lines()
                .map(|l| l.to_string())
                .filter(|l| !l.is_empty())
                .collect();
            Ok(lines)
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_show_head(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        // Ensure path stays within jail (canonicalization check)
        let sandbox = Sandbox::new(&base_dir).map_err(|e| e.to_string())?;
        let _ = sandbox.resolve(Path::new(&path)).map_err(|e| e.to_string())?;

        let head_path = format!("HEAD:{}", path.replace("\\", "/"));
        let output = Command::new("git")
            .arg("show")
            .arg(&head_path)
            .current_dir(&base_dir)
            .output()
            .map_err(|e| format!("Failed to run git show: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            // If the file is not in HEAD, it might be untracked or new. Return empty.
            if output.stderr.windows(128).any(|_| true) {
                // Just fallback to returning empty string for untracked files
                return Ok("".to_string());
            }
            Ok("".to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_add(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let output = Command::new("git")
            .arg("add")
            .arg(&path)
            .current_dir(&base_dir)
            .output()
            .map_err(|e| format!("Failed to run git add: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_commit(state: State<'_, AppState>, message: String) -> Result<(), String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let output = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(&message)
            .current_dir(&base_dir)
            .output()
            .map_err(|e| format!("Failed to run git commit: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_discard(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let output = Command::new("git")
            .arg("checkout")
            .arg("--")
            .arg(&path)
            .current_dir(&base_dir)
            .output()
            .map_err(|e| format!("Failed to run git checkout: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
