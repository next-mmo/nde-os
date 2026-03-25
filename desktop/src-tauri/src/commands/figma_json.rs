//! Tauri IPC commands for the Figma JSON Render Engine.
//!
//! Exposes core Rust logic to the frontend via `#[tauri::command]`.
//! The frontend sends JSON strings, Rust does the heavy parsing/conversion,
//! and returns ready-to-render data.

use std::collections::{BTreeMap, HashMap};

use ai_launcher_core::figma_json::{
    converter, llm_prompt, style_resolver,
    types::FDocument,
};

/// Convert raw JSON (FDocument or Figma API response) into an FDocument.
///
/// Accepts either:
/// - A valid FDocument JSON (has `version` + `children`)
/// - A raw Figma REST API file response (has `document.children`)
///
/// Returns the FDocument as a JSON value.
#[tauri::command]
pub async fn convert_figma_json(json_str: String) -> Result<FDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // Try parsing as FDocument first
        if let Ok(doc) = serde_json::from_str::<FDocument>(&json_str) {
            return Ok(doc);
        }

        // Try parsing as raw Figma API response
        converter::convert_figma_file(&json_str, &HashMap::new(), 0)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Pre-compute CSS styles for an entire FDocument.
///
/// Returns a map of `node_id → css_inline_string` for every node in the tree.
/// The frontend applies these directly without any JS computation.
#[tauri::command]
pub async fn resolve_document_styles(
    json_str: String,
) -> Result<BTreeMap<String, String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let doc: FDocument = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
        Ok(style_resolver::resolve_document_styles(&doc))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Fetch a Figma file via REST API and convert to FDocument.
/// Network I/O + parsing all happens in Rust.
#[tauri::command]
pub async fn fetch_figma_file(
    file_key: String,
    token: String,
    page_index: Option<usize>,
) -> Result<FDocument, String> {
    converter::fetch_and_convert(&file_key, &token, page_index.unwrap_or(0))
        .await
        .map_err(|e| e.to_string())
}

/// Build LLM prompt for auto-generating FDocument JSON.
#[tauri::command]
pub fn build_figma_llm_prompt(description: String) -> llm_prompt::LLMPrompt {
    llm_prompt::build_llm_prompt(&description)
}

/// Get the sample FDocument for testing.
#[tauri::command]
pub fn get_figma_sample() -> FDocument {
    FDocument::sample()
}
