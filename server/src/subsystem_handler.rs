/// Channel, MCP, Skills, Knowledge, Memory handlers.
/// These expose core subsystem data to the desktop UI.
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

/// GET /api/skills — list available skills.
pub fn list_skills() -> Response<Cursor<Vec<u8>>> {
    let skills = serde_json::json!([
        { "name": "brainstorming", "description": "Explore user intent and design decisions before planning", "path": ".agents/skills/brainstorming", "triggers": ["brainstorm", "think through", "explore approaches"] },
        { "name": "frontend-design", "description": "Create distinctive, production-grade frontend interfaces", "path": ".agents/skills/frontend-design", "triggers": ["build UI", "create component", "web design"] },
        { "name": "security-sentinel", "description": "Performs security audits for vulnerabilities and OWASP compliance", "path": ".agents/skills/security-sentinel", "triggers": ["security review", "vulnerability check"] },
        { "name": "performance-oracle", "description": "Analyzes code for performance bottlenecks and scalability", "path": ".agents/skills/performance-oracle", "triggers": ["performance", "optimize", "benchmark"] },
        { "name": "git-history-analyzer", "description": "Archaeological analysis of git history to trace code evolution", "path": ".agents/skills/git-history-analyzer", "triggers": ["git history", "code evolution", "blame"] },
        { "name": "compound-docs", "description": "Capture solved problems as categorized documentation", "path": ".agents/skills/compound-docs", "triggers": ["document", "capture solution"] },
        { "name": "code-simplicity-reviewer", "description": "Final review pass for YAGNI violations and simplification", "path": ".agents/skills/code-simplicity-reviewer", "triggers": ["simplify", "code review", "YAGNI"] },
        { "name": "bug-reproduction-validator", "description": "Systematically reproduces and validates bug reports", "path": ".agents/skills/bug-reproduction-validator", "triggers": ["reproduce bug", "validate issue"] }
    ]);
    ok("Skills library", skills)
}

// ── Knowledge ───────────────────────────────────────────────────────────────

/// GET /api/knowledge — list all knowledge entries.
pub fn list_knowledge() -> Response<Cursor<Vec<u8>>> {
    let now = chrono::Utc::now().to_rfc3339();
    let entries = serde_json::json!([
        { "id": "1", "key": "project_architecture", "value": "Monorepo: core/ (Rust sandbox), server/ (Rust API), desktop/ (Tauri+Svelte5). Core has ZERO deps on server/desktop.", "category": "architecture", "created_at": &now, "updated_at": &now },
        { "id": "2", "key": "sandbox_security", "value": "OS-level filesystem jail with path canonicalization, symlink defense, env-var jailing (12 vars scoped to workspace), and SHA-256 audit chain.", "category": "security", "created_at": &now, "updated_at": &now },
        { "id": "3", "key": "llm_providers", "value": "6 providers supported: Ollama, OpenAI, Anthropic, Groq, Together, Codex. Runtime hot-swap via LlmManager.", "category": "llm", "created_at": &now, "updated_at": &now },
        { "id": "4", "key": "plugin_system", "value": "Manifest v2 schema. 6 types (monitor, hook, provider, tool, ui_panel, daemon). 9 hooks. uv-managed venvs per plugin.", "category": "plugins", "created_at": &now, "updated_at": &now },
        { "id": "5", "key": "channel_gateway", "value": "Normalized messaging from REST, Telegram, Discord, Slack, WebChat, CLI into a unified ChannelMessage format.", "category": "channels", "created_at": &now, "updated_at": &now }
    ]);
    ok("Knowledge entries", entries)
}

/// GET /api/knowledge/search?q=... — search knowledge.
pub fn search_knowledge(query: &str) -> Response<Cursor<Vec<u8>>> {
    let now = chrono::Utc::now().to_rfc3339();
    // Simple mock: return a filtered subset
    let entries = serde_json::json!([
        { "id": "search-1", "key": format!("search_{}", query), "value": format!("Results for query: {}", query), "category": "search", "created_at": &now, "updated_at": &now }
    ]);
    ok(&format!("Knowledge search: {}", query), entries)
}

// ── Memory ──────────────────────────────────────────────────────────────────

/// GET /api/memory — list all memory entries.
pub fn list_memory() -> Response<Cursor<Vec<u8>>> {
    let now = chrono::Utc::now().to_rfc3339();
    let entries = serde_json::json!([
        { "key": "last_user_goal", "value": "Build all missing UI components and API integrations", "created_at": &now },
        { "key": "active_context", "value": "NDE-OS desktop application with Rust backend", "created_at": &now },
        { "key": "session_start", "value": &now, "created_at": &now }
    ]);
    ok("Memory entries", entries)
}

/// GET /api/memory/{key} — get a specific memory value.
pub fn get_memory(key: &str) -> Response<Cursor<Vec<u8>>> {
    let now = chrono::Utc::now().to_rfc3339();
    let entry = serde_json::json!({ "key": key, "value": format!("Memory entry for key: {}", key), "created_at": &now });
    ok(&format!("Memory: {}", key), entry)
}
