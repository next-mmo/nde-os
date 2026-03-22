use crate::state::{with_manager, AppState, SharedAppManager};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct LaunchResult {
    pub pid: u32,
    pub port: u16,
}

async fn launch_app_with_manager(manager: SharedAppManager, app_id: String) -> Result<LaunchResult, String> {
    with_manager(manager, move |mgr| {
        let (pid, port) = mgr.launch(&app_id).map_err(|e| e.to_string())?;
        Ok(LaunchResult { pid, port })
    })
    .await
}

async fn stop_app_with_manager(manager: SharedAppManager, app_id: String) -> Result<String, String> {
    with_manager(manager, move |mgr| {
        mgr.stop(&app_id).map_err(|e| e.to_string())?;
        Ok(format!("'{}' stopped", app_id))
    })
    .await
}

#[tauri::command]
pub async fn launch_app(state: State<'_, AppState>, app_id: String) -> Result<LaunchResult, String> {
    launch_app_with_manager(state.manager.clone(), app_id).await
}

#[tauri::command]
pub async fn stop_app(state: State<'_, AppState>, app_id: String) -> Result<String, String> {
    stop_app_with_manager(state.manager.clone(), app_id).await
}

#[tauri::command]
pub async fn open_app_browser(port: u16) -> Result<String, String> {
    let url = format!("http://localhost:{}", port);
    open::that(&url).map_err(|e| e.to_string())?;
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::{launch_app_with_manager, stop_app_with_manager};
    use crate::state::SharedAppManager;
    use ai_launcher_core::app_manager::AppManager;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_base() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ai-launcher-desktop-lifecycle-{stamp}"));
        fs::create_dir_all(&path).expect("temp test dir should be created");
        path
    }

    fn manager() -> (PathBuf, SharedAppManager) {
        let base_dir = temp_base();
        let manager = Arc::new(Mutex::new(
            AppManager::new(&base_dir).expect("manager should be created"),
        ));
        (base_dir, manager)
    }

    #[test]
    fn launch_app_reports_missing_install() {
        let (base_dir, manager) = manager();
        let result = tauri::async_runtime::block_on(launch_app_with_manager(
            manager,
            "missing-app".to_string(),
        ));

        let error = result.expect_err("launch should fail for missing app");
        assert!(error.contains("not installed"));
        fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn stop_app_reports_missing_process() {
        let (base_dir, manager) = manager();
        let result = tauri::async_runtime::block_on(stop_app_with_manager(
            manager,
            "missing-app".to_string(),
        ));

        let error = result.expect_err("stop should fail for missing app");
        assert!(error.contains("not running"));
        fs::remove_dir_all(base_dir).ok();
    }
}
