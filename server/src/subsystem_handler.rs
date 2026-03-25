/// Channel, MCP, Skills, Knowledge, Memory handlers.
/// These expose core subsystem data to the desktop UI.
/// Skills / Knowledge / Memory now use real core modules with OpenViking fallback.
use std::io::Cursor;
use tiny_http::Response;

use crate::response::*;

use std::fs;
use std::path::Path;
use serde_json::Value;

// ── Channels ────────────────────────────────────────────────────────────────

/// GET /api/channels — list registered channels + status.
/// Reads from channels.json configuration or env vars.
pub fn list_channels(data_dir: &Path) -> Response<Cursor<Vec<u8>>> {
    let mut tg_running = false;
    let mut dc_running = false;
    let mut sl_running = false;
    
    let config_path = data_dir.join("channels.json");
    if let Ok(config_str) = fs::read_to_string(&config_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_str) {
            if let Some(tg) = config.get("telegram") {
                if tg.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false) && tg.get("token").and_then(|t| t.as_str()).unwrap_or("") != "" {
                    tg_running = true;
                }
            }
            if let Some(dc) = config.get("discord") {
                if dc.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false) && dc.get("token").and_then(|t| t.as_str()).unwrap_or("") != "" {
                    dc_running = true;
                }
            }
            if let Some(sl) = config.get("slack") {
                if sl.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false) && sl.get("token").and_then(|t| t.as_str()).unwrap_or("") != "" {
                    sl_running = true;
                }
            }
        }
    } else {
        tg_running = std::env::var("TELEGRAM_BOT_TOKEN").is_ok();
        dc_running = std::env::var("DISCORD_BOT_TOKEN").is_ok();
        sl_running = std::env::var("SLACK_BOT_TOKEN").is_ok();
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
            "messages_received": 0,
            "messages_sent": 0
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
pub fn configure_channel(req: &mut tiny_http::Request, data_dir: &Path) -> Response<Cursor<Vec<u8>>> {
    let mut content = String::new();
    if req.as_reader().read_to_string(&mut content).is_err() {
        return err(400, "Invalid request body");
    }
    
    let payload: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
    };
    
    let channel_type = payload.get("channel_type").and_then(|v| v.as_str()).unwrap_or("");
    let enabled = payload.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let token = payload.get("token").and_then(|v| v.as_str()).unwrap_or("");
    
    if channel_type.is_empty() {
        return err(400, "Missing channel_type");
    }
    
    let config_path = data_dir.join("channels.json");
    let mut root_config = match fs::read_to_string(&config_path) {
        Ok(content) => serde_json::from_str::<serde_json::Value>(&content).unwrap_or(serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    };
    
    if let Some(obj) = root_config.as_object_mut() {
        obj.insert(channel_type.to_string(), serde_json::json!({
            "enabled": enabled,
            "token": token
        }));
    }
    
    let _ = fs::write(&config_path, serde_json::to_string_pretty(&root_config).unwrap_or_default());
    
    // Apply to current process env vars so they work immediately
    if enabled && !token.is_empty() {
        match channel_type {
            "telegram" => std::env::set_var("TELEGRAM_BOT_TOKEN", token),
            "discord" => std::env::set_var("DISCORD_BOT_TOKEN", token),
            "slack" => std::env::set_var("SLACK_BOT_TOKEN", token),
            _ => {}
        }
    } else {
        match channel_type {
            "telegram" => std::env::remove_var("TELEGRAM_BOT_TOKEN"),
            "discord" => std::env::remove_var("DISCORD_BOT_TOKEN"),
            "slack" => std::env::remove_var("SLACK_BOT_TOKEN"),
            _ => {}
        }
    }
    
    ok("Channel configured", serde_json::json!({ "success": true }))
}

// ── MCP ─────────────────────────────────────────────────────────────────────

/// GET /api/mcp/tools — list all MCP-exposed tools from core builtin server.
pub fn list_mcp_tools() -> Response<Cursor<Vec<u8>>> {
    let tools = ai_launcher_core::mcp::builtin::builtin_tool_definitions();
    ok(&format!("{} MCP tools", tools.len()), tools)
}

/// GET /api/mcp/servers — list MCP server connections.
pub fn list_mcp_servers() -> Response<Cursor<Vec<u8>>> {
    let servers = ai_launcher_core::mcp::builtin::builtin_server_info();
    ok("MCP servers", servers)
}

// ── Agent Tools ─────────────────────────────────────────────────────────────

/// GET /api/agent/tools — list all built-in agent tools from the tool registry.
pub fn list_agent_tools() -> Response<Cursor<Vec<u8>>> {
    let registry = ai_launcher_core::tools::builtin::default_registry();
    let defs: Vec<serde_json::Value> = registry
        .definitions()
        .into_iter()
        .map(|d| serde_json::json!({
            "name": d.name,
            "description": d.description,
            "parameters": d.parameters,
        }))
        .collect();
    ok(&format!("{} agent tools", defs.len()), defs)
}

// ── Skills ──────────────────────────────────────────────────────────────────

/// GET /api/skills — list available skills from real SkillLoader.
pub fn list_skills() -> Response<Cursor<Vec<u8>>> {
    // Build search paths: CWD/.agents/skills, CWD/.agent/skills, and exe-relative
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
                .map(|s| serde_json::json!({
                    "name": s.name,
                    "description": s.description,
                    "path": s.path,
                    "triggers": s.triggers,
                }))
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
pub fn list_knowledge(data_dir: &Path) -> Response<Cursor<Vec<u8>>> {
    let db_path = data_dir.join("knowledge.db");
    match ai_launcher_core::knowledge::KnowledgeGraph::new(&db_path) {
        Ok(kg) => {
            // Get all entity types by searching broadly
            let mut all_entities = Vec::new();
            for entity_type in &["app", "concept", "project", "architecture", "security", "llm", "plugins", "channels"] {
                if let Ok(entities) = kg.find_by_type(entity_type) {
                    all_entities.extend(entities);
                }
            }
            // Also do a wildcard search to catch anything
            if let Ok(entities) = kg.search("%") {
                for entity in entities {
                    if !all_entities.iter().any(|e: &ai_launcher_core::knowledge::Entity| e.id == entity.id) {
                        all_entities.push(entity);
                    }
                }
            }

            let entries: Vec<serde_json::Value> = all_entities
                .iter()
                .map(|e| serde_json::json!({
                    "id": e.id,
                    "key": e.name,
                    "value": e.metadata,
                    "category": e.entity_type,
                }))
                .collect();
            ok(&format!("{} knowledge entries", entries.len()), entries)
        }
        Err(e) => {
            eprintln!("Knowledge graph error: {}", e);
            ok("0 knowledge entries", serde_json::json!([]))
        }
    }
}

/// GET /api/knowledge/search?q=... — search knowledge.
pub fn search_knowledge(query: &str, data_dir: &Path) -> Response<Cursor<Vec<u8>>> {
    let db_path = data_dir.join("knowledge.db");
    match ai_launcher_core::knowledge::KnowledgeGraph::new(&db_path) {
        Ok(kg) => {
            match kg.search(query) {
                Ok(entities) => {
                    let entries: Vec<serde_json::Value> = entities
                        .iter()
                        .map(|e| serde_json::json!({
                            "id": e.id,
                            "key": e.name,
                            "value": e.metadata,
                            "category": e.entity_type,
                        }))
                        .collect();
                    ok(&format!("Knowledge search: {} ({} results)", query, entries.len()), entries)
                }
                Err(e) => {
                    eprintln!("Knowledge search error: {}", e);
                    ok(&format!("Knowledge search: {}", query), serde_json::json!([]))
                }
            }
        }
        Err(e) => {
            eprintln!("Knowledge graph error: {}", e);
            ok(&format!("Knowledge search: {}", query), serde_json::json!([]))
        }
    }
}

// ── Memory ──────────────────────────────────────────────────────────────────

/// GET /api/memory — list all memory entries from real MemoryManager.
pub fn list_memory(data_dir: &Path) -> Response<Cursor<Vec<u8>>> {
    let db_path = data_dir.join("memory.db");
    match ai_launcher_core::memory::KvStore::new(&db_path) {
        Ok(kv) => {
            match kv.list_keys("") {
                Ok(keys) => {
                    let mut entries = Vec::new();
                    for key in &keys {
                        let value = kv.get(key).unwrap_or(None).unwrap_or_default();
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
        Err(e) => {
            eprintln!("Memory store error: {}", e);
            ok("0 memory entries", serde_json::json!([]))
        }
    }
}

/// GET /api/memory/{key} — get a specific memory value.
pub fn get_memory(key: &str, data_dir: &Path) -> Response<Cursor<Vec<u8>>> {
    let db_path = data_dir.join("memory.db");
    match ai_launcher_core::memory::KvStore::new(&db_path) {
        Ok(kv) => {
            match kv.get(key) {
                Ok(Some(value)) => {
                    ok(&format!("Memory: {}", key), serde_json::json!({
                        "key": key,
                        "value": value,
                    }))
                }
                Ok(None) => err(404, &format!("Memory key not found: {}", key)),
                Err(e) => err(500, &format!("Memory read error: {}", e)),
            }
        }
        Err(e) => err(500, &format!("Memory store error: {}", e)),
    }
}

// ── OpenViking ──────────────────────────────────────────────────────────────

/// GET /api/viking/status — check OpenViking server connectivity.
pub fn viking_status(rt: &tokio::runtime::Runtime) -> Response<Cursor<Vec<u8>>> {
    let client = ai_launcher_core::openviking::VikingClient::new("http://localhost:1933");
    let healthy = rt.block_on(async { client.health().await.unwrap_or(false) });

    if healthy {
        match rt.block_on(async { client.status().await }) {
            Ok(status) => ok("OpenViking connected", serde_json::json!({
                "connected": true,
                "status": status,
            })),
            Err(_) => ok("OpenViking connected", serde_json::json!({
                "connected": true,
            })),
        }
    } else {
        ok("OpenViking not available", serde_json::json!({
            "connected": false,
            "message": "OpenViking server not running on localhost:1933. Install with: pip install openviking"
        }))
    }
}
