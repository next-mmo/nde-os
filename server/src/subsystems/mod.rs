/// Channel, MCP, Skills, Knowledge, Memory handlers.
/// These expose core subsystem data to the desktop UI.
use crate::response::*;

pub mod downloads;
pub mod freecut;
pub mod kfa;
pub mod translate;
pub mod whisper;
pub mod memory;

use serde_json::Value;
use std::fs;
use std::path::Path;

// ── Knowledge entity helper ─────────────────────────────────────────────────

/// Convert a core Entity to a JSON value.
/// Used by both list_knowledge and search_knowledge to avoid duplication.
fn entity_to_json(e: &ai_launcher_core::memory::types::Entity) -> serde_json::Value {
    serde_json::json!({
        "id": e.id,
        "key": e.name,
        "value": e.properties,
        "category": e.entity_type,
    })
}

// ── Channels ────────────────────────────────────────────────────────────────

/// GET /api/channels — list registered channels + status.
/// Reads live state from the Telegram gateway and config for others.
pub fn list_channels(
    data_dir: &Path,
    tg_state: &std::sync::Arc<crate::gateway::GatewayState>,
) -> HttpResponse {
    let tg_running = tg_state.running.load(std::sync::atomic::Ordering::SeqCst);
    let tg_received = tg_state
        .messages_received
        .load(std::sync::atomic::Ordering::Relaxed);
    let tg_sent = tg_state
        .messages_sent
        .load(std::sync::atomic::Ordering::Relaxed);

    let mut dc_running = false;
    let mut sl_running = false;
    let mut tg_allowed_users: Vec<i64> = Vec::new();

    let config_path = data_dir.join("channels.json");
    if let Ok(config_str) = fs::read_to_string(&config_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_str) {
            if let Some(dc) = config.get("discord") {
                if dc.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false)
                    && dc.get("token").and_then(|t| t.as_str()).unwrap_or("") != ""
                {
                    dc_running = true;
                }
            }
            if let Some(sl) = config.get("slack") {
                if sl.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false)
                    && sl.get("token").and_then(|t| t.as_str()).unwrap_or("") != ""
                {
                    sl_running = true;
                }
            }
            if let Some(tg) = config.get("telegram") {
                tg_allowed_users = tg
                    .get("allowed_users")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
                    .unwrap_or_default();
            }
        }
    } else {
        // No channels.json — no gateways configured
        dc_running = false;
        sl_running = false;
    }

    let default_channels = serde_json::json!([
        {
            "name": "rest-api",
            "channel_type": "rest",
            "is_running": true,
            "messages_received": 0,
            "messages_sent": 0
        },
        {
            "name": "telegram-bot",
            "channel_type": "telegram",
            "is_running": tg_running,
            "messages_received": tg_received,
            "messages_sent": tg_sent,
            "allowed_users": tg_allowed_users
        },
        {
            "name": "discord-bot",
            "channel_type": "discord",
            "is_running": dc_running,
            "messages_received": 0,
            "messages_sent": 0
        },
        {
            "name": "slack-bot",
            "channel_type": "slack",
            "is_running": sl_running,
            "messages_received": 0,
            "messages_sent": 0
        }
    ]);
    ok("Channel list", default_channels)
}

/// POST /api/channels/{name}/configure
pub fn configure_channel(req: &mut tiny_http::Request, data_dir: &Path) -> HttpResponse {
    let payload: Value = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let channel_type = payload
        .get("channel_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let enabled = payload
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let token = payload.get("token").and_then(|v| v.as_str()).unwrap_or("");
    let allowed_users: Vec<i64> = payload
        .get("allowed_users")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
        .unwrap_or_default();

    if channel_type.is_empty() {
        return err(400, "Missing channel_type");
    }

    let config_path = data_dir.join("channels.json");
    let mut root_config = match fs::read_to_string(&config_path) {
        Ok(content) => {
            serde_json::from_str::<serde_json::Value>(&content).unwrap_or(serde_json::json!({}))
        }
        Err(_) => serde_json::json!({}),
    };

    // Encrypt the token before persisting, or preserve existing if not provided
    let encrypted_token = if !token.is_empty() && enabled {
        match ai_launcher_core::secrets::encrypt_token(token, data_dir) {
            Ok(enc) => enc,
            Err(e) => return err(500, &format!("Failed to encrypt token: {}", e)),
        }
    } else if token.is_empty() && enabled {
        root_config
            .get(channel_type)
            .and_then(|ch| ch.get("token"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    };

    if let Some(obj) = root_config.as_object_mut() {
        let mut channel_obj = serde_json::json!({
            "enabled": enabled,
            "token": encrypted_token
        });
        if channel_type == "telegram" {
            channel_obj["allowed_users"] = serde_json::json!(allowed_users);
        }
        obj.insert(channel_type.to_string(), channel_obj);
    }

    let _ = fs::write(
        &config_path,
        serde_json::to_string_pretty(&root_config).unwrap_or_default(),
    );

    // SECURITY: Do NOT set gateway tokens as process env vars.
    // They would be inherit by LLM-spawned subprocesses (shell_exec) and
    // could be exfiltrated via prompt injection. Each gateway reads its
    // token directly from channels.json on startup instead.
    if enabled {
        tracing::info!(
            channel = channel_type,
            "Channel configured (token encrypted at rest)"
        );
    }

    ok("Channel configured", serde_json::json!({ "success": true }))
}

// ── MCP ─────────────────────────────────────────────────────────────────────

/// GET /api/mcp/tools — list all MCP-exposed tools from core builtin server.
pub fn list_mcp_tools() -> HttpResponse {
    let tools = ai_launcher_core::mcp::builtin::builtin_tool_definitions();
    ok(&format!("{} MCP tools", tools.len()), tools)
}

/// GET /api/mcp/servers — list MCP server connections.
pub fn list_mcp_servers() -> HttpResponse {
    let servers = ai_launcher_core::mcp::builtin::builtin_server_info();
    ok("MCP servers", servers)
}

// ── Agent Tools ─────────────────────────────────────────────────────────────

/// GET /api/agent/tools — list all built-in agent tools from the tool registry.
pub fn list_agent_tools() -> HttpResponse {
    let registry = ai_launcher_core::tools::builtin::default_registry();
    let defs: Vec<serde_json::Value> = registry
        .definitions()
        .into_iter()
        .map(|d| {
            serde_json::json!({
                "name": d.name,
                "description": d.description,
                "parameters": d.parameters,
            })
        })
        .collect();
    ok(&format!("{} agent tools", defs.len()), defs)
}

// ── Skills ──────────────────────────────────────────────────────────────────

/// GET /api/skills — list available skills from real SkillLoader.
pub fn list_skills() -> HttpResponse {
    let mut search_paths = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        search_paths.push(cwd.join(".agents").join("skills"));
        search_paths.push(cwd.join(".agent").join("skills"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            search_paths.push(exe_dir.join(".agents").join("skills"));
        }
    }

    let loader = ai_launcher_core::skills::SkillLoader::new(search_paths);
    match loader.discover() {
        Ok(skills) => {
            let entries: Vec<serde_json::Value> = skills
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "description": s.description,
                        "path": s.path,
                        "triggers": s.triggers,
                    })
                })
                .collect();
            ok(&format!("{} skills", entries.len()), entries)
        }
        Err(e) => {
            eprintln!("Skill discovery error: {}", e);
            ok("0 skills", serde_json::json!([]))
        }
    }
}

// ── Knowledge ───────────────────────────────────────────────────────────────

/// GET /api/knowledge — list all knowledge entries from real KnowledgeGraph.
pub fn list_knowledge(state: &crate::router::AppState) -> HttpResponse {
    let mut all_entities = Vec::new();
    let kg = &state.memory_substrate.knowledge;

    // Pattern match to get all entities (or implement a list method if we had one)
    // For now we'll simulate the search by querying all nodes up to a certain limit
    if let Ok(matches) = kg.query_graph(ai_launcher_core::memory::types::GraphPattern {
        source: None,
        relation: None,
        target: None,
        max_depth: 1,
    }) {
        for m in matches {
            if !all_entities.iter().any(|e: &ai_launcher_core::memory::types::Entity| e.id == m.source.id) {
                all_entities.push(m.source);
            }
            if !all_entities.iter().any(|e: &ai_launcher_core::memory::types::Entity| e.id == m.target.id) {
                all_entities.push(m.target);
            }
        }
    }

    let entries: Vec<serde_json::Value> = all_entities.iter().map(entity_to_json).collect();
    ok(&format!("{} knowledge entries", entries.len()), entries)
}

/// GET /api/knowledge/search?q=... — search knowledge.
pub fn search_knowledge(query: &str, state: &crate::router::AppState) -> HttpResponse {
    let kg = &state.memory_substrate.knowledge;
    // We do a source OR target match on the query name
    match kg.query_graph(ai_launcher_core::memory::types::GraphPattern {
        source: Some(query.to_string()),
        relation: None,
        target: None,
        max_depth: 1,
    }) {
        Ok(matches) => {
            let mut all_entities = Vec::new();
            for m in matches {
                if !all_entities.iter().any(|e: &ai_launcher_core::memory::types::Entity| e.id == m.source.id) {
                    all_entities.push(m.source);
                }
                if !all_entities.iter().any(|e: &ai_launcher_core::memory::types::Entity| e.id == m.target.id) {
                    all_entities.push(m.target);
                }
            }
            let entries: Vec<serde_json::Value> = all_entities.iter().map(entity_to_json).collect();
            ok(
                &format!("Knowledge search: {} ({} results)", query, entries.len()),
                entries,
            )
        }
        Err(e) => {
            eprintln!("Knowledge search error: {}", e);
            ok(
                &format!("Knowledge search: {}", query),
                serde_json::json!([]),
            )
        }
    }
}

// ── Memory ──────────────────────────────────────────────────────────────────

/// GET /api/memory — list all memory entries from real MemoryManager.
pub fn list_memory(state: &crate::router::AppState) -> HttpResponse {
    let kv = &state.memory_substrate.structured;
    // Note: We need a default agent ID for global memory. Let's use a zero UUID.
    let global_agent = ai_launcher_core::memory::types::AgentId(uuid::Uuid::nil());
    
    match kv.list_kv(global_agent) {
        Ok(pairs) => {
            let mut entries = Vec::new();
            for (key, value) in pairs {
                entries.push(serde_json::json!({
                    "key": key,
                    "value": value,
                }));
            }
            ok(&format!("{} memory entries", entries.len()), entries)
        }
        Err(e) => {
            eprintln!("Memory list error: {}", e);
            ok("0 memory entries", serde_json::json!([]))
        }
    }
}

/// GET /api/memory/{key} — get a specific memory value.
pub fn get_memory(key: &str, state: &crate::router::AppState) -> HttpResponse {
    let kv = &state.memory_substrate.structured;
    let global_agent = ai_launcher_core::memory::types::AgentId(uuid::Uuid::nil());
    
    match kv.get(global_agent, key) {
        Ok(Some(value)) => ok(
            &format!("Memory: {}", key),
            serde_json::json!({
                "key": key,
                "value": value,
            }),
        ),
        Ok(None) => err(404, &format!("Memory key not found: {}", key)),
        Err(e) => err(500, &format!("Memory read error: {}", e)),
    }
}

/// POST /api/memory/remember
pub fn remember_memory(req: &mut tiny_http::Request, state: &crate::router::AppState) -> HttpResponse {
    let payload: serde_json::Value = match crate::response::parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    
    let content = payload.get("content").and_then(|v| v.as_str()).unwrap_or("");
    if content.is_empty() {
        return err(400, "Missing 'content' field");
    }
    
    let scope = payload.get("scope").and_then(|v| v.as_str()).unwrap_or("global");
    
    let agent_id = ai_launcher_core::memory::types::AgentId(uuid::Uuid::nil());
    let source = ai_launcher_core::memory::types::MemorySource::User; 
    let metadata = std::collections::HashMap::new();
    
    match state.memory_substrate.semantic.remember(agent_id, content, source, scope, metadata) {
        Ok(id) => ok("Memory stored", serde_json::json!({ "id": id.0.to_string() })),
        Err(e) => err(500, &format!("Failed to store memory: {}", e)),
    }
}

/// POST /api/memory/recall
pub fn recall_memory(req: &mut tiny_http::Request, state: &crate::router::AppState) -> HttpResponse {
    let payload: serde_json::Value = match crate::response::parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    
    let query = payload.get("query").and_then(|v| v.as_str()).unwrap_or("");
    let limit = payload.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    
    match state.memory_substrate.semantic.recall(query, limit, None) {
        Ok(fragments) => ok("Memories recalled", serde_json::json!(fragments)),
        Err(e) => err(500, &format!("Failed to recall memories: {}", e)),
    }
}

/// GET /api/memory/sessions
pub fn list_sessions(state: &crate::router::AppState) -> HttpResponse {
    let agent_id = ai_launcher_core::memory::types::AgentId(uuid::Uuid::nil());
    match state.memory_substrate.session.list_sessions(agent_id) {
        Ok(sessions) => ok("Sessions listed", serde_json::json!(sessions)),
        Err(e) => err(500, &format!("Failed to list sessions: {}", e)),
    }
}

/// POST /api/memory/sessions
pub fn create_session(state: &crate::router::AppState) -> HttpResponse {
    let agent_id = ai_launcher_core::memory::types::AgentId(uuid::Uuid::nil());
    match state.memory_substrate.session.create_session(agent_id) {
        Ok(session) => ok("Session created", serde_json::json!({ "id": session.id.0.to_string() })),
        Err(e) => err(500, &format!("Failed to create session: {}", e)),
    }
}

/// GET /api/memory/sessions/{id}
pub fn get_session(id: &str, state: &crate::router::AppState) -> HttpResponse {
    if let Ok(session_uuid) = uuid::Uuid::parse_str(id) {
        let session_id = ai_launcher_core::memory::types::SessionId(session_uuid);
        match state.memory_substrate.session.get_session(session_id) {
            Ok(Some(session)) => ok("Session loaded", serde_json::json!(session)),
            Ok(None) => err(404, "Session not found"),
            Err(e) => err(500, &format!("Failed to load session: {}", e)),
        }
    } else {
        err(400, "Invalid session ID format")
    }
}

/// DELETE /api/memory/sessions/{id}
pub fn delete_session(id: &str, state: &crate::router::AppState) -> HttpResponse {
    if let Ok(session_uuid) = uuid::Uuid::parse_str(id) {
        let session_id = ai_launcher_core::memory::types::SessionId(session_uuid);
        match state.memory_substrate.session.delete_session(session_id) {
            Ok(_) => ok("Session deleted", serde_json::json!({ "success": true })),
            Err(e) => err(500, &format!("Failed to delete session: {}", e)),
        }
    } else {
        err(400, "Invalid session ID format")
    }
}

/// POST /api/memory/consolidate
pub fn consolidate_memory(state: &crate::router::AppState) -> HttpResponse {
    match state.memory_substrate.consolidation.consolidate() {
        Ok(report) => ok("Consolidation complete", serde_json::json!(report)),
        Err(e) => err(500, &format!("Failed to consolidate memory: {}", e)),
    }
}

/// GET /api/memory/status
pub fn memory_status(state: &crate::router::AppState) -> HttpResponse {
    let agent_id = ai_launcher_core::memory::types::AgentId(uuid::Uuid::nil());
    // Get basic stats to show
    let mut stats = serde_json::json!({
        "status": "online",
        "agent": agent_id.0.to_string(),
    });
    
    if let Ok(sessions) = state.memory_substrate.session.list_sessions(agent_id) {
        stats["total_sessions"] = serde_json::json!(sessions.len());
    }
    
    ok("Memory status", stats)
}
