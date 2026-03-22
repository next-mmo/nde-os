use crate::state::AppState;
use ai_launcher_core::manifest::AppManifest;
use tauri::State;

#[tauri::command]
pub fn get_catalog(state: State<'_, AppState>) -> Result<Vec<AppManifest>, String> {
    let mgr = state.manager.lock().map_err(|e| e.to_string())?;
    Ok(mgr.catalog())
}
