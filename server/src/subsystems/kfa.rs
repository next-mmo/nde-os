//! KFA (Khmer Forced Aligner) server endpoints.
//!
//! POST /api/kfa/align      — multipart/form-data: `audio` (WAV) + `text` (Khmer)
//! POST /api/kfa/align-json — JSON body: `{ "audio_base64": "...", "text": "..." }`

use std::io::Read;
use std::sync::{Arc, Mutex};

use ai_launcher_core::kfa::audio::load_wav_mono_16k_bytes;
use ai_launcher_core::kfa::{Alignment, AlignmentSession};
use once_cell::sync::OnceCell;
use tiny_http::Request;

use crate::response::{err, ok, HttpResponse};

/// Shared lazy-initialized alignment session (model load is expensive).
static SESSION: OnceCell<Arc<Mutex<AlignmentSession>>> = OnceCell::new();

fn get_session() -> Result<Arc<Mutex<AlignmentSession>>, String> {
    match SESSION.get_or_try_init(|| -> Result<_, String> {
        AlignmentSession::new(false)
            .map(|s| Arc::new(Mutex::new(s)))
            .map_err(|e| e.to_string())
    }) {
        Ok(arc) => Ok(Arc::clone(arc)),
        Err(e) => Err(e.clone()),
    }
}

/// Parse a simple multipart/form-data body.
/// Returns a list of `(field_name, field_bytes)` pairs.
fn parse_multipart(body: &[u8], boundary: &str) -> Vec<(String, Vec<u8>)> {
    let delimiter = format!("--{}", boundary);
    let body_str = String::from_utf8_lossy(body);
    let mut fields = Vec::new();

    for part in body_str.split(&delimiter as &str) {
        if let Some(header_end) = part.find("\r\n\r\n") {
            let headers = &part[..header_end];
            let body_start = header_end + 4;
            let part_bytes = part[body_start..].trim_end_matches("\r\n").as_bytes().to_vec();

            if let Some(cd_line) = headers.lines().find(|l| l.starts_with("Content-Disposition")) {
                if let Some(name_start) = cd_line.find("name=\"") {
                    let after = &cd_line[name_start + 6..];
                    if let Some(name_end) = after.find('"') {
                        fields.push((after[..name_end].to_string(), part_bytes));
                    }
                }
            }
        }
    }
    fields
}

/// POST /api/kfa/align — multipart form upload (audio WAV + text transcript).
pub fn handle_align_multipart(req: &mut Request) -> HttpResponse {
    let content_type = req
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case("content-type"))
        .map(|h| h.value.as_str().to_string())
        .unwrap_or_default();

    let boundary = match content_type.split("boundary=").nth(1) {
        Some(b) => b.trim().to_string(),
        None => return err(400, "Missing multipart boundary in Content-Type header"),
    };

    let mut body = Vec::new();
    if let Err(e) = req.as_reader().read_to_end(&mut body) {
        return err(500, &format!("Failed to read request body: {e}"));
    }

    let fields = parse_multipart(&body, &boundary);
    let audio_bytes = fields.iter().find(|(n, _)| n == "audio").map(|(_, v)| v.clone());
    let text = fields
        .iter()
        .find(|(n, _)| n == "text")
        .and_then(|(_, v)| String::from_utf8(v.clone()).ok())
        .unwrap_or_default();

    let audio_bytes = match audio_bytes {
        Some(b) => b,
        None => return err(400, "Missing 'audio' field in multipart form"),
    };
    if text.trim().is_empty() {
        return err(400, "Missing 'text' field in multipart form");
    }

    run_alignment(&audio_bytes, &text)
}

/// POST /api/kfa/align-json — JSON body: `{ "audio_base64": "...", "text": "..." }`.
pub fn handle_align_json(req: &mut Request) -> HttpResponse {
    let mut body = String::new();
    if let Err(e) = req.as_reader().read_to_string(&mut body) {
        return err(500, &format!("Failed to read body: {e}"));
    }
    let v: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return err(400, &format!("Invalid JSON: {e}")),
    };

    let audio_b64 = v["audio_base64"].as_str().unwrap_or("");
    let text = v["text"].as_str().unwrap_or("").to_string();

    if audio_b64.is_empty() {
        return err(400, "Missing 'audio_base64' field");
    }
    if text.trim().is_empty() {
        return err(400, "Missing 'text' field");
    }

    use base64::Engine;
    let audio_bytes = match base64::engine::general_purpose::STANDARD.decode(audio_b64) {
        Ok(b) => b,
        Err(e) => return err(400, &format!("Invalid base64 in 'audio_base64': {e}")),
    };

    run_alignment(&audio_bytes, &text)
}

fn run_alignment(audio_bytes: &[u8], text: &str) -> HttpResponse {
    let (samples, sr) = match load_wav_mono_16k_bytes(audio_bytes) {
        Ok(v) => v,
        Err(e) => return err(400, &format!("Failed to decode WAV: {e}")),
    };

    let session_arc = match get_session() {
        Ok(s) => s,
        Err(e) => return err(500, &format!("KFA session init failed: {e}")),
    };

    let results: Vec<Alignment> = {
        let mut session = match session_arc.lock() {
            Ok(s) => s,
            Err(_) => return err(500, "KFA session mutex poisoned"),
        };
        match session.align(&samples, sr, text, None) {
            Ok(r) => r,
            Err(e) => return err(500, &format!("Alignment failed: {e}")),
        }
    };

    ok(
        &format!("{} segments aligned", results.len()),
        serde_json::json!({ "segments": results }),
    )
}
