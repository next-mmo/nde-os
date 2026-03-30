use crate::state::{with_manager, AppState};
use ai_launcher_core::sandbox::SandboxVerifyResult;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct DiskUsage {
    pub app_id: String,
    pub bytes: u64,
    pub human_readable: String,
}

#[tauri::command]
pub async fn verify_sandbox(
    state: State<'_, AppState>,
    app_id: String,
) -> Result<SandboxVerifyResult, String> {
    with_manager(state.manager.clone(), move |mgr| {
        mgr.verify_sandbox(&app_id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn get_disk_usage(
    state: State<'_, AppState>,
    app_id: String,
) -> Result<DiskUsage, String> {
    with_manager(state.manager.clone(), move |mgr| {
        let bytes = mgr.disk_usage(&app_id).map_err(|e| e.to_string())?;
        let human_readable = if bytes > 1_073_741_824 {
            format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
        } else if bytes > 1_048_576 {
            format!("{:.2} MB", bytes as f64 / 1_048_576.0)
        } else {
            format!("{:.2} KB", bytes as f64 / 1024.0)
        };
        Ok(DiskUsage {
            app_id,
            bytes,
            human_readable,
        })
    })
    .await
}
