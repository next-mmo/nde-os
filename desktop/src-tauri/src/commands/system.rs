use crate::state::AppState;
use ai_launcher_core::system_metrics::{snapshot_resource_usage, ResourceUsage};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use tauri::State;

use ai_launcher_core::manifest::AppManifest;

#[derive(Serialize)]
pub struct SystemInfoPayload {
    os: &'static str,
    arch: &'static str,
    python_version: Option<String>,
    gpu_detected: bool,
    uv: Value,
    base_dir: String,
    total_apps: usize,
    running_apps: usize,
}

#[tauri::command]
pub fn health_check() -> Result<String, String> {
    Ok("AI Launcher is running".to_string())
}

fn resource_usage_for(base_dir: PathBuf) -> Result<ResourceUsage, String> {
    snapshot_resource_usage(&base_dir).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_system_info(state: State<'_, AppState>) -> Result<SystemInfoPayload, String> {
    let manager = state.manager.clone();
    let py_cmd = AppManifest::python_cmd().to_string();

    tauri::async_runtime::spawn_blocking(move || -> Result<SystemInfoPayload, String> {
        let py = Command::new(&py_cmd)
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        let gpu = Command::new("nvidia-smi")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let mgr = manager.lock().map_err(|e| e.to_string())?;

        Ok(SystemInfoPayload {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            python_version: py,
            gpu_detected: gpu,
            uv: json!(mgr.uv_info()),
            base_dir: mgr.base_dir().to_string_lossy().to_string(),
            total_apps: mgr.total_count(),
            running_apps: mgr.running_count(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_resource_usage(state: State<'_, AppState>) -> Result<ResourceUsage, String> {
    let base_dir = state.base_dir.clone();

    tauri::async_runtime::spawn_blocking(move || resource_usage_for(base_dir))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::resource_usage_for;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_base() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ai-launcher-desktop-system-{stamp}"));
        fs::create_dir_all(&path).expect("temp test dir should be created");
        path
    }

    #[test]
    fn resource_usage_for_reports_percentages() {
        let base_dir = temp_base();

        let usage = resource_usage_for(base_dir.clone()).expect("resource usage should load");

        assert!(usage.memory_total_bytes > 0);
        assert!(usage.disk_total_bytes > 0);
        assert!(usage.memory_percent <= 100);
        assert!(usage.disk_percent <= 100);

        fs::remove_dir_all(base_dir).ok();
    }
}
