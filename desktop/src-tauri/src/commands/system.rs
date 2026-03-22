use crate::state::AppState;
use serde_json::{json, Value};
use std::process::Command;
use tauri::State;

use ai_launcher_core::manifest::AppManifest;

#[tauri::command]
pub fn health_check() -> Result<String, String> {
    Ok("AI Launcher is running".to_string())
}

#[tauri::command]
pub fn get_system_info(state: State<'_, AppState>) -> Result<Value, String> {
    let mgr = state.manager.lock().map_err(|e| e.to_string())?;

    let py_cmd = AppManifest::python_cmd();
    let py = Command::new(py_cmd)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    let gpu = Command::new("nvidia-smi")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    Ok(json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "python_version": py,
        "gpu_detected": gpu,
        "uv": mgr.uv_info(),
        "base_dir": mgr.base_dir().to_string_lossy(),
        "total_apps": mgr.total_count(),
        "running_apps": mgr.running_count(),
    }))
}
