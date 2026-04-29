use std::path::Path;
use std::io::Write;
use tiny_http::Request;
use uuid::Uuid;

use ai_launcher_core::freecut::movie_dub::stt::SttEngine;
use ai_launcher_core::voice::runtime::VoiceRuntime;
use ai_launcher_core::media::ffmpeg::ensure_ffmpeg;

use crate::response::{err, ok, parse_multipart, HttpResponse};

fn extract_multipart(req: &mut Request) -> Result<(Vec<u8>, String), HttpResponse> {
    let content_type = req
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case("content-type"))
        .map(|h| h.value.as_str().to_string())
        .unwrap_or_default();

    let boundary = match content_type.split("boundary=").nth(1) {
        Some(b) => b.trim().to_string(),
        None => return Err(err(400, "Missing multipart boundary in Content-Type header")),
    };

    let mut body = Vec::new();
    if let Err(e) = req.as_reader().read_to_end(&mut body) {
        return Err(err(500, &format!("Failed to read request body: {e}")));
    }

    let fields = parse_multipart(&body, &boundary);
    let audio_bytes = fields.iter().find(|(n, _)| n == "audio").map(|(_, v)| v.clone());
    let model_size = fields
        .iter()
        .find(|(n, _)| n == "model_size")
        .and_then(|(_, v)| String::from_utf8(v.clone()).ok())
        .unwrap_or_else(|| "base".to_string());

    let audio_bytes = match audio_bytes {
        Some(b) => b,
        None => return Err(err(400, "Missing 'audio' field in multipart form")),
    };

    Ok((audio_bytes, model_size))
}

/// POST /api/transcript
/// Receives a multipart form data containing an 'audio' file and optional 'model_size'.
pub fn handle_transcript(req: &mut Request, data_dir: &Path, rt: &tokio::runtime::Runtime) -> HttpResponse {
    let (audio_bytes, model_size) = match extract_multipart(req) {
        Ok(res) => res,
        Err(resp) => return resp,
    };

    let voice_rt = VoiceRuntime::new(data_dir);
    if !voice_rt.is_installed() || voice_rt.resolve_tool("whisper").is_none() {
        return err(
            409,
            "Whisper CLI not found. Install 'openai-whisper' via Service Hub -> Voice Runtime.",
        );
    }

    let ffmpeg_path = match ensure_ffmpeg(data_dir) {
        Ok(bins) => bins.ffmpeg,
        Err(e) => return err(500, &format!("Failed to ensure ffmpeg: {e}")),
    };
    let whisper_path = voice_rt.resolve_tool("whisper");

    let work_dir = data_dir.join("temp").join("whisper");
    if let Err(e) = std::fs::create_dir_all(&work_dir) {
        return err(500, &format!("Failed to create work directory: {e}"));
    }

    let audio_path = work_dir.join(format!("{}.input", Uuid::new_v4()));
    if let Err(e) = std::fs::write(&audio_path, &audio_bytes) {
        return err(500, &format!("Failed to write audio data: {e}"));
    }

    let engine = SttEngine::new(&model_size, work_dir, ffmpeg_path, whisper_path);

    let res = rt.block_on(async {
        engine.transcribe(&audio_path, None).await
    });

    // Clean up temporary audio file
    let _ = std::fs::remove_file(&audio_path);

    match res {
        Ok(segments) => {
            let text = segments.iter().map(|s| s.source_text.clone()).collect::<Vec<_>>().join(" ");
            ok(
                "Transcription successful",
                serde_json::json!({
                    "text": text,
                    "segments": segments,
                }),
            )
        }
        Err(e) => err(500, &format!("Transcription failed: {e}")),
    }
}
