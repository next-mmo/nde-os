use crate::state::AppState;
use ai_launcher_core::manifest::{AppManifest, InstalledApp};
use tauri::State;

#[tauri::command]
pub fn list_apps(state: State<'_, AppState>) -> Result<Vec<InstalledApp>, String> {
    let mgr = state.manager.lock().map_err(|e| e.to_string())?;
    Ok(mgr.list_apps())
}

#[tauri::command]
pub fn get_app(state: State<'_, AppState>, app_id: String) -> Result<Option<InstalledApp>, String> {
    let mgr = state.manager.lock().map_err(|e| e.to_string())?;
    Ok(mgr.get_app(&app_id))
}

#[tauri::command]
pub fn install_app(state: State<'_, AppState>, manifest: AppManifest) -> Result<InstalledApp, String> {
    let mgr = state.manager.lock().map_err(|e| e.to_string())?;
    mgr.install(&manifest).map_err(|e| e.to_string())?;
    mgr.get_app(&manifest.id)
        .ok_or_else(|| "App installed but not found in registry".to_string())
}

#[tauri::command]
pub fn uninstall_app(state: State<'_, AppState>, app_id: String) -> Result<String, String> {
    let mgr = state.manager.lock().map_err(|e| e.to_string())?;
    mgr.uninstall(&app_id).map_err(|e| e.to_string())?;
    Ok(format!("'{}' uninstalled", app_id))
}
