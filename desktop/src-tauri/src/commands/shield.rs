use ai_launcher_core::shield::browser::BrowserEngine;
use ai_launcher_core::shield::profile::{ProfileManager, ShieldProfile};
use ai_launcher_core::shield::engine::EngineManager;
use ai_launcher_core::shield::launcher::{self, BrowserLauncher};
use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::Emitter;
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
pub async fn list_shield_profiles(
    state: tauri::State<'_, AppState>,
    launcher_state: tauri::State<'_, ShieldLauncherState>,
) -> Result<Vec<ShieldProfileResponse>, String> {
    let mgr = ProfileManager::new(&state.base_dir);
    let profiles = mgr.list_profiles().map_err(|e| e.to_string())?;

    // Cross-reference with the in-memory launcher state for authoritative
    // running status (on-disk metadata can be stale after crashes or races).
    let launcher = launcher_state.launcher.lock().await;
    let running_ids = launcher.running_profiles().await;
    drop(launcher);

    Ok(profiles
        .into_iter()
        .map(|p| {
            let actually_running = running_ids.contains(&p.id);
            let mut resp = ShieldProfileResponse::from(p);
            resp.is_running = actually_running;
            resp
        })
        .collect())
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
pub async fn get_shield_status(
    state: tauri::State<'_, AppState>,
    launcher_state: tauri::State<'_, ShieldLauncherState>,
) -> Result<ShieldStatusResponse, String> {
    let profile_mgr = ProfileManager::new(&state.base_dir);
    let engine_mgr = EngineManager::new(&state.base_dir);

    let profiles = profile_mgr.list_profiles().map_err(|e| e.to_string())?;
    let engines = engine_mgr.list_downloaded().map_err(|e| e.to_string())?;

    // Use in-memory launcher state for authoritative running count
    let launcher = launcher_state.launcher.lock().await;
    let running_count = launcher.running_profiles().await.len();
    drop(launcher);

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
    app: tauri::AppHandle,
    launcher_state: tauri::State<'_, ShieldLauncherState>,
    id: String,
    url: Option<String>,
) -> Result<u16, String> {
    let launcher = launcher_state.launcher.lock().await;
    let port = launcher
        .launch_profile(
            &id,
            url.as_deref(),
            Some(move |profile_id: String| {
                log::info!("Emitting shield-profile-stopped for '{}'", profile_id);
                let _ = app.emit("shield-profile-stopped", profile_id);
            }),
        )
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

#[derive(Clone, Serialize)]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
    percent: u8,
}

#[tauri::command]
pub async fn download_shield_engine(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    engine: String,
    version: String,
) -> Result<String, String> {
    let engine_type = BrowserEngine::from_str(&engine).map_err(|e: anyhow::Error| e.to_string())?;
    let download_url = launcher::get_download_url(&engine_type, &version)
        .map_err(|e: anyhow::Error| e.to_string())?;

    let engine_mgr = EngineManager::new(&state.base_dir);

    let app_handle = app.clone();
    let last_percent = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(255));
    let install_dir = engine_mgr
        .download_engine(&engine_type, &version, &download_url, move |downloaded, total| {
            let percent = if total > 0 {
                ((downloaded as f64 / total as f64) * 100.0).min(100.0) as u8
            } else {
                0
            };
            // Only emit when percent actually changes to avoid excessive re-renders
            let prev = last_percent.swap(percent, std::sync::atomic::Ordering::Relaxed);
            if prev != percent {
                let _ = app_handle.emit("shield-download-progress", DownloadProgress {
                    downloaded,
                    total,
                    percent,
                });
            }
        })
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

#[tauri::command]
pub fn remove_shield_engine(
    state: tauri::State<'_, AppState>,
    engine: String,
    version: String,
) -> Result<(), String> {
    let engine_type = BrowserEngine::from_str(&engine).map_err(|e: anyhow::Error| e.to_string())?;
    let engine_mgr = EngineManager::new(&state.base_dir);
    engine_mgr.remove_engine(&engine_type, &version).map_err(|e| e.to_string())
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
