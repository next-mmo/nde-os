//! Tauri IPC commands for the FreeCut video editor.
//!
//! All media processing runs in Rust; the frontend receives results via
//! Tauri events (never polls). Heavy work is spawned into blocking threads
//! to avoid starving the async runtime.

use ai_launcher_core::freecut::{
    media_probe,
    project::{
        ExportConfig, ExportProgressEvent, FrameRenderedEvent, MediaImportedEvent, MediaMetadata,
        Project, ProjectResolution, ThumbnailsReadyEvent, WaveformReadyEvent,
    },
    storage::{FreeCutStore, ProjectSummary},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Mutex;

/// Managed state for FreeCut — holds the SQLite store.
pub struct FreeCutState {
    pub store: Arc<Mutex<FreeCutStore>>,
    pub projects_dir: PathBuf,
    pub media_dir: PathBuf,
    pub thumbnails_dir: PathBuf,
    pub render_dir: PathBuf,
}

impl FreeCutState {
    pub fn new(base_dir: &std::path::Path) -> anyhow::Result<Self> {
        let freecut_dir = base_dir.join("freecut");
        let projects_dir = freecut_dir.join("projects");
        let media_dir = freecut_dir.join("media");
        let thumbnails_dir = freecut_dir.join("thumbnails");
        let render_dir = freecut_dir.join("render");

        std::fs::create_dir_all(&projects_dir)?;
        std::fs::create_dir_all(&media_dir)?;
        std::fs::create_dir_all(&thumbnails_dir)?;
        std::fs::create_dir_all(&render_dir)?;

        let db_path = freecut_dir.join("freecut.db");
        let store = FreeCutStore::new(&db_path)?;

        Ok(Self {
            store: Arc::new(Mutex::new(store)),
            projects_dir,
            media_dir,
            thumbnails_dir,
            render_dir,
        })
    }
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

    // Probe on blocking thread (FFprobe is sync).
    let dest_clone = dest.clone();
    let meta = tokio::task::spawn_blocking(move || media_probe::probe_media(&dest_clone, None))
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
    let _ = app.emit("freecut://media-imported", MediaImportedEvent {
        media: meta.clone(),
    });

    Ok(meta)
}

#[tauri::command]
pub async fn freecut_probe_media(file_path: String) -> Result<MediaMetadata, String> {
    let path = PathBuf::from(&file_path);
    tokio::task::spawn_blocking(move || media_probe::probe_media(&path, None))
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

    let paths = tokio::task::spawn_blocking(move || {
        media_probe::generate_thumbnails(&src, &out_dir, thumb_count, 320, None)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "freecut://thumbnails-ready",
        ThumbnailsReadyEvent {
            media_id,
            thumbnail_paths: paths.clone(),
        },
    );

    Ok(paths)
}

#[tauri::command]
pub async fn freecut_generate_waveform(
    app: tauri::AppHandle,
    media_id: String,
    file_path: String,
    sample_count: Option<usize>,
) -> Result<Vec<f32>, String> {
    let src = PathBuf::from(&file_path);
    let samples = sample_count.unwrap_or(500);

    let peaks = tokio::task::spawn_blocking(move || {
        media_probe::generate_waveform(&src, samples, None)
    })
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
    let output = tokio::task::spawn_blocking(move || {
        ai_launcher_core::freecut::render_engine::render_frame(&project, frame, &render_dir, None)
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
pub async fn freecut_get_hw_encoders() -> Result<Vec<HwEncoder>, String> {
    // Detect available hardware encoders via ffmpeg -encoders.
    let output = tokio::task::spawn_blocking(|| {
        std::process::Command::new("ffmpeg")
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
    _state: tauri::State<'_, FreeCutState>,
    project: Project,
    config: ExportConfig,
) -> Result<String, String> {
    let app_clone = app.clone();

    let result = tokio::task::spawn_blocking(move || {
        ai_launcher_core::freecut::render_engine::export_video(
            &project,
            &config,
            None,
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
