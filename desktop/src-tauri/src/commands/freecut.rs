//! Tauri IPC commands for the FreeCut video editor.
//!
//! All media processing runs in Rust; the frontend receives results via
//! Tauri events (never polls). Heavy work is spawned into blocking threads
//! to avoid starving the async runtime.

use crate::state::AppState;
use ai_launcher_core::freecut::{
    dubbing::{
        detect_local_tools, generate_dubbing_assets, import_srt_as_session, DubbingImportResult,
        DubbingToolReport, WhisperSettings,
    },
    media_probe,
    project::{
        DubbingSession, ExportConfig, ExportProgressEvent, FrameRenderedEvent, MediaImportedEvent,
        MediaMetadata, Project, ProjectResolution, ThumbnailsReadyEvent, WaveformReadyEvent,
    },
    storage::{FreeCutStore, ProjectSummary},
};
use ai_launcher_core::media::ffmpeg as ffmpeg_bootstrap;
use ai_launcher_core::uv_env::{ensure_uv, UvEnv};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Mutex;

/// Managed state for FreeCut — holds the SQLite store.
pub struct FreeCutState {
    pub store: Arc<Mutex<FreeCutStore>>,
    pub base_dir: PathBuf,
    pub freecut_dir: PathBuf,
    pub projects_dir: PathBuf,
    pub media_dir: PathBuf,
    pub thumbnails_dir: PathBuf,
    pub render_dir: PathBuf,
    pub tooling_dir: PathBuf,
    pub tool_env_lock: Arc<Mutex<()>>,
}

impl FreeCutState {
    pub fn new(base_dir: &std::path::Path) -> anyhow::Result<Self> {
        let freecut_dir = base_dir.join("freecut");
        let projects_dir = freecut_dir.join("projects");
        let media_dir = freecut_dir.join("media");
        let thumbnails_dir = freecut_dir.join("thumbnails");
        let render_dir = freecut_dir.join("render");
        let tooling_dir = freecut_dir.join("tooling");

        std::fs::create_dir_all(&projects_dir)?;
        std::fs::create_dir_all(&media_dir)?;
        std::fs::create_dir_all(&thumbnails_dir)?;
        std::fs::create_dir_all(&render_dir)?;
        std::fs::create_dir_all(&tooling_dir)?;

        let db_path = freecut_dir.join("freecut.db");
        let store = FreeCutStore::new(&db_path)?;

        Ok(Self {
            store: Arc::new(Mutex::new(store)),
            base_dir: base_dir.to_path_buf(),
            freecut_dir,
            projects_dir,
            media_dir,
            thumbnails_dir,
            render_dir,
            tooling_dir,
            tool_env_lock: Arc::new(Mutex::new(())),
        })
    }

    /// Resolve FFmpeg bins (bundled → system → auto-download).
    pub fn resolve_ffmpeg(&self) -> Result<ffmpeg_bootstrap::FfmpegBins, String> {
        ffmpeg_bootstrap::ensure_ffmpeg(&self.base_dir).map_err(|e| e.to_string())
    }

    /// Get ffprobe path string for media probing.
    pub fn ffprobe_str(&self) -> Option<String> {
        ffmpeg_bootstrap::find_ffmpeg(&self.base_dir)
            .map(|bins| bins.ffprobe.to_string_lossy().to_string())
    }

    /// Get ffmpeg path string for media processing.
    pub fn ffmpeg_str(&self) -> Option<String> {
        ffmpeg_bootstrap::find_ffmpeg(&self.base_dir)
            .map(|bins| bins.ffmpeg.to_string_lossy().to_string())
    }
}

fn dubbing_runtime_workspace(tooling_dir: &std::path::Path) -> PathBuf {
    tooling_dir.join("dubbing-runtime")
}

fn dubbing_runtime_bin_dir(tooling_dir: &std::path::Path) -> PathBuf {
    let workspace = dubbing_runtime_workspace(tooling_dir);
    if cfg!(windows) {
        workspace.join(".venv").join("Scripts")
    } else {
        workspace.join(".venv").join("bin")
    }
}

fn with_tool_runtime_path<T, F>(tooling_dir: PathBuf, op: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    let runtime_bin = dubbing_runtime_bin_dir(&tooling_dir);
    let old_path = std::env::var("PATH").ok();

    if runtime_bin.exists() {
        let sep = if cfg!(windows) { ";" } else { ":" };
        let new_path = match &old_path {
            Some(existing) if !existing.is_empty() => {
                format!("{}{}{}", runtime_bin.to_string_lossy(), sep, existing)
            }
            _ => runtime_bin.to_string_lossy().to_string(),
        };
        std::env::set_var("PATH", new_path);
    }

    let result = op();

    match old_path {
        Some(path) => std::env::set_var("PATH", path),
        None => std::env::remove_var("PATH"),
    }

    result
}

/// Generate a unique ID using timestamp + thread hash for uniqueness.
fn generate_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut hasher = DefaultHasher::new();
    ts.hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);
    let h = hasher.finish();
    format!("fc-{ts:x}-{h:08x}")
}

// ─── Project CRUD ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateProjectArgs {
    pub name: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub fps: Option<u32>,
}

#[tauri::command]
pub async fn freecut_create_project(
    state: tauri::State<'_, FreeCutState>,
    args: CreateProjectArgs,
) -> Result<Project, String> {
    let now = Utc::now();
    let project = Project {
        id: generate_id(),
        name: args.name,
        description: String::new(),
        created_at: now,
        updated_at: now,
        duration: 0,
        schema_version: 1,
        metadata: ProjectResolution {
            width: args.width.unwrap_or(1920),
            height: args.height.unwrap_or(1080),
            fps: args.fps.unwrap_or(30),
            ..Default::default()
        },
        timeline: None,
        dubbing: None,
    };

    let store = state.store.lock().await;
    store.save_project(&project).map_err(|e| e.to_string())?;

    Ok(project)
}

#[tauri::command]
pub async fn freecut_list_projects(
    state: tauri::State<'_, FreeCutState>,
) -> Result<Vec<ProjectSummary>, String> {
    let store = state.store.lock().await;
    store.list_projects().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn freecut_get_project(
    state: tauri::State<'_, FreeCutState>,
    id: String,
) -> Result<Option<Project>, String> {
    let store = state.store.lock().await;
    store.get_project(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn freecut_save_project(
    state: tauri::State<'_, FreeCutState>,
    project: Project,
) -> Result<(), String> {
    let store = state.store.lock().await;
    store.save_project(&project).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn freecut_delete_project(
    state: tauri::State<'_, FreeCutState>,
    id: String,
) -> Result<bool, String> {
    let store = state.store.lock().await;
    store.delete_project(&id).map_err(|e| e.to_string())
}

// ─── Media Import ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn freecut_import_media(
    app: tauri::AppHandle,
    state: tauri::State<'_, FreeCutState>,
    project_id: String,
    file_path: String,
) -> Result<MediaMetadata, String> {
    let src = PathBuf::from(&file_path);
    if !src.exists() {
        return Err(format!("file not found: {file_path}"));
    }

    // Copy to media directory.
    let file_name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let dest = state.media_dir.join(&file_name);

    // Don't re-copy if already in media dir.
    if src != dest {
        tokio::fs::copy(&src, &dest)
            .await
            .map_err(|e| format!("copy failed: {e}"))?;
    }

    // Resolve FFmpeg (auto-downloads if needed) — runs on blocking thread.
    let base_dir = state.base_dir.clone();
    let ffprobe_bin = tokio::task::spawn_blocking(move || {
        ffmpeg_bootstrap::ensure_ffmpeg(&base_dir)
            .ok()
            .map(|bins| bins.ffprobe.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?;

    // Probe on blocking thread (FFprobe is sync).
    let dest_clone = dest.clone();
    let meta = tokio::task::spawn_blocking(move || {
        media_probe::probe_media(&dest_clone, ffprobe_bin.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // Persist to DB.
    {
        let store = state.store.lock().await;
        store
            .save_media(&project_id, &meta)
            .map_err(|e| e.to_string())?;
    }

    // Emit event.
    let _ = app.emit(
        "freecut://media-imported",
        MediaImportedEvent {
            media: meta.clone(),
        },
    );

    Ok(meta)
}

#[tauri::command]
pub async fn freecut_probe_media(
    state: tauri::State<'_, FreeCutState>,
    file_path: String,
) -> Result<MediaMetadata, String> {
    let path = PathBuf::from(&file_path);
    let ffprobe_bin = state.ffprobe_str();
    tokio::task::spawn_blocking(move || media_probe::probe_media(&path, ffprobe_bin.as_deref()))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn freecut_generate_thumbnails(
    app: tauri::AppHandle,
    state: tauri::State<'_, FreeCutState>,
    media_id: String,
    file_path: String,
    count: Option<usize>,
) -> Result<Vec<String>, String> {
    let src = PathBuf::from(&file_path);
    let out_dir = state.thumbnails_dir.join(&media_id);
    let thumb_count = count.unwrap_or(10);
    let ffmpeg_bin = state.ffmpeg_str();

    let paths = tokio::task::spawn_blocking(move || {
        media_probe::generate_thumbnails(&src, &out_dir, thumb_count, 320, ffmpeg_bin.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "freecut://thumbnails-ready",
        ThumbnailsReadyEvent {
            media_id: media_id.clone(),
            thumbnail_paths: paths.clone(),
        },
    );

    // Persist the first thumbnail path so list_media returns it on reload.
    if let Some(thumb) = paths.first() {
        let store = state.store.lock().await;
        let _ = store.update_media_thumbnail(&media_id, thumb);
    }

    Ok(paths)
}

#[tauri::command]
pub async fn freecut_generate_waveform(
    app: tauri::AppHandle,
    state: tauri::State<'_, FreeCutState>,
    media_id: String,
    file_path: String,
    sample_count: Option<usize>,
) -> Result<Vec<f32>, String> {
    let src = PathBuf::from(&file_path);
    let samples = sample_count.unwrap_or(500);
    let ffmpeg_bin = state.ffmpeg_str();

    let peaks =
        tokio::task::spawn_blocking(move || media_probe::generate_waveform(&src, samples, ffmpeg_bin.as_deref()))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "freecut://waveform-ready",
        WaveformReadyEvent {
            media_id,
            peaks: peaks.clone(),
        },
    );

    Ok(peaks)
}

// ─── Media Library ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn freecut_list_media(
    state: tauri::State<'_, FreeCutState>,
    project_id: String,
) -> Result<Vec<MediaMetadata>, String> {
    let store = state.store.lock().await;
    store.list_media(&project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn freecut_delete_media(
    state: tauri::State<'_, FreeCutState>,
    media_id: String,
) -> Result<bool, String> {
    let store = state.store.lock().await;
    store.delete_media(&media_id).map_err(|e| e.to_string())
}

// ─── Rendering ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn freecut_render_frame(
    app: tauri::AppHandle,
    state: tauri::State<'_, FreeCutState>,
    project: Project,
    frame: u32,
) -> Result<String, String> {
    let render_dir = state.render_dir.join(&project.id);
    let ffmpeg_bin = state.ffmpeg_str();
    let output = tokio::task::spawn_blocking(move || {
        ai_launcher_core::freecut::render_engine::render_frame(
            &project, frame, &render_dir, ffmpeg_bin.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let path_str = output.path.to_string_lossy().to_string();

    let _ = app.emit(
        "freecut://frame-rendered",
        FrameRenderedEvent {
            frame,
            bitmap_path: path_str.clone(),
            width: output.width,
            height: output.height,
        },
    );

    Ok(path_str)
}

// ─── Hardware Encoders ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct HwEncoder {
    pub name: String,
    pub codec: String,
    pub device: String,
}

#[tauri::command]
pub async fn freecut_get_hw_encoders(
    state: tauri::State<'_, FreeCutState>,
) -> Result<Vec<HwEncoder>, String> {
    let ffmpeg_bin = state.ffmpeg_str().unwrap_or_else(|| "ffmpeg".to_string());

    // Detect available hardware encoders via ffmpeg -encoders.
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&ffmpeg_bin)
            .args(["-hide_banner", "-encoders"])
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("ffmpeg not found: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut encoders = Vec::new();

    let hw_names = [
        ("h264_nvenc", "H.264", "NVIDIA NVENC"),
        ("hevc_nvenc", "H.265", "NVIDIA NVENC"),
        ("h264_amf", "H.264", "AMD AMF"),
        ("hevc_amf", "H.265", "AMD AMF"),
        ("h264_qsv", "H.264", "Intel QuickSync"),
        ("hevc_qsv", "H.265", "Intel QuickSync"),
        ("h264_videotoolbox", "H.264", "VideoToolbox"),
        ("hevc_videotoolbox", "H.265", "VideoToolbox"),
    ];

    for (name, codec, device) in &hw_names {
        if stdout.contains(name) {
            encoders.push(HwEncoder {
                name: name.to_string(),
                codec: codec.to_string(),
                device: device.to_string(),
            });
        }
    }

    Ok(encoders)
}

// ─── Export ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct ExportCompleteEvent {
    pub success: bool,
    pub output_path: String,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn freecut_export_video(
    app: tauri::AppHandle,
    state: tauri::State<'_, FreeCutState>,
    project: Project,
    config: ExportConfig,
) -> Result<String, String> {
    let app_clone = app.clone();
    let ffmpeg_bin = state.ffmpeg_str();

    let result = tokio::task::spawn_blocking(move || {
        ai_launcher_core::freecut::render_engine::export_video(
            &project,
            &config,
            ffmpeg_bin.as_deref(),
            move |current_frame, total_frames| {
                let percent = if total_frames > 0 {
                    (current_frame as f64 / total_frames as f64) * 100.0
                } else {
                    0.0
                };
                let _ = app_clone.emit(
                    "freecut://export-progress",
                    ExportProgressEvent {
                        percent,
                        current_frame,
                        total_frames,
                        eta_secs: None,
                    },
                );
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?;

    match result {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            let _ = app.emit(
                "freecut://export-complete",
                ExportCompleteEvent {
                    success: true,
                    output_path: path_str.clone(),
                    error: None,
                },
            );
            Ok(path_str)
        }
        Err(e) => {
            let err_msg = e.to_string();
            let _ = app.emit(
                "freecut://export-complete",
                ExportCompleteEvent {
                    success: false,
                    output_path: String::new(),
                    error: Some(err_msg.clone()),
                },
            );
            Err(err_msg)
        }
    }
}

// ─── Dubbing ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct DubbingReadyEvent {
    pub session: DubbingSession,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DubbingRuntimeInstallResult {
    pub runtime: String,
    pub installed_packages: Vec<String>,
    pub workspace_path: String,
    pub bin_path: String,
    pub message: String,
}

#[tauri::command]
pub async fn freecut_detect_dubbing_tools(
    state: tauri::State<'_, FreeCutState>,
) -> Result<DubbingToolReport, String> {
    let tooling_dir = state.tooling_dir.clone();
    let _guard = state.tool_env_lock.lock().await;
    tokio::task::spawn_blocking(move || {
        with_tool_runtime_path(tooling_dir, || {
            detect_local_tools().map_err(|e| e.to_string())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn freecut_import_dubbing_srt(file_path: String) -> Result<DubbingImportResult, String> {
    let path = PathBuf::from(file_path);
    tokio::task::spawn_blocking(move || import_srt_as_session(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn freecut_generate_dub_assets(
    app: tauri::AppHandle,
    state: tauri::State<'_, FreeCutState>,
    project_id: String,
    session: DubbingSession,
    whisper: Option<WhisperSettings>,
) -> Result<DubbingSession, String> {
    let render_dir = state.render_dir.clone();
    let tooling_dir = state.tooling_dir.clone();
    let app_clone = app.clone();
    let whisper = whisper.unwrap_or_default();
    let _guard = state.tool_env_lock.lock().await;

    let result = tokio::task::spawn_blocking(move || {
        with_tool_runtime_path(tooling_dir, || {
            generate_dubbing_assets(&project_id, &render_dir, session, whisper, move |update| {
                let _ = app_clone.emit("freecut://dubbing-progress", update);
            })
            .map_err(|e| e.to_string())
        })
    })
    .await
    .map_err(|e| e.to_string())??;

    let _ = app.emit(
        "freecut://dubbing-ready",
        DubbingReadyEvent {
            session: result.clone(),
        },
    );
    Ok(result)
}

#[tauri::command]
pub async fn freecut_install_dubbing_runtime(
    app_state: tauri::State<'_, AppState>,
    state: tauri::State<'_, FreeCutState>,
    runtime: String,
) -> Result<DubbingRuntimeInstallResult, String> {
    let base_dir = app_state.base_dir.clone();
    let tooling_dir = state.tooling_dir.clone();
    let runtime_name = runtime.to_ascii_lowercase();
    let _guard = state.tool_env_lock.lock().await;

    tokio::task::spawn_blocking(move || {
        let runtime_workspace = dubbing_runtime_workspace(&tooling_dir);
        std::fs::create_dir_all(&runtime_workspace).map_err(|e| e.to_string())?;

        let packages = match runtime_name.as_str() {
            "core" => vec!["openai-whisper".to_string(), "edge-tts".to_string()],
            "whisper" => vec!["openai-whisper".to_string()],
            "edge_tts" => vec!["edge-tts".to_string()],
            "diarization" => vec!["whisperx".to_string()],
            other => return Err(format!("unknown runtime '{other}'")),
        };

        let uv_bin = ensure_uv(&base_dir).map_err(|e| e.to_string())?;
        let uv = UvEnv::new(&uv_bin, &runtime_workspace, "3.11");
        uv.ensure_python().map_err(|e| e.to_string())?;
        uv.create_venv().map_err(|e| e.to_string())?;
        uv.install_deps(&packages).map_err(|e| e.to_string())?;

        let bin_dir = dubbing_runtime_bin_dir(&tooling_dir);
        Ok(DubbingRuntimeInstallResult {
            runtime: runtime_name,
            installed_packages: packages,
            workspace_path: runtime_workspace.to_string_lossy().to_string(),
            bin_path: bin_dir.to_string_lossy().to_string(),
            message: "Installed into the FreeCut NDE-OS runtime".to_string(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn freecut_remove_background(
    app_state: tauri::State<'_, AppState>,
    input_path: String,
    output_path: String,
) -> Result<(), String> {
    let base_dir = app_state.base_dir.clone();
    tokio::task::spawn_blocking(move || {
        let rt = ai_launcher_core::freecut::vision::VisionRuntime::new(&base_dir);
        let in_p = std::path::Path::new(&input_path);
        let out_p = std::path::Path::new(&output_path);
        if let Some(parent) = out_p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        rt.remove_background(in_p, out_p)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

// ─── Settings ──────────────────────────────────────────────────────────────────

/// Read a UI setting from the FreeCut SQLite DB.
/// Returns `null` (serialised as `None`) if the key does not exist.
#[tauri::command]
pub async fn freecut_get_setting(
    state: tauri::State<'_, FreeCutState>,
    key: String,
) -> Result<Option<String>, String> {
    let store = state.store.lock().await;
    store.get_setting(&key).map_err(|e| e.to_string())
}

/// Persist a UI setting into the FreeCut SQLite DB (upsert).
#[tauri::command]
pub async fn freecut_set_setting(
    state: tauri::State<'_, FreeCutState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let store = state.store.lock().await;
    store.set_setting(&key, &value).map_err(|e| e.to_string())
}

/// Delete a UI setting from the FreeCut SQLite DB.
/// Returns `true` if the key existed.
#[tauri::command]
pub async fn freecut_delete_setting(
    state: tauri::State<'_, FreeCutState>,
    key: String,
) -> Result<bool, String> {
    let store = state.store.lock().await;
    store.delete_setting(&key).map_err(|e| e.to_string())
}
