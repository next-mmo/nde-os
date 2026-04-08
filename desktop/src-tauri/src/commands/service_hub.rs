//! Tauri IPC commands for the global Service Hub.
//!
//! Provides service status detection and installation for all NDE-OS services.
//! Any app can check service status and redirect users to the Service Hub
//! for onboarding.

use crate::state::AppState;
use ai_launcher_core::services::{config as svc_config, registry, types::ServiceStatus};
use ai_launcher_core::voice::runtime::VoiceRuntime;
use ai_launcher_core::voice::types::VoiceRuntimeStatus;
use std::collections::HashMap;

/// Managed state for the voice runtime (shared across all apps).
pub struct VoiceRuntimeState {
    pub runtime: VoiceRuntime,
}

// Re-export VikingState so lib.rs can use it from a single module
pub use crate::commands::viking::VikingState;

// ─── Service Hub Commands ──────────────────────────────────────────────────────

/// Get the status of all registered NDE-OS services.
#[tauri::command]
pub async fn service_hub_status(
    app_state: tauri::State<'_, AppState>,
) -> Result<Vec<ServiceStatus>, String> {
    let base_dir = app_state.base_dir.clone();
    tokio::task::spawn_blocking(move || registry::detect_all(&base_dir))
        .await
        .map_err(|e| e.to_string())
}

/// Install a specific service by ID.
#[tauri::command]
pub async fn service_hub_install(
    app_state: tauri::State<'_, AppState>,
    service_id: String,
) -> Result<String, String> {
    let base_dir = app_state.base_dir.clone();
    tokio::task::spawn_blocking(move || registry::install_service(&service_id, &base_dir))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

// ─── Voice Runtime Commands (global, not FreeCut-specific) ─────────────────────

/// Get the global voice runtime status.
#[tauri::command]
pub async fn voice_runtime_status(
    voice_state: tauri::State<'_, VoiceRuntimeState>,
) -> Result<VoiceRuntimeStatus, String> {
    let runtime = voice_state.runtime.clone();
    tokio::task::spawn_blocking(move || runtime.detect_status())
        .await
        .map_err(|e| e.to_string())
}

/// Install voice runtime components globally.
#[tauri::command]
pub async fn voice_runtime_install(
    voice_state: tauri::State<'_, VoiceRuntimeState>,
    components: Vec<String>,
) -> Result<ai_launcher_core::voice::types::VoiceInstallResult, String> {
    let runtime = voice_state.runtime.clone();
    tokio::task::spawn_blocking(move || runtime.install_components(&components))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

// ─── Service Config Commands ─────────────────────────────────────────────────

/// Get the config schema + current values for a service.
#[tauri::command]
pub async fn service_hub_get_config(
    app_state: tauri::State<'_, AppState>,
    service_id: String,
) -> Result<svc_config::ServiceConfig, String> {
    let base_dir = app_state.base_dir.clone();
    tokio::task::spawn_blocking(move || svc_config::get_service_config(&service_id, &base_dir))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Save config values for a service.
#[tauri::command]
pub async fn service_hub_set_config(
    app_state: tauri::State<'_, AppState>,
    service_id: String,
    values: HashMap<String, serde_json::Value>,
) -> Result<String, String> {
    let base_dir = app_state.base_dir.clone();
    tokio::task::spawn_blocking(move || {
        svc_config::set_service_config(&service_id, values, &base_dir)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    Ok("Config saved".to_string())
}
