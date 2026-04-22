//! Server handlers for the standalone SRT translation service.
//!
//! Endpoints:
//! - `POST /api/translate/srt` — translate an SRT file
//! - `POST /api/translate/srt-multipart` — translate an uploaded SRT file
//! - `POST /api/translate/text` — translate a single text string
//! - `GET  /api/translate/providers` — list available providers

use crate::response::*;
use serde::Deserialize;
use std::path::Path;
use std::sync::{Arc, Mutex};

use ai_launcher_core::llm::manager::LlmManager;
use ai_launcher_core::translate::{Provider, TranslateConfig, TranslateService};

// ── Request/Response types ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TranslateSrtRequest {
    /// SRT content as a string (mutually exclusive with `srt_path`).
    #[serde(default)]
    srt_content: Option<String>,
    /// Path to SRT file on disk (mutually exclusive with `srt_content`).
    #[serde(default)]
    srt_path: Option<String>,
    /// Source language code (default: "en").
    #[serde(default = "default_source_lang")]
    source_lang: String,
    /// Target language code (default: "km").
    #[serde(default = "default_target_lang")]
    target_lang: String,
    /// Provider configuration.
    #[serde(default)]
    provider: Option<Provider>,
    /// Path to write the translated SRT file (optional).
    #[serde(default)]
    output_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TranslateTextRequest {
    /// Text to translate.
    text: String,
    /// Source language code (default: "en").
    #[serde(default = "default_source_lang")]
    source_lang: String,
    /// Target language code (default: "km").
    #[serde(default = "default_target_lang")]
    target_lang: String,
    /// Provider configuration.
    #[serde(default)]
    provider: Option<Provider>,
}

fn default_source_lang() -> String {
    "en".into()
}

fn default_target_lang() -> String {
    "km".into()
}

// ── Helper: resolve NdeAgent → direct LLM ───────────────────────────────────

/// When the user selects `nde_agent` as provider, we resolve it to a direct
/// `Llm` provider using the currently active LLM from the server's LlmManager.
/// This avoids HTTP loopback and the agent tool-calling loop that returns
/// empty responses.
fn resolve_provider(provider: Option<Provider>, llm_manager: &Arc<Mutex<LlmManager>>) -> Provider {
    match provider {
        Some(Provider::NdeAgent { .. }) | None => {
            // Try to resolve from the active LLM
            if let Ok(mgr) = llm_manager.lock() {
                let active_name = mgr.active_name().to_string();
                if let Some(config) = mgr.configs().iter().find(|c| c.name == active_name) {
                    let api_key = config.api_key.clone().or_else(|| {
                        config
                            .api_key_env
                            .as_ref()
                            .and_then(|env| std::env::var(env).ok())
                    });
                    return Provider::Llm {
                        provider: config.provider_type.clone(),
                        model: config.model.clone(),
                        api_key,
                        base_url: config.base_url.clone(),
                    };
                }
            }
            // Fallback if no LLM manager available — keep NdeAgent
            provider.unwrap_or_default()
        }
        Some(p) => p,
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/translate/srt-multipart — translate an uploaded SRT file.
pub fn handle_translate_srt_multipart(
    req: &mut tiny_http::Request,
    _data_dir: &Path,
    rt: &tokio::runtime::Runtime,
    llm_manager: &Arc<Mutex<LlmManager>>,
) -> HttpResponse {
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

    let fields = crate::response::parse_multipart(&body, &boundary);

    let srt_content = match fields.iter().find(|(n, _)| n == "file").map(|(_, v)| v.clone()) {
        Some(b) => match String::from_utf8(b) {
            Ok(s) => s,
            Err(_) => return err(400, "SRT file contains invalid UTF-8"),
        },
        None => return err(400, "Missing 'file' field in multipart form"),
    };

    let source_lang = fields
        .iter()
        .find(|(n, _)| n == "source_lang")
        .and_then(|(_, v)| String::from_utf8(v.clone()).ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "en".to_string());

    let target_lang = fields
        .iter()
        .find(|(n, _)| n == "target_lang")
        .and_then(|(_, v)| String::from_utf8(v.clone()).ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "km".to_string());

    let provider: Option<Provider> = fields
        .iter()
        .find(|(n, _)| n == "provider")
        .and_then(|(_, v)| String::from_utf8(v.clone()).ok())
        .and_then(|s| serde_json::from_str(&s).ok());

    let resolved = resolve_provider(provider, llm_manager);

    let config = TranslateConfig {
        provider: resolved,
        source_lang,
        target_lang,
        ..Default::default()
    };

    let service = match TranslateService::new(config) {
        Ok(s) => s,
        Err(e) => return err(500, &format!("Failed to create translation service: {e:#}")),
    };

    let result = match rt.block_on(service.translate_srt(&srt_content)) {
        Ok(r) => r,
        Err(e) => return err(500, &format!("Translation failed: {e:#}")),
    };

    ok(
        &format!("{} cues translated via {}", result.cue_count, result.provider),
        serde_json::json!(result),
    )
}

/// POST /api/translate/srt — translate an SRT file (JSON payload).
pub fn handle_translate_srt(
    req: &mut tiny_http::Request,
    _data_dir: &Path,
    rt: &tokio::runtime::Runtime,
    llm_manager: &Arc<Mutex<LlmManager>>,
) -> HttpResponse {
    let payload: TranslateSrtRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    // Load SRT content from either inline content or file path.
    let srt_content = match (&payload.srt_content, &payload.srt_path) {
        (Some(content), _) if !content.is_empty() => content.clone(),
        (_, Some(path)) if !path.is_empty() => {
            let path = std::path::Path::new(path);
            if !path.exists() {
                return err(404, &format!("SRT file not found: {}", path.display()));
            }
            match std::fs::read_to_string(path) {
                Ok(content) => content,
                Err(e) => return err(500, &format!("Failed to read SRT file: {e}")),
            }
        }
        _ => return err(400, "Either 'srt_content' or 'srt_path' must be provided"),
    };

    let resolved = resolve_provider(payload.provider, llm_manager);

    // Build config.
    let config = TranslateConfig {
        provider: resolved,
        source_lang: payload.source_lang,
        target_lang: payload.target_lang,
        ..Default::default()
    };

    // Create service and translate.
    let service = match TranslateService::new(config) {
        Ok(s) => s,
        Err(e) => return err(500, &format!("Failed to create translation service: {e:#}")),
    };

    let result = match rt.block_on(service.translate_srt(&srt_content)) {
        Ok(r) => r,
        Err(e) => return err(500, &format!("Translation failed: {e:#}")),
    };

    // Optionally write to disk.
    if let Some(output_path) = &payload.output_path {
        if let Some(parent) = std::path::Path::new(output_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(output_path, &result.srt_content) {
            return err(500, &format!("Failed to write output SRT: {e}"));
        }
    }

    ok(
        &format!("{} cues translated via {}", result.cue_count, result.provider),
        serde_json::json!(result),
    )
}

/// POST /api/translate/text — translate a single text string.
pub fn handle_translate_text(
    req: &mut tiny_http::Request,
    _data_dir: &Path,
    rt: &tokio::runtime::Runtime,
    llm_manager: &Arc<Mutex<LlmManager>>,
) -> HttpResponse {
    let payload: TranslateTextRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if payload.text.is_empty() {
        return err(400, "Empty 'text' field");
    }

    let resolved = resolve_provider(payload.provider, llm_manager);

    let config = TranslateConfig {
        provider: resolved,
        source_lang: payload.source_lang.clone(),
        target_lang: payload.target_lang.clone(),
        ..Default::default()
    };

    let service = match TranslateService::new(config) {
        Ok(s) => s,
        Err(e) => return err(500, &format!("Failed to create translation service: {e:#}")),
    };

    let translated = match rt.block_on(service.translate_text(&payload.text)) {
        Ok(t) => t,
        Err(e) => return err(500, &format!("Translation failed: {e:#}")),
    };

    ok(
        "Text translated",
        serde_json::json!({
            "original": payload.text,
            "translated": translated,
            "source_lang": payload.source_lang,
            "target_lang": payload.target_lang,
        }),
    )
}

/// GET /api/translate/providers — list available translation providers.
pub fn handle_list_providers() -> HttpResponse {
    let providers = TranslateService::available_providers();
    ok(
        &format!("{} providers available", providers.len()),
        serde_json::json!(providers),
    )
}
