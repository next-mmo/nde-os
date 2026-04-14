use crate::response::{err, ok, parse_body, HttpResponse};
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

    let opts = DubVideoOptions {
        input_path,
        output_path: output_path.clone(),
        source_lang: Lang::En,
        dual_audio: false,
        generate_subtitles: true,
        burn_subtitles: false,
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
