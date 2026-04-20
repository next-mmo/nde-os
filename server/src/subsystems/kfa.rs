//! KFA (Khmer Forced Aligner) server endpoints.
//!
//! POST /api/kfa/align — multipart form: `audio` (WAV bytes) + `text` (Khmer transcript)
//! POST /api/kfa/align-json — JSON body: `{ "audio_base64": "...", "text": "..." }`

use std::io::Read;
use std::sync::{Arc, Mutex};

use ai_launcher_core::kfa::audio::load_wav_mono_16k_bytes;
use ai_launcher_core::kfa::{Alignment, AlignmentSession};
use tiny_http::Request;

use crate::response::{err, ok, HttpResponse};

/// Shared lazy-initialized alignment session (model load is expensive).
static SESSION: std::sync::OnceLock<Arc<Mutex<AlignmentSession>>> = std::sync::OnceLock::new();

fn get_session() -> Result<Arc<Mutex<AlignmentSession>>, String> {
    SESSION
        .get_or_try_init(|| {
            AlignmentSession::new(false)
                .map(|s| Arc::new(Mutex::new(s)))
                .map_err(|e| e.to_string())
        })
        .map(|s| s.clone())
        .map_err(|e| e.to_string())
}

/// Parse a simple multipart/form-data body.
/// Extracts fields by name into (name, content_bytes).
fn parse_multipart(body: &[u8], boundary: &str) -> Vec<(String, Vec<u8>)> {
    let delimiter = format!("--{}", boundary);
    let body_str = String::from_utf8_lossy(body);
    let mut fields = Vec::new();

    for part in body_str.split(&delimiter) {
        // Find the double CRLF that separates headers from body
        if let Some(header_end) = part.find("\r\n\r\n") {
            let headers = &part[..header_end];
            // Extract the raw bytes after the header block
            let body_start = header_end + 4;
            let raw_part = &body[..];

            // Locate the part bytes in the original body
            let part_bytes = part[body_start..].trim_end_matches("\r\n").as_bytes().to_vec();

            // Extract Content-Disposition: form-data; name="..."
            if let Some(cd_line) = headers.lines().find(|l| l.starts_with("Content-Disposition")) {
                if let Some(name_start) = cd_line.find("name=\"") {
                    let after = &cd_line[name_start + 6..];
                    if let Some(name_end) = after.find('"') {
                        let field_name = after[..name_end].to_string();
                        fields.push((field_name, part_bytes));
                    }
                }
            }
        }
    }
    fields
}

/// POST /api/kfa/align — multipart form upload (audio WAV + text transcript).
pub fn handle_align_multipart(req: &mut Request) -> HttpResponse {
    // Parse content-type to get boundary
    let content_type = req
        .headers()
        .iter()
        .find(|h| h.field.as_str().to_lowercase() == "content-type")
        .map(|h| h.value.as_str().to_string())
        .unwrap_or_default();

    let boundary = if let Some(b) = content_type.split("boundary=").nth(1) {
        b.trim().to_string()
    } else {
        return err(400, "Missing multipart boundary in Content-Type header");
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
    // Load audio
    let (samples, sr) = match load_wav_mono_16k_bytes(audio_bytes) {
        Ok(v) => v,
        Err(e) => return err(400, &format!("Failed to decode WAV: {e}")),
    };

    // Get or init session
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
