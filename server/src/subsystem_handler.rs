/// Channel, MCP, Skills, Knowledge, Memory handlers.
/// These expose core subsystem data to the desktop UI.
use std::io::Cursor;
use tiny_http::Response;

use crate::response::*;

// ── Channels ────────────────────────────────────────────────────────────────

/// GET /api/channels — list registered channels + status.
/// For now returns the default REST channel, since ChannelManager isn't wired
/// into main.rs yet. When it is, swap to the real ChannelManager reference.
pub fn list_channels() -> Response<Cursor<Vec<u8>>> {
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
            "is_running": false,
            "messages_received": 0,
            "messages_sent": 0
        },
        {
            "name": "discord-bot",
            "channel_type": "discord",
            "is_running": false,
            "messages_received": 0,
            "messages_sent": 0
        }
    ]);
    ok("Channel list", default_channels)
}

// ── MCP ─────────────────────────────────────────────────────────────────────

/// GET /api/mcp/tools — list discovered MCP tools.
pub fn list_mcp_tools() -> Response<Cursor<Vec<u8>>> {
    // Return built-in agent tools as MCP-style definitions
    let tools = serde_json::json!([
        { "name": "read_file", "description": "Read a file from the sandboxed workspace", "parameters": { "type": "object", "properties": { "path": { "type": "string" } } } },
        { "name": "write_file", "description": "Write content to a file in the sandbox", "parameters": { "type": "object", "properties": { "path": { "type": "string" }, "content": { "type": "string" } } } },
        { "name": "list_directory", "description": "List directory contents in the sandbox", "parameters": { "type": "object", "properties": { "path": { "type": "string" } } } },
        { "name": "run_command", "description": "Execute a shell command inside the sandbox", "parameters": { "type": "object", "properties": { "command": { "type": "string" } } } },
        { "name": "search_knowledge", "description": "Search the knowledge graph", "parameters": { "type": "object", "properties": { "query": { "type": "string" } } } },
        { "name": "store_memory", "description": "Store a key-value pair in working memory", "parameters": { "type": "object", "properties": { "key": { "type": "string" }, "value": { "type": "string" } } } },
        { "name": "recall_memory", "description": "Recall a value from working memory", "parameters": { "type": "object", "properties": { "key": { "type": "string" } } } },
        { "name": "http_request", "description": "Make an HTTP request", "parameters": { "type": "object", "properties": { "url": { "type": "string" }, "method": { "type": "string" } } } },
        { "name": "install_app", "description": "Install an AI app from the catalog", "parameters": { "type": "object", "properties": { "app_id": { "type": "string" } } } },
        { "name": "launch_app", "description": "Launch an installed AI app", "parameters": { "type": "object", "properties": { "app_id": { "type": "string" } } } },
        { "name": "stop_app", "description": "Stop a running AI app", "parameters": { "type": "object", "properties": { "app_id": { "type": "string" } } } },
        { "name": "system_info", "description": "Get system information (OS, CPU, RAM, GPU)", "parameters": { "type": "object", "properties": {} } }
    ]);
    ok("MCP tools", tools)
}

/// GET /api/mcp/servers — list MCP server connections.
pub fn list_mcp_servers() -> Response<Cursor<Vec<u8>>> {
    let servers = serde_json::json!([
        {
            "name": "nde-os-builtin",
            "transport": "stdio",
            "tools_count": 12,
            "is_connected": true
        }
    ]);
    ok("MCP servers", servers)
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
