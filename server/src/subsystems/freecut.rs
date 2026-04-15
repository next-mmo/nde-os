use crate::response::{err, ok, parse_body, sse_response, HttpResponse};
use ai_launcher_core::freecut::movie_dub::{
    config::MovieDubConfig,
    lang::Lang,
    pipeline::DubVideoOptions,
    MovieDubPipeline,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
pub struct DubVideoRequest {
    pub input_path: String,
    pub output_path: Option<String>,
    pub export_orig_srt: Option<bool>,
    pub export_mode: Option<String>,
    /// Path to a pre-translated SRT file. Skips Whisper + Translation when provided.
    pub srt_path: Option<String>,
}

/// POST /api/freecut/dub
pub fn handle_dub(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: DubVideoRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let input_path = match std::fs::canonicalize(&payload.input_path) {
        Ok(p) => p,
        Err(_) => return err(400, "Input file does not exist or is invalid"),
    };

    let input_filename = input_path.file_stem().unwrap_or_default().to_string_lossy();
    let parent = input_path.parent().unwrap_or(Path::new(""));
    let output_path = payload.output_path.map(PathBuf::from).unwrap_or_else(|| {
        parent.join(format!("{}_dubbed.mp4", input_filename))
    });

    // We use data_dir (e.g. ~/.ai-launcher) as base_dir for VoiceRuntime/FFmpeg
    let workspace = data_dir.join("movie_dub_workspace");

    std::fs::create_dir_all(&workspace).ok();

    // 1. Check VoiceDependencies
    let voice_rt = ai_launcher_core::voice::runtime::VoiceRuntime::new(data_dir);
    if voice_rt.resolve_tool("whisper").is_none()
        || voice_rt.resolve_tool("edge-tts").is_none()
        || voice_rt.resolve_tool("demucs").is_none()
    {
        tracing::info!("Auto-installing Voice tools via NDE-OS service hub...");
        std::fs::create_dir_all(voice_rt.workspace_dir()).ok();
        if let Ok(uv_bin) = ai_launcher_core::uv_env::ensure_uv(data_dir) {
            let uv = ai_launcher_core::uv_env::UvEnv::new(&uv_bin, voice_rt.workspace_dir(), "3.11");
            let _ = uv.ensure_python();
            let _ = uv.create_venv();
            let _ = uv.install_deps(&[
                "openai-whisper".to_string(),
                "edge-tts".to_string(),
                "demucs".to_string(),
            ]);
        }
    }

    // 2. Initialize pipeline (defaults to English -> Khmer using free Lingva)
    let config = MovieDubConfig::default();
    let pipeline = match MovieDubPipeline::new(config, data_dir, workspace) {
        Ok(p) => p,
        Err(e) => return err(500, &format!("Pipeline init error: {}", e)),
    };

    let export_mode = match payload.export_mode.as_deref() {
        Some("SpeechOnly") => ai_launcher_core::freecut::movie_dub::pipeline::ExportMode::SpeechOnly,
        Some("BackgroundOnly") => ai_launcher_core::freecut::movie_dub::pipeline::ExportMode::BackgroundOnly,
        Some("DualAudio") => {
            // we handle DualAudio differently, but we can keep MergeAll as the mode
            ai_launcher_core::freecut::movie_dub::pipeline::ExportMode::MergeAll
        }
        _ => ai_launcher_core::freecut::movie_dub::pipeline::ExportMode::MergeAll,
    };
    
    let is_dual = payload.export_mode.as_deref() == Some("DualAudio");

    let srt_path = payload.srt_path.as_ref().map(|p| {
        match std::fs::canonicalize(p) {
            Ok(cp) => cp,
            Err(_) => PathBuf::from(p),
        }
    });

    let opts = DubVideoOptions {
        input_path,
        output_path: output_path.clone(),
        source_lang: Lang::En,
        dual_audio: is_dual,
        generate_subtitles: true,
        export_orig_srt: payload.export_orig_srt.unwrap_or(false),
        export_mode,
        burn_subtitles: false,
        srt_path,
    };

    // Run blocking wait
    let result = rt.block_on(async {
        pipeline
            .dub_video(&opts, |phase, progress, msg| {
                tracing::info!(
                    "[MovieDub] [{}] {:.2}% - {}",
                    phase,
                    progress * 100.0,
                    msg
                );
            })
            .await
    });

    match result {
        Ok(out) => ok(
            "Dubbing complete",
            serde_json::json!({ "output_path": out.to_string_lossy() }),
        ),
        Err(e) => err(500, &format!("Dubbing failed: {}", e)),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Split API
// ────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SplitVideoRequest {
    pub input_path: String,
    pub segment_duration_secs: u64,
    pub target_lang: Option<String>,
}

/// POST /api/freecut/dub/split
pub fn handle_split(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: SplitVideoRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let input_path = match std::fs::canonicalize(&payload.input_path) {
        Ok(p) => p,
        Err(_) => return err(400, "Input file does not exist or is invalid"),
    };

    if payload.segment_duration_secs == 0 {
        return err(400, "segment_duration_secs must be > 0");
    }

    let target_lang = match payload.target_lang.as_deref() {
        Some("km") | None => Lang::Km,
        Some("en") => Lang::En,
        Some("zh") => Lang::Zh,
        Some(other) => return err(400, &format!("Unsupported target language: {}", other)),
    };

    // Resolve tools.
    let voice_rt = ai_launcher_core::voice::runtime::VoiceRuntime::new(data_dir);
    let ffmpeg_dir = data_dir.join(".ffmpeg");
    let ffmpeg_path = ffmpeg_dir.join("ffmpeg");
    let whisper_path = match voice_rt.resolve_tool("whisper") {
        Some(p) => p,
        None => return err(500, "Whisper not found. Install via Service Hub → Voice Runtime"),
    };

    let config = MovieDubConfig::default();

    let result = rt.block_on(async {
        ai_launcher_core::freecut::movie_dub::split::execute_split(
            &config,
            &ffmpeg_path,
            &whisper_path,
            data_dir,
            &input_path,
            payload.segment_duration_secs,
            target_lang,
            |part_idx, total, msg| {
                tracing::info!("[Split] [{}/{}] {}", part_idx + 1, total, msg);
            },
        )
        .await
    });

    match result {
        Ok(job) => ok("Split complete", serde_json::to_value(&job).unwrap_or_default()),
        Err(e) => err(500, &format!("Split failed: {}", e)),
    }
}

/// POST /api/freecut/dub/split-stream
///
/// Like `handle_split` but returns a Server-Sent Events body so the caller
/// can read progress in real-time.  Each part emits a `progress` event with:
///   `{ "part": N, "total": T, "status": "done"|"error", "msg": "..." }`
/// followed by a final `done` event carrying the full job JSON.
pub fn handle_split_stream(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: SplitVideoRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let input_path = match std::fs::canonicalize(&payload.input_path) {
        Ok(p) => p,
        Err(_) => return err(400, "Input file does not exist or is invalid"),
    };

    if payload.segment_duration_secs == 0 {
        return err(400, "segment_duration_secs must be > 0");
    }

    let target_lang = match payload.target_lang.as_deref() {
        Some("km") | None => Lang::Km,
        Some("en") => Lang::En,
        Some("zh") => Lang::Zh,
        Some(other) => return err(400, &format!("Unsupported target language: {}", other)),
    };

    let voice_rt = ai_launcher_core::voice::runtime::VoiceRuntime::new(data_dir);
    let ffmpeg_dir = data_dir.join(".ffmpeg");
    let ffmpeg_path = ffmpeg_dir.join("ffmpeg");
    let whisper_path = match voice_rt.resolve_tool("whisper") {
        Some(p) => p,
        None => return err(500, "Whisper not found. Install via Service Hub → Voice Runtime"),
    };

    let config = MovieDubConfig::default();

    // Channel to collect per-part progress events from the async callback.
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    let result = rt.block_on(async {
        ai_launcher_core::freecut::movie_dub::split::execute_split(
            &config,
            &ffmpeg_path,
            &whisper_path,
            data_dir,
            &input_path,
            payload.segment_duration_secs,
            target_lang,
            |part_idx, total, msg| {
                let event = serde_json::json!({
                    "part": part_idx + 1,
                    "total": total,
                    "status": "done",
                    "msg": msg,
                });
                let line = format!("event: progress\ndata: {}\n\n", event);
                let _ = tx.send(line);
                tracing::info!("[Split] [{}/{}] {}", part_idx + 1, total, msg);
            },
        )
        .await
    });

    // Drain all progress events collected during the run.
    let mut body = String::new();
    while let Ok(event) = rx.try_recv() {
        body.push_str(&event);
    }

    // Append final `done` or `error` event.
    match result {
        Ok(job) => {
            let job_json = serde_json::to_string(&job).unwrap_or_default();
            body.push_str(&format!("event: done\ndata: {}\n\n", job_json));
        }
        Err(e) => {
            let err_json = serde_json::json!({ "error": format!("{}", e) });
            body.push_str(&format!("event: error\ndata: {}\n\n", err_json));
        }
    }

    sse_response(body.into_bytes())
}

// ────────────────────────────────────────────────────────────────────────
// Dub Part API
// ────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DubPartRequest {
    pub job_id: String,
    pub part_index: usize,
    pub srt_path: Option<String>,
    pub export_mode: Option<String>,
}

/// POST /api/freecut/dub/part
pub fn handle_dub_part(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: DubPartRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let job_workspace = data_dir
        .join("movie_dub_workspace")
        .join("jobs")
        .join(&payload.job_id);

    let mut job = match ai_launcher_core::freecut::movie_dub::split::SplitJob::load(&job_workspace) {
        Ok(j) => j,
        Err(e) => return err(404, &format!("Job not found: {}", e)),
    };

    let part = match job.parts.get(payload.part_index) {
        Some(p) => p.clone(),
        None => return err(400, &format!("Part index {} out of range (total: {})", payload.part_index, job.parts.len())),
    };

    // Use user-provided SRT or the auto-translated one.
    let srt_path = payload.srt_path
        .as_ref()
        .map(|p| std::fs::canonicalize(p).unwrap_or_else(|_| PathBuf::from(p)))
        .unwrap_or_else(|| part.translated_srt_path.clone());

    let dubbed_output = part.video_path.with_extension("dubbed.mp4");

    let export_mode = match payload.export_mode.as_deref() {
        Some("SpeechOnly") => ai_launcher_core::freecut::movie_dub::pipeline::ExportMode::SpeechOnly,
        Some("BackgroundOnly") => ai_launcher_core::freecut::movie_dub::pipeline::ExportMode::BackgroundOnly,
        _ => ai_launcher_core::freecut::movie_dub::pipeline::ExportMode::MergeAll,
    };

    // Set up per-part workspace.
    let part_workspace = part.video_path.parent().unwrap().join("dub_work");
    std::fs::create_dir_all(&part_workspace).ok();

    let config = MovieDubConfig::default();
    let pipeline = match MovieDubPipeline::new(config, data_dir, part_workspace) {
        Ok(p) => p,
        Err(e) => return err(500, &format!("Pipeline init error: {}", e)),
    };

    let opts = DubVideoOptions {
        input_path: part.video_path.clone(),
        output_path: dubbed_output.clone(),
        source_lang: Lang::En,
        dual_audio: false,
        generate_subtitles: true,
        export_orig_srt: false,
        export_mode,
        burn_subtitles: false,
        srt_path: Some(srt_path),
    };

    let result = rt.block_on(async {
        pipeline
            .dub_video(&opts, |phase, progress, msg| {
                tracing::info!(
                    "[DubPart {}] [{}] {:.0}% - {}",
                    payload.part_index + 1,
                    phase,
                    progress * 100.0,
                    msg
                );
            })
            .await
    });

    match result {
        Ok(out) => {
            // Update job state.
            if let Some(p) = job.parts.get_mut(payload.part_index) {
                p.dubbed_path = Some(out.clone());
                p.status = ai_launcher_core::freecut::movie_dub::split::PartStatus::Dubbed;
            }
            let _ = job.save();
            ok(
                &format!("Part {} dubbed", payload.part_index + 1),
                serde_json::json!({ "dubbed_path": out.to_string_lossy() }),
            )
        }
        Err(e) => {
            if let Some(p) = job.parts.get_mut(payload.part_index) {
                p.status = ai_launcher_core::freecut::movie_dub::split::PartStatus::Error;
                p.error = Some(format!("{}", e));
            }
            let _ = job.save();
            err(500, &format!("Part dub failed: {}", e))
        }
    }
}

// ────────────────────────────────────────────────────────────────────────
// Merge API
// ────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MergeRequest {
    pub job_id: String,
    pub output_path: Option<String>,
}

/// POST /api/freecut/dub/merge
pub fn handle_merge(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: MergeRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let job_workspace = data_dir
        .join("movie_dub_workspace")
        .join("jobs")
        .join(&payload.job_id);

    let job = match ai_launcher_core::freecut::movie_dub::split::SplitJob::load(&job_workspace) {
        Ok(j) => j,
        Err(e) => return err(404, &format!("Job not found: {}", e)),
    };

    // Check all parts are dubbed.
    let not_dubbed: Vec<usize> = job.parts.iter()
        .filter(|p| p.dubbed_path.is_none())
        .map(|p| p.index + 1)
        .collect();

    if !not_dubbed.is_empty() {
        return err(400, &format!("Parts not yet dubbed: {:?}", not_dubbed));
    }

    let dubbed_paths: Vec<PathBuf> = job.parts.iter()
        .filter_map(|p| p.dubbed_path.clone())
        .collect();

    let output_path = payload.output_path
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let stem = job.input_path.file_stem().unwrap_or_default().to_string_lossy();
            job.input_path.parent().unwrap_or(Path::new(""))
                .join(format!("{}_dubbed_full.mp4", stem))
        });

    let ffmpeg_path = data_dir.join(".ffmpeg").join("ffmpeg");

    let result = rt.block_on(async {
        ai_launcher_core::freecut::movie_dub::merge::merge_parts(
            &ffmpeg_path,
            &dubbed_paths,
            &output_path,
        )
        .await
    });

    match result {
        Ok(out) => ok(
            "Merge complete",
            serde_json::json!({
                "output_path": out.to_string_lossy(),
                "total_parts": job.total_parts,
            }),
        ),
        Err(e) => err(500, &format!("Merge failed: {}", e)),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Job Status API
// ────────────────────────────────────────────────────────────────────────

/// GET /api/freecut/dub/job/{job_id}
pub fn handle_job_status(job_id: &str, data_dir: &Path) -> HttpResponse {
    let job_workspace = data_dir
        .join("movie_dub_workspace")
        .join("jobs")
        .join(job_id);

    match ai_launcher_core::freecut::movie_dub::split::SplitJob::load(&job_workspace) {
        Ok(job) => ok("Job found", serde_json::to_value(&job).unwrap_or_default()),
        Err(e) => err(404, &format!("Job not found: {}", e)),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Dub All API — sequentially dub every undubbed part in a job
// ────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DubAllRequest {
    pub job_id: String,
    pub export_mode: Option<String>,
}

/// POST /api/freecut/dub/all
pub fn handle_dub_all(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: DubAllRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let job_workspace = data_dir
        .join("movie_dub_workspace")
        .join("jobs")
        .join(&payload.job_id);

    let mut job = match ai_launcher_core::freecut::movie_dub::split::SplitJob::load(&job_workspace)
    {
        Ok(j) => j,
        Err(e) => return err(404, &format!("Job not found: {}", e)),
    };

    let export_mode_str = payload.export_mode.as_deref().unwrap_or("MergeAll");

    let mut results = Vec::new();
    let total = job.parts.len();

    for i in 0..total {
        // Skip already-dubbed parts.
        if job.parts[i].status
            == ai_launcher_core::freecut::movie_dub::split::PartStatus::Dubbed
        {
            results.push(serde_json::json!({ "part": i, "status": "already_dubbed" }));
            continue;
        }

        let part = job.parts[i].clone();
        let srt_path = part.translated_srt_path.clone();
        let dubbed_output = part.video_path.with_extension("dubbed.mp4");
        let part_workspace = part.video_path.parent().unwrap().join("dub_work");
        std::fs::create_dir_all(&part_workspace).ok();

        let export_mode = match export_mode_str {
            "SpeechOnly" => {
                ai_launcher_core::freecut::movie_dub::pipeline::ExportMode::SpeechOnly
            }
            "BackgroundOnly" => {
                ai_launcher_core::freecut::movie_dub::pipeline::ExportMode::BackgroundOnly
            }
            _ => ai_launcher_core::freecut::movie_dub::pipeline::ExportMode::MergeAll,
        };

        let config = MovieDubConfig::default();
        let pipeline = match MovieDubPipeline::new(config, data_dir, part_workspace) {
            Ok(p) => p,
            Err(e) => {
                job.parts[i].status =
                    ai_launcher_core::freecut::movie_dub::split::PartStatus::Error;
                job.parts[i].error = Some(format!("{}", e));
                let _ = job.save();
                results.push(
                    serde_json::json!({ "part": i, "status": "error", "error": format!("{}", e) }),
                );
                continue;
            }
        };

        let opts = DubVideoOptions {
            input_path: part.video_path.clone(),
            output_path: dubbed_output.clone(),
            source_lang: Lang::En,
            dual_audio: false,
            generate_subtitles: true,
            export_orig_srt: false,
            export_mode,
            burn_subtitles: false,
            srt_path: Some(srt_path),
        };

        tracing::info!("[DubAll] Starting part {}/{}", i + 1, total);

        let result = rt.block_on(async {
            pipeline
                .dub_video(&opts, |phase, progress, msg| {
                    tracing::info!(
                        "[DubAll {}/{}] [{}] {:.0}% - {}",
                        i + 1,
                        total,
                        phase,
                        progress * 100.0,
                        msg
                    );
                })
                .await
        });

        match result {
            Ok(out) => {
                job.parts[i].dubbed_path = Some(out.clone());
                job.parts[i].status =
                    ai_launcher_core::freecut::movie_dub::split::PartStatus::Dubbed;
                let _ = job.save();
                results.push(serde_json::json!({
                    "part": i,
                    "status": "dubbed",
                    "path": out.to_string_lossy()
                }));
            }
            Err(e) => {
                job.parts[i].status =
                    ai_launcher_core::freecut::movie_dub::split::PartStatus::Error;
                job.parts[i].error = Some(format!("{}", e));
                let _ = job.save();
                results.push(
                    serde_json::json!({ "part": i, "status": "error", "error": format!("{}", e) }),
                );
            }
        }
    }

    ok(
        "Dub all complete",
        serde_json::json!({
            "results": results,
            "job": serde_json::to_value(&job).unwrap_or_default()
        }),
    )
}

// ────────────────────────────────────────────────────────────────────────
// SRT Read / Save APIs
// ────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ReadSrtRequest {
    pub job_id: String,
    pub part_index: usize,
    /// "original" or "translated" (default)
    pub srt_type: Option<String>,
}

/// POST /api/freecut/dub/srt/read
pub fn handle_read_srt(
    req: &mut tiny_http::Request,
    data_dir: &Path,
) -> HttpResponse {
    let payload: ReadSrtRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let job_workspace = data_dir
        .join("movie_dub_workspace")
        .join("jobs")
        .join(&payload.job_id);

    let job = match ai_launcher_core::freecut::movie_dub::split::SplitJob::load(&job_workspace) {
        Ok(j) => j,
        Err(e) => return err(404, &format!("Job not found: {}", e)),
    };

    let part = match job.parts.get(payload.part_index) {
        Some(p) => p,
        None => {
            return err(
                400,
                &format!("Part index {} out of range", payload.part_index),
            )
        }
    };

    let srt_path = match payload.srt_type.as_deref() {
        Some("original") => &part.orig_srt_path,
        _ => &part.translated_srt_path,
    };

    match std::fs::read_to_string(srt_path) {
        Ok(content) => ok(
            "SRT content",
            serde_json::json!({
                "content": content,
                "path": srt_path.to_string_lossy()
            }),
        ),
        Err(e) => err(500, &format!("Failed to read SRT: {}", e)),
    }
}

#[derive(Deserialize)]
pub struct SaveSrtRequest {
    pub job_id: String,
    pub part_index: usize,
    pub content: String,
}

/// POST /api/freecut/dub/srt/save
pub fn handle_save_srt(
    req: &mut tiny_http::Request,
    data_dir: &Path,
) -> HttpResponse {
    let payload: SaveSrtRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let job_workspace = data_dir
        .join("movie_dub_workspace")
        .join("jobs")
        .join(&payload.job_id);

    let job = match ai_launcher_core::freecut::movie_dub::split::SplitJob::load(&job_workspace) {
        Ok(j) => j,
        Err(e) => return err(404, &format!("Job not found: {}", e)),
    };

    let part = match job.parts.get(payload.part_index) {
        Some(p) => p,
        None => {
            return err(
                400,
                &format!("Part index {} out of range", payload.part_index),
            )
        }
    };

    match std::fs::write(&part.translated_srt_path, &payload.content) {
        Ok(_) => ok(
            "SRT saved",
            serde_json::json!({ "path": part.translated_srt_path.to_string_lossy() }),
        ),
        Err(e) => err(500, &format!("Failed to save SRT: {}", e)),
    }
}

// ════════════════════════════════════════════════════════════════════════
// FFmpeg Tools — common operations exposed as REST endpoints
// ════════════════════════════════════════════════════════════════════════

fn resolve_ffmpeg(data_dir: &Path) -> PathBuf {
    data_dir.join(".ffmpeg").join("ffmpeg")
}

fn resolve_ffprobe(data_dir: &Path) -> PathBuf {
    data_dir.join(".ffmpeg").join("ffprobe")
}

/// Run an ffmpeg command and return success/error response.
fn run_ffmpeg_blocking(
    rt: &tokio::runtime::Runtime,
    ffmpeg: &Path,
    args: &[&str],
) -> Result<String, String> {
    let args_owned: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let ff = ffmpeg.to_path_buf();
    rt.block_on(async {
        let output = tokio::process::Command::new(&ff)
            .args(&args_owned)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("FFmpeg error: {}", stderr))
        }
    })
}

fn derive_output_path(input: &str, suffix: &str, ext: &str) -> String {
    let p = Path::new(input);
    let stem = p.file_stem().unwrap_or_default().to_string_lossy();
    let parent = p.parent().unwrap_or(Path::new("."));
    parent
        .join(format!("{}_{}.{}", stem, suffix, ext))
        .to_string_lossy()
        .to_string()
}

// ─── Convert Format ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ConvertRequest {
    pub input_path: String,
    pub output_format: String,   // mp4, mkv, avi, webm, mov
    pub output_path: Option<String>,
}

/// POST /api/freecut/ffmpeg/convert
pub fn handle_ffmpeg_convert(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: ConvertRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let ffmpeg = resolve_ffmpeg(data_dir);
    let fmt = payload.output_format.trim().to_lowercase();
    let allowed = ["mp4", "mkv", "avi", "webm", "mov", "ts", "flv"];
    if !allowed.contains(&fmt.as_str()) {
        return err(400, &format!("Unsupported format: {}", fmt));
    }

    let output = payload
        .output_path
        .unwrap_or_else(|| derive_output_path(&payload.input_path, "converted", &fmt));

    let args = vec![
        "-y", "-i", &payload.input_path,
        "-c", "copy",
        &output,
    ];

    match run_ffmpeg_blocking(rt, &ffmpeg, &args.iter().map(|s| *s).collect::<Vec<_>>()) {
        Ok(_) => ok("Converted", serde_json::json!({ "output_path": output })),
        Err(e) => err(500, &e),
    }
}

// ─── Extract Audio ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ExtractAudioRequest {
    pub input_path: String,
    pub output_format: Option<String>, // mp3, wav, aac, flac (default: mp3)
    pub output_path: Option<String>,
}

/// POST /api/freecut/ffmpeg/extract-audio
pub fn handle_ffmpeg_extract_audio(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: ExtractAudioRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let ffmpeg = resolve_ffmpeg(data_dir);
    let fmt = payload.output_format.as_deref().unwrap_or("mp3");
    let output = payload
        .output_path
        .unwrap_or_else(|| derive_output_path(&payload.input_path, "audio", fmt));

    let codec = match fmt {
        "mp3" => "libmp3lame",
        "aac" => "aac",
        "flac" => "flac",
        "wav" => "pcm_s16le",
        "ogg" => "libvorbis",
        _ => "libmp3lame",
    };

    let args: Vec<&str> = vec![
        "-y", "-i", &payload.input_path,
        "-vn", "-acodec", codec,
        &output,
    ];

    match run_ffmpeg_blocking(rt, &ffmpeg, &args) {
        Ok(_) => ok("Audio extracted", serde_json::json!({ "output_path": output })),
        Err(e) => err(500, &e),
    }
}

// ─── Trim / Cut ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TrimRequest {
    pub input_path: String,
    pub start_time: String,  // HH:MM:SS or seconds
    pub end_time: String,
    pub output_path: Option<String>,
}

/// POST /api/freecut/ffmpeg/trim
pub fn handle_ffmpeg_trim(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: TrimRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let ffmpeg = resolve_ffmpeg(data_dir);
    let ext = Path::new(&payload.input_path)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let output = payload
        .output_path
        .unwrap_or_else(|| derive_output_path(&payload.input_path, "trimmed", &ext));

    let args: Vec<&str> = vec![
        "-y", "-i", &payload.input_path,
        "-ss", &payload.start_time,
        "-to", &payload.end_time,
        "-c", "copy",
        &output,
    ];

    match run_ffmpeg_blocking(rt, &ffmpeg, &args) {
        Ok(_) => ok("Trimmed", serde_json::json!({ "output_path": output })),
        Err(e) => err(500, &e),
    }
}

// ─── Compress ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CompressRequest {
    pub input_path: String,
    pub crf: Option<u32>,        // 0-51, default 28 (lower = better quality)
    pub preset: Option<String>,  // ultrafast, fast, medium, slow, veryslow
    pub output_path: Option<String>,
}

/// POST /api/freecut/ffmpeg/compress
pub fn handle_ffmpeg_compress(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: CompressRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let ffmpeg = resolve_ffmpeg(data_dir);
    let crf_val = payload.crf.unwrap_or(28).min(51).to_string();
    let preset = payload.preset.unwrap_or_else(|| "medium".to_string());
    let output = payload
        .output_path
        .unwrap_or_else(|| derive_output_path(&payload.input_path, "compressed", "mp4"));

    let args: Vec<&str> = vec![
        "-y", "-i", &payload.input_path,
        "-c:v", "libx264", "-crf", &crf_val, "-preset", &preset,
        "-c:a", "aac", "-b:a", "128k",
        &output,
    ];

    match run_ffmpeg_blocking(rt, &ffmpeg, &args) {
        Ok(_) => ok("Compressed", serde_json::json!({ "output_path": output })),
        Err(e) => err(500, &e),
    }
}

// ─── Resize ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ResizeRequest {
    pub input_path: String,
    pub width: u32,
    pub height: u32,
    pub output_path: Option<String>,
}

/// POST /api/freecut/ffmpeg/resize
pub fn handle_ffmpeg_resize(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: ResizeRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let ffmpeg = resolve_ffmpeg(data_dir);
    let scale = format!("{}:{}", payload.width, payload.height);
    let ext = Path::new(&payload.input_path)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let label = format!("{}x{}", payload.width, payload.height);
    let output = payload
        .output_path
        .unwrap_or_else(|| derive_output_path(&payload.input_path, &label, &ext));

    let vf_filter = format!("scale={}", scale);

    let args: Vec<&str> = vec![
        "-y", "-i", &payload.input_path,
        "-vf", &vf_filter,
        "-c:a", "copy",
        &output,
    ];

    match run_ffmpeg_blocking(rt, &ffmpeg, &args) {
        Ok(_) => ok("Resized", serde_json::json!({ "output_path": output })),
        Err(e) => err(500, &e),
    }
}

// ─── Remove Audio ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RemoveAudioRequest {
    pub input_path: String,
    pub output_path: Option<String>,
}

/// POST /api/freecut/ffmpeg/remove-audio
pub fn handle_ffmpeg_remove_audio(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: RemoveAudioRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let ffmpeg = resolve_ffmpeg(data_dir);
    let ext = Path::new(&payload.input_path)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let output = payload
        .output_path
        .unwrap_or_else(|| derive_output_path(&payload.input_path, "noaudio", &ext));

    let args: Vec<&str> = vec![
        "-y", "-i", &payload.input_path,
        "-an", "-c:v", "copy",
        &output,
    ];

    match run_ffmpeg_blocking(rt, &ffmpeg, &args) {
        Ok(_) => ok("Audio removed", serde_json::json!({ "output_path": output })),
        Err(e) => err(500, &e),
    }
}

// ─── Create GIF ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GifRequest {
    pub input_path: String,
    pub start_time: Option<String>,
    pub duration: Option<String>,  // e.g. "5" for 5 seconds
    pub fps: Option<u32>,          // default 10
    pub width: Option<u32>,        // default 480
    pub output_path: Option<String>,
}

/// POST /api/freecut/ffmpeg/gif
pub fn handle_ffmpeg_gif(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: GifRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let ffmpeg = resolve_ffmpeg(data_dir);
    let fps = payload.fps.unwrap_or(10);
    let width = payload.width.unwrap_or(480);
    let vf = format!("fps={},scale={}:-1:flags=lanczos", fps, width);
    let output = payload
        .output_path
        .unwrap_or_else(|| derive_output_path(&payload.input_path, "clip", "gif"));

    let mut args: Vec<String> = vec!["-y".into()];
    if let Some(ref ss) = payload.start_time {
        args.push("-ss".into());
        args.push(ss.clone());
    }
    args.push("-i".into());
    args.push(payload.input_path.clone());
    if let Some(ref dur) = payload.duration {
        args.push("-t".into());
        args.push(dur.clone());
    }
    args.push("-vf".into());
    args.push(vf);
    args.push("-loop".into());
    args.push("0".into());
    args.push(output.clone());

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match run_ffmpeg_blocking(rt, &ffmpeg, &args_ref) {
        Ok(_) => ok("GIF created", serde_json::json!({ "output_path": output })),
        Err(e) => err(500, &e),
    }
}

// ─── Media Info (ffprobe) ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct MediaInfoRequest {
    pub input_path: String,
}

/// POST /api/freecut/ffmpeg/info
pub fn handle_ffmpeg_info(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: MediaInfoRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let ffprobe = resolve_ffprobe(data_dir);
    let ffprobe_str = ffprobe.to_string_lossy().to_string();
    let result = rt.block_on(async {
        let output = tokio::process::Command::new(&ffprobe_str)
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                &payload.input_path,
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            serde_json::from_str::<serde_json::Value>(&json_str)
                .map_err(|e| format!("Failed to parse ffprobe output: {}", e))
        } else {
            Err(format!(
                "ffprobe error: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    });

    match result {
        Ok(info) => ok("Media info", info),
        Err(e) => err(500, &e),
    }
}
