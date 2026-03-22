use crate::state::{with_manager, AppState};
use ai_launcher_core::manifest::AppManifest;
use tauri::State;

#[tauri::command]
pub async fn get_catalog(state: State<'_, AppState>) -> Result<Vec<AppManifest>, String> {
    with_manager(state.manager.clone(), |mgr| Ok(mgr.catalog())).await
}
