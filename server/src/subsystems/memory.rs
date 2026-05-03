use ai_launcher_core::memory::MemorySubstrate;
use ai_launcher_core::memory::types::{AgentId, MemorySource, SessionId};
use crate::response::{HttpResponse, json_resp, err, parse_body};
use std::sync::Arc;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

pub fn get_status(memory: &Arc<MemorySubstrate>) -> HttpResponse {
    let agent_id = AgentId(Uuid::nil());
    let mut stats = json!({
        "status": "active",
        "database": "sqlite",
        "agent": agent_id.0.to_string(),
    });

    if let Ok(sessions) = memory.session.list_sessions(agent_id) {
        stats["row_counts"] = json!({
            "sessions": sessions.len(),
        });
    }

    json_resp(200, &stats)
}

pub fn remember(req: &mut tiny_http::Request, memory: &Arc<MemorySubstrate>) -> HttpResponse {
    let body: serde_json::Value = match parse_body(req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let content = body["content"].as_str().unwrap_or_default();
    let agent_id = AgentId(Uuid::nil());
    let source = MemorySource::User; // Default to User for API
    let metadata = HashMap::new();

    match memory.semantic.remember(agent_id, content, source, "ephemeral", metadata) {
        Ok(_) => json_resp(200, &json!({"status": "remembered"})),
        Err(e) => err(500, &e.to_string()),
    }
}

pub fn recall(req: &mut tiny_http::Request, memory: &Arc<MemorySubstrate>) -> HttpResponse {
    let body: serde_json::Value = match parse_body(req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let query = body["query"].as_str().unwrap_or_default();
    let limit = body["limit"].as_u64().unwrap_or(5) as usize;

    match memory.semantic.recall(query, limit, None) {
        Ok(fragments) => json_resp(200, &json!({ "results": fragments })),
        Err(e) => err(500, &e.to_string()),
    }
}

pub fn list_sessions(memory: &Arc<MemorySubstrate>) -> HttpResponse {
    let agent_id = AgentId(Uuid::nil());
    match memory.session.list_sessions(agent_id) {
        Ok(sessions) => json_resp(200, &json!({ "sessions": sessions })),
        Err(e) => err(500, &e.to_string()),
    }
}

pub fn create_session(memory: &Arc<MemorySubstrate>) -> HttpResponse {
    let agent_id = AgentId(Uuid::nil());
    match memory.session.create_session(agent_id) {
        Ok(session) => json_resp(200, &json!(session)),
        Err(e) => err(500, &e.to_string()),
    }
}

pub fn get_session(id: &str, memory: &Arc<MemorySubstrate>) -> HttpResponse {
    let uuid = match Uuid::parse_str(id) {
        Ok(u) => u,
        Err(_) => return err(400, "Invalid UUID format"),
    };
    let session_id = SessionId(uuid);
    match memory.session.get_session(session_id) {
        Ok(Some(session)) => json_resp(200, &json!(session)),
        Ok(None) => err(404, "Session not found"),
        Err(e) => err(500, &e.to_string()),
    }
}

pub fn delete_session(id: &str, memory: &Arc<MemorySubstrate>) -> HttpResponse {
    let uuid = match Uuid::parse_str(id) {
        Ok(u) => u,
        Err(_) => return err(400, "Invalid UUID format"),
    };
    let session_id = SessionId(uuid);
    match memory.session.delete_session(session_id) {
        Ok(_) => json_resp(200, &json!({"status": "deleted"})),
        Err(e) => err(500, &e.to_string()),
    }
}

pub fn consolidate(memory: &Arc<MemorySubstrate>) -> HttpResponse {
    match memory.consolidation.consolidate() {
        Ok(report) => json_resp(200, &json!(report)),
        Err(e) => err(500, &e.to_string()),
    }
}
