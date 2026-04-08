/// Channel, MCP, Skills, Knowledge, Memory handlers.
/// These expose core subsystem data to the desktop UI.
/// Skills / Knowledge / Memory now use real core modules with OpenViking fallback.
use crate::response::*;

use serde_json::Value;
use std::fs;
use std::path::Path;

// ── Knowledge entity helper ─────────────────────────────────────────────────

/// Convert a core Entity to a JSON value.
/// Used by both list_knowledge and search_knowledge to avoid duplication.
fn entity_to_json(e: &ai_launcher_core::knowledge::Entity) -> serde_json::Value {
    serde_json::json!({
        "id": e.id,
        "key": e.name,
        "value": e.metadata,
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
pub fn list_knowledge(data_dir: &Path) -> HttpResponse {
    let db_path = data_dir.join("knowledge.db");
    match ai_launcher_core::knowledge::KnowledgeGraph::new(&db_path) {
        Ok(kg) => {
            let mut all_entities = Vec::new();
            for entity_type in &[
                "app",
                "concept",
                "project",
                "architecture",
                "security",
                "llm",
                "plugins",
                "channels",
            ] {
                if let Ok(entities) = kg.find_by_type(entity_type) {
                    all_entities.extend(entities);
                }
            }
            if let Ok(entities) = kg.search("%") {
                for entity in entities {
                    if !all_entities
                        .iter()
                        .any(|e: &ai_launcher_core::knowledge::Entity| e.id == entity.id)
                    {
                        all_entities.push(entity);
                    }
                }
            }

            let entries: Vec<serde_json::Value> = all_entities.iter().map(entity_to_json).collect();
            ok(&format!("{} knowledge entries", entries.len()), entries)
        }
        Err(e) => {
            eprintln!("Knowledge graph error: {}", e);
            ok("0 knowledge entries", serde_json::json!([]))
        }
    }
}

/// GET /api/knowledge/search?q=... — search knowledge.
pub fn search_knowledge(query: &str, data_dir: &Path) -> HttpResponse {
    let db_path = data_dir.join("knowledge.db");
    match ai_launcher_core::knowledge::KnowledgeGraph::new(&db_path) {
        Ok(kg) => match kg.search(query) {
            Ok(entities) => {
                let entries: Vec<serde_json::Value> = entities.iter().map(entity_to_json).collect();
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
        },
        Err(e) => {
            eprintln!("Knowledge graph error: {}", e);
            ok(
                &format!("Knowledge search: {}", query),
                serde_json::json!([]),
            )
        }
    }
}

// ── Memory ──────────────────────────────────────────────────────────────────

/// GET /api/memory — list all memory entries from real MemoryManager.
pub fn list_memory(data_dir: &Path) -> HttpResponse {
    let db_path = data_dir.join("memory.db");
    match ai_launcher_core::memory::KvStore::new(&db_path) {
        Ok(kv) => match kv.list_keys("") {
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
        },
        Err(e) => {
            eprintln!("Memory store error: {}", e);
            ok("0 memory entries", serde_json::json!([]))
        }
    }
}

/// GET /api/memory/{key} — get a specific memory value.
pub fn get_memory(key: &str, data_dir: &Path) -> HttpResponse {
    let db_path = data_dir.join("memory.db");
    match ai_launcher_core::memory::KvStore::new(&db_path) {
        Ok(kv) => match kv.get(key) {
            Ok(Some(value)) => ok(
                &format!("Memory: {}", key),
                serde_json::json!({
                    "key": key,
                    "value": value,
                }),
            ),
            Ok(None) => err(404, &format!("Memory key not found: {}", key)),
            Err(e) => err(500, &format!("Memory read error: {}", e)),
        },
        Err(e) => err(500, &format!("Memory store error: {}", e)),
    }
}

// ── OpenViking ──────────────────────────────────────────────────────────────

/// GET /api/viking/status — check OpenViking server status (process + connectivity).
pub fn viking_status(
    rt: &tokio::runtime::Runtime,
    viking: &std::sync::Mutex<ai_launcher_core::openviking::VikingProcess>,
) -> HttpResponse {
    let process_running = match viking.lock() {
        Ok(mut v) => v.is_running(),
        Err(_) => false,
    };

    let client = ai_launcher_core::openviking::VikingClient::new("http://localhost:1933");
    let healthy = rt.block_on(async { client.health().await.unwrap_or(false) });

    if healthy {
        let status = rt.block_on(async { client.status().await }).ok();
        ok(
            "OpenViking connected",
            serde_json::json!({
                "connected": true,
                "process_managed": process_running,
                "status": status,
            }),
        )
    } else {
        ok(
            "OpenViking not available",
            serde_json::json!({
                "connected": false,
                "process_managed": process_running,
                "message": "OpenViking server not running. Use POST /api/viking/install then POST /api/viking/start"
            }),
        )
    }
}

/// POST /api/viking/install — install OpenViking via uv/pip inside the sandbox.
pub fn viking_install(
    rt: &tokio::runtime::Runtime,
    viking: &std::sync::Mutex<ai_launcher_core::openviking::VikingProcess>,
) -> HttpResponse {
    let v = match viking.lock() {
        Ok(v) => v,
        Err(_) => return err(500, "Viking process lock failed"),
    };

    match rt.block_on(v.ensure_installed()) {
        Ok(true) => ok(
            "OpenViking installed",
            serde_json::json!({ "installed": true }),
        ),
        Ok(false) => err(
            500,
            "OpenViking installation failed. Ensure Python and uv/pip are available.",
        ),
        Err(e) => err(500, &format!("Installation error: {}", e)),
    }
}

/// POST /api/viking/start — start the OpenViking server process.
pub fn viking_start(
    rt: &tokio::runtime::Runtime,
    viking: &std::sync::Mutex<ai_launcher_core::openviking::VikingProcess>,
) -> HttpResponse {
    let mut v = match viking.lock() {
        Ok(v) => v,
        Err(_) => return err(500, "Viking process lock failed"),
    };

    match rt.block_on(v.start()) {
        Ok(()) => {
            let port = v.config().port;
            ok(
                "OpenViking server started",
                serde_json::json!({
                    "running": true,
                    "port": port,
                    "url": format!("http://localhost:{}", port),
                }),
            )
        }
        Err(e) => err(500, &format!("Failed to start OpenViking: {}", e)),
    }
}

/// POST /api/viking/stop — stop the OpenViking server process.
pub fn viking_stop(
    rt: &tokio::runtime::Runtime,
    viking: &std::sync::Mutex<ai_launcher_core::openviking::VikingProcess>,
) -> HttpResponse {
    let mut v = match viking.lock() {
        Ok(v) => v,
        Err(_) => return err(500, "Viking process lock failed"),
    };

    rt.block_on(v.stop());
    ok(
        "OpenViking server stopped",
        serde_json::json!({ "running": false }),
    )
}
