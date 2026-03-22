use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct LaunchResult {
    pub pid: u32,
    pub port: u16,
}

#[tauri::command]
pub fn launch_app(state: State<'_, AppState>, app_id: String) -> Result<LaunchResult, String> {
    let mgr = state.manager.lock().map_err(|e| e.to_string())?;
    let (pid, port) = mgr.launch(&app_id).map_err(|e| e.to_string())?;
    Ok(LaunchResult { pid, port })
}

#[tauri::command]
pub fn stop_app(state: State<'_, AppState>, app_id: String) -> Result<String, String> {
    let mgr = state.manager.lock().map_err(|e| e.to_string())?;
    mgr.stop(&app_id).map_err(|e| e.to_string())?;
    Ok(format!("'{}' stopped", app_id))
}

#[tauri::command]
pub async fn open_app_browser(port: u16) -> Result<String, String> {
    let url = format!("http://localhost:{}", port);
    open::that(&url).map_err(|e| e.to_string())?;
    Ok(url)
}
