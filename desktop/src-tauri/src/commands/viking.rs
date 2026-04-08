use ai_launcher_core::openviking::{VikingClient, VikingProcess};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared Viking process state managed by Tauri.
pub struct VikingState {
    pub process: Arc<Mutex<VikingProcess>>,
}

#[derive(Debug, Serialize)]
pub struct VikingStatus {
    pub connected: bool,
    pub process_managed: bool,
    pub port: u16,
    pub status: Option<serde_json::Value>,
    pub message: Option<String>,
}

#[tauri::command]
pub async fn viking_status(state: tauri::State<'_, VikingState>) -> Result<VikingStatus, String> {
    let port = {
        let v = state.process.lock().await;
        v.config().port
    };

    let client = VikingClient::new(&format!("http://localhost:{}", port));
    let healthy = client.health().await.unwrap_or(false);

    if healthy {
        let status = client.status().await.ok();
        let process_managed = state.process.lock().await.is_running();
        Ok(VikingStatus {
            connected: true,
            process_managed,
            port,
            status,
            message: None,
        })
    } else {
        let process_managed = state.process.lock().await.is_running();
        Ok(VikingStatus {
            connected: false,
            process_managed,
            port,
            status: None,
            message: Some("OpenViking server not running".into()),
        })
    }
}

#[tauri::command]
pub async fn viking_install(state: tauri::State<'_, VikingState>) -> Result<bool, String> {
    let v = state.process.lock().await;
    v.ensure_installed()
        .await
        .map_err(|e| format!("Installation error: {}", e))
}

#[tauri::command]
pub async fn viking_start(state: tauri::State<'_, VikingState>) -> Result<VikingStatus, String> {
    let mut v = state.process.lock().await;
    v.start()
        .await
        .map_err(|e| format!("Start failed: {}", e))?;
    let port = v.config().port;
    drop(v);

    let client = VikingClient::new(&format!("http://localhost:{}", port));
    let connected = client.health().await.unwrap_or(false);

    Ok(VikingStatus {
        connected,
        process_managed: true,
        port,
        status: None,
        message: None,
    })
}

#[tauri::command]
pub async fn viking_stop(state: tauri::State<'_, VikingState>) -> Result<VikingStatus, String> {
    let mut v = state.process.lock().await;
    v.stop().await;
    let port = v.config().port;

    Ok(VikingStatus {
        connected: false,
        process_managed: false,
        port,
        status: None,
        message: Some("OpenViking server stopped".into()),
    })
}
