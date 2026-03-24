use ai_launcher_core::shield::browser::BrowserEngine;
use ai_launcher_core::shield::profile::{ProfileManager, ShieldProfile};
use ai_launcher_core::shield::engine::EngineManager;
use ai_launcher_core::shield::launcher::{self, BrowserLauncher};
use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

// ─── Response Types ────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ShieldProfileResponse {
    pub id: String,
    pub name: String,
    pub engine: String,
    pub engine_version: String,
    pub is_running: bool,
    pub last_launch: Option<u64>,
    pub created_at: u64,
    pub tags: Vec<String>,
    pub note: Option<String>,
    pub has_proxy: bool,
    pub fingerprint_os: Option<String>,
}

impl From<ShieldProfile> for ShieldProfileResponse {
    fn from(p: ShieldProfile) -> Self {
        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            engine: p.engine.as_str().to_string(),
            engine_version: p.engine_version.clone(),
            is_running: p.is_running(),
            last_launch: p.last_launch,
            created_at: p.created_at,
            tags: p.tags.clone(),
            note: p.note.clone(),
            has_proxy: p.proxy.is_some(),
            fingerprint_os: p.fingerprint.os.clone(),
        }
    }
}

// ─── Profile Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn list_shield_profiles(state: tauri::State<'_, AppState>) -> Result<Vec<ShieldProfileResponse>, String> {
    let mgr = ProfileManager::new(&state.base_dir);
    let profiles = mgr.list_profiles().map_err(|e| e.to_string())?;
    Ok(profiles.into_iter().map(ShieldProfileResponse::from).collect())
}

#[tauri::command]
pub fn get_shield_profile(state: tauri::State<'_, AppState>, id: String) -> Result<ShieldProfileResponse, String> {
    let mgr = ProfileManager::new(&state.base_dir);
    let profile = mgr.get_profile(&id).map_err(|e| e.to_string())?;
    Ok(ShieldProfileResponse::from(profile))
}

#[tauri::command]
pub fn create_shield_profile(
    state: tauri::State<'_, AppState>,
    name: String,
    engine: String,
    engine_version: String,
) -> Result<ShieldProfileResponse, String> {
    let engine_type = BrowserEngine::from_str(&engine).map_err(|e| e.to_string())?;
    let profile = ShieldProfile::new(name, engine_type, engine_version);
    let mgr = ProfileManager::new(&state.base_dir);
    mgr.create_profile(&profile).map_err(|e| e.to_string())?;
    Ok(ShieldProfileResponse::from(profile))
}

#[tauri::command]
pub fn delete_shield_profile(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let mgr = ProfileManager::new(&state.base_dir);
    mgr.delete_profile(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_shield_profile(
    state: tauri::State<'_, AppState>,
    id: String,
    new_name: String,
) -> Result<ShieldProfileResponse, String> {
    let mgr = ProfileManager::new(&state.base_dir);
    let profile = mgr.rename_profile(&id, &new_name).map_err(|e| e.to_string())?;
    Ok(ShieldProfileResponse::from(profile))
}

#[tauri::command]
pub fn get_shield_status(state: tauri::State<'_, AppState>) -> Result<ShieldStatusResponse, String> {
    let profile_mgr = ProfileManager::new(&state.base_dir);
    let engine_mgr = EngineManager::new(&state.base_dir);

    let profiles = profile_mgr.list_profiles().map_err(|e| e.to_string())?;
    let engines = engine_mgr.list_downloaded().map_err(|e| e.to_string())?;

    let running_count = profiles.iter().filter(|p| p.is_running()).count();

    Ok(ShieldStatusResponse {
        total_profiles: profiles.len(),
        running_profiles: running_count,
        installed_engines: engines
            .into_iter()
            .map(|(e, v)| InstalledEngine {
                engine: e.as_str().to_string(),
                version: v,
            })
            .collect(),
    })
}

#[derive(Serialize)]
pub struct ShieldStatusResponse {
    pub total_profiles: usize,
    pub running_profiles: usize,
    pub installed_engines: Vec<InstalledEngine>,
}

#[derive(Serialize)]
pub struct InstalledEngine {
    pub engine: String,
    pub version: String,
}

// ─── Launch / Stop Commands ────────────────────────────────────────

#[tauri::command]
pub async fn launch_shield_profile(
    launcher_state: tauri::State<'_, ShieldLauncherState>,
    id: String,
    url: Option<String>,
) -> Result<u16, String> {
    let launcher = launcher_state.launcher.lock().await;
    let port = launcher
        .launch_profile(&id, url.as_deref())
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;
    Ok(port)
}

#[tauri::command]
pub async fn stop_shield_profile(
    launcher_state: tauri::State<'_, ShieldLauncherState>,
    id: String,
) -> Result<(), String> {
    let launcher = launcher_state.launcher.lock().await;
    launcher
        .stop_profile(&id)
        .await
        .map_err(|e: anyhow::Error| e.to_string())
}

// ─── Engine Download Command ───────────────────────────────────────

#[tauri::command]
pub async fn download_shield_engine(
    state: tauri::State<'_, AppState>,
    engine: String,
    version: String,
) -> Result<String, String> {
    let engine_type = BrowserEngine::from_str(&engine).map_err(|e: anyhow::Error| e.to_string())?;
    let download_url = launcher::get_download_url(&engine_type, &version)
        .map_err(|e: anyhow::Error| e.to_string())?;

    let engine_mgr = EngineManager::new(&state.base_dir);
    let install_dir = engine_mgr
        .download_engine(&engine_type, &version, &download_url)
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    Ok(install_dir.display().to_string())
}

#[tauri::command]
pub fn is_shield_engine_downloaded(
    state: tauri::State<'_, AppState>,
    engine: String,
    version: String,
) -> Result<bool, String> {
    let engine_type = BrowserEngine::from_str(&engine).map_err(|e: anyhow::Error| e.to_string())?;
    let engine_mgr = EngineManager::new(&state.base_dir);
    Ok(engine_mgr.is_downloaded(&engine_type, &version))
}

// ─── Onboarding Commands ───────────────────────────────────────────

#[tauri::command]
pub async fn resolve_engine_version(engine: String) -> Result<String, String> {
    let engine_type = BrowserEngine::from_str(&engine).map_err(|e: anyhow::Error| e.to_string())?;
    launcher::resolve_latest_version(&engine_type)
        .await
        .map_err(|e: anyhow::Error| e.to_string())
}

#[tauri::command]
pub fn get_available_engines() -> Vec<launcher::AvailableEngine> {
    launcher::get_available_engines()
}

// ─── Managed State ─────────────────────────────────────────────────

/// Tauri-managed state for the browser launcher (async-safe).
pub struct ShieldLauncherState {
    pub launcher: Arc<Mutex<BrowserLauncher>>,
}
