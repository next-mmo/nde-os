use crate::state::{with_manager, AppState};
use ai_launcher_core::manifest::{AppManifest, InstalledApp, StoreUploadRequest, StoreUploadResult};
use tauri::State;

#[tauri::command]
pub async fn list_apps(state: State<'_, AppState>) -> Result<Vec<InstalledApp>, String> {
    with_manager(state.manager.clone(), |mgr| Ok(mgr.list_apps())).await
}

#[tauri::command]
pub async fn get_app(state: State<'_, AppState>, app_id: String) -> Result<Option<InstalledApp>, String> {
    with_manager(state.manager.clone(), move |mgr| Ok(mgr.get_app(&app_id))).await
}

#[tauri::command]
pub async fn install_app(state: State<'_, AppState>, manifest: AppManifest) -> Result<InstalledApp, String> {
    with_manager(state.manager.clone(), move |mgr| {
        mgr.install(&manifest).map_err(|e| e.to_string())?;
        mgr.get_app(&manifest.id)
            .ok_or_else(|| "App installed but not found in registry".to_string())
    })
    .await
}

#[tauri::command]
pub async fn uninstall_app(state: State<'_, AppState>, app_id: String) -> Result<String, String> {
    with_manager(state.manager.clone(), move |mgr| {
        mgr.uninstall(&app_id).map_err(|e| e.to_string())?;
        Ok(format!("'{}' uninstalled", app_id))
    })
    .await
}

#[tauri::command]
pub async fn upload_app(state: State<'_, AppState>, req: StoreUploadRequest) -> Result<StoreUploadResult, String> {
    with_manager(state.manager.clone(), move |mgr| {
        mgr.upload_to_store(&req).map_err(|e| e.to_string())
    })
    .await
}
