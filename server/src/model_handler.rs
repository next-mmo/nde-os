/// LLM model management handlers.
use ai_launcher_core::llm::manager::LlmManager;
use std::io::Cursor;
use std::sync::Mutex;
use tiny_http::{Request, Response};

use crate::response::*;

/// GET /api/models — list providers.
pub fn list_models(manager: &Mutex<LlmManager>) -> Response<Cursor<Vec<u8>>> {
    match manager.lock() {
        Ok(m) => ok("LLM providers", m.status()),
        Err(_) => err(500, "LLM manager lock failed"),
    }
}

/// GET /api/models/active — current active model.
pub fn active_model(manager: &Mutex<LlmManager>) -> Response<Cursor<Vec<u8>>> {
    match manager.lock() {
        Ok(m) => ok("Active provider", m.active_name()),
        Err(_) => err(500, "LLM manager lock failed"),
    }
}

/// POST /api/models/switch — switch active provider.
pub fn switch_model(req: &mut Request, manager: &Mutex<LlmManager>) -> Response<Cursor<Vec<u8>>> {
    let body = match read_body(req) {
        Some(b) => b,
        None => return err(400, "Missing request body"),
    };

    #[derive(serde::Deserialize)]
    struct SwitchReq {
        name: String,
    }

    let switch: SwitchReq = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
    };

    match manager.lock() {
        Ok(mut m) => match m.switch(&switch.name) {
            Ok(()) => ok(&format!("Switched to '{}'", switch.name), switch.name),
            Err(e) => err(400, &e.to_string()),
        },
        Err(_) => err(500, "LLM manager lock failed"),
    }
}
