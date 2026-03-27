//! Built-in MCP tool server registration.
//!
//! Registers all NDE-OS tools as MCP-accessible tools, organized by category.
//! External AI clients (Claude Code, Cursor, etc.) can use these tools via MCP protocol.

use super::server::McpServer;
use serde_json::json;

/// Create an MCP server with all built-in NDE-OS tools registered (schema-only, no executor).
pub fn create_default_server() -> McpServer {
    let mut server = McpServer::new();
    register_tools(&mut server);
    crate::mcp::kanban::register(&mut server);
    server
}

/// Create an MCP server with all built-in tools AND a real execution backend.
/// External agents/IDEs can call tools and get real results.
///
/// `workspace` is the directory to sandbox — all file operations are jailed here.
pub fn create_executable_server(workspace: &std::path::Path) -> anyhow::Result<McpServer> {
    let sandbox = crate::sandbox::Sandbox::new(workspace)?;
    let registry = crate::tools::builtin::default_registry();

    let mut server = McpServer::with_executor(registry, sandbox)?;
    register_tools(&mut server);
    crate::mcp::kanban::register(&mut server);

    Ok(server)
}

/// Register all built-in tool schemas on a server. Shared between
/// `create_default_server` (schema-only) and `create_executable_server` (live).
fn register_tools(server: &mut McpServer) {
    // ── Filesystem tools ─────────────────────────────────────────────────
    server.register_tool(
        "nde_file_read",
        "Read the contents of a file in the NDE-OS sandbox. Returns file content with line numbers.",
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace" },
                "max_lines": { "type": "integer", "description": "Maximum lines to read (default: unlimited)" }
            },
            "required": ["path"]
        }),
    );

    server.register_tool(
        "nde_file_write",
        "Write content to a file in the NDE-OS sandbox. Creates parent directories automatically.",
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to workspace" },
                "content": { "type": "string", "description": "Content to write" }
            },
            "required": ["path", "content"]
        }),
    );

    server.register_tool(
        "nde_file_list",
        "List files and directories in the NDE-OS sandbox workspace.",
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path (default: '.')" },
                "recursive": { "type": "boolean", "description": "List recursively (default: false)" }
            }
        }),
    );

    server.register_tool(
        "nde_file_search",
        "Search for text patterns across files in the sandbox workspace.",
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Text pattern to search for" },
                "path": { "type": "string", "description": "Directory to search in (default: '.')" },
                "extensions": { "type": "string", "description": "File extensions to search (comma-separated, e.g., 'rs,ts,py')" }
            },
            "required": ["pattern"]
        }),
    );

    server.register_tool(
        "nde_file_patch",
        "Apply a search-and-replace edit to a file.",
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path" },
                "search": { "type": "string", "description": "Text to find" },
                "replace": { "type": "string", "description": "Replacement text" },
                "all": { "type": "boolean", "description": "Replace all occurrences (default: false)" }
            },
            "required": ["path", "search", "replace"]
        }),
    );

    // ── Shell ────────────────────────────────────────────────────────────
    server.register_tool(
        "nde_shell",
        "Execute a shell command in the NDE-OS sandbox. 30-second timeout.",
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" },
                "timeout_secs": { "type": "integer", "description": "Timeout in seconds (default: 30)" }
            },
            "required": ["command"]
        }),
    );

    // ── Code analysis ────────────────────────────────────────────────────
    server.register_tool(
        "nde_code_search",
        "Search for code patterns across source files using regex.",
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Regex pattern" },
                "path": { "type": "string", "description": "Directory to search (default: '.')" },
                "glob": { "type": "string", "description": "File glob pattern (e.g., '*.rs')" }
            },
            "required": ["pattern"]
        }),
    );

    server.register_tool(
        "nde_code_symbols",
        "Extract symbols (functions, classes, structs) from source files. Supports Rust, Python, JS/TS.",
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to analyze" }
            },
            "required": ["path"]
        }),
    );

    // ── Web tools ────────────────────────────────────────────────────────
    server.register_tool(
        "nde_web_browse",
        "Fetch a web page and extract readable text content, links, and metadata.",
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL to browse" },
                "extract": {
                    "type": "string",
                    "enum": ["text", "links", "all"],
                    "description": "What to extract (default: 'all')"
                }
            },
            "required": ["url"]
        }),
    );

    server.register_tool(
        "nde_web_search",
        "Search the web using DuckDuckGo (no API key needed). Returns titles, URLs, and snippets.",
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "max_results": { "type": "integer", "description": "Max results (default: 10)" }
            },
            "required": ["query"]
        }),
    );

    server.register_tool(
        "nde_http_fetch",
        "Make an HTTP request (GET/POST/PUT/DELETE). For API calls and service health checks.",
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL to request" },
                "method": { "type": "string", "enum": ["GET", "POST", "PUT", "DELETE"], "description": "HTTP method" },
                "body": { "type": "string", "description": "Request body" },
                "headers": { "type": "object", "description": "Request headers" }
            },
            "required": ["url"]
        }),
    );

    // ── Git operations ───────────────────────────────────────────────────
    server.register_tool(
        "nde_git",
        "Run git operations in the workspace (status, diff, log, branch, commit, etc.).",
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "enum": ["status", "diff", "log", "branch", "add", "commit", "checkout", "stash", "remote", "tag", "init", "show"],
                    "description": "Git command"
                },
                "args": { "type": "string", "description": "Additional arguments" }
            },
            "required": ["command"]
        }),
    );

    // ── Memory & Knowledge ───────────────────────────────────────────────
    server.register_tool(
        "nde_memory_store",
        "Store a key-value pair in the agent's persistent memory.",
        json!({
            "type": "object",
            "properties": {
                "key": { "type": "string", "description": "Memory key" },
                "value": { "type": "string", "description": "Value to store" }
            },
            "required": ["key", "value"]
        }),
    );

    server.register_tool(
        "nde_memory_recall",
        "Recall a value from persistent memory by key, or list all keys.",
        json!({
            "type": "object",
            "properties": {
                "key": { "type": "string", "description": "Key to recall (omit to list all)" },
                "prefix": { "type": "string", "description": "List keys matching prefix" }
            }
        }),
    );

    server.register_tool(
        "nde_knowledge_store",
        "Store an entity or relation in the knowledge graph.",
        json!({
            "type": "object",
            "properties": {
                "entity_type": { "type": "string", "description": "Entity type (e.g., 'person', 'concept', 'project')" },
                "name": { "type": "string", "description": "Entity name" },
                "metadata": { "type": "object", "description": "Additional attributes" },
                "relation_to": { "type": "string", "description": "Target entity for a relation" },
                "relation_type": { "type": "string", "description": "Type of relation (e.g., 'depends_on', 'authored_by')" }
            },
            "required": ["entity_type", "name"]
        }),
    );

    server.register_tool(
        "nde_knowledge_query",
        "Query the knowledge graph for entities and relations.",
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "entity_type": { "type": "string", "description": "Filter by entity type" }
            }
        }),
    );

    // ── System ───────────────────────────────────────────────────────────
    server.register_tool(
        "nde_system_info",
        "Get system information: platform, memory, disk, sandbox status.",
        json!({
            "type": "object",
            "properties": {}
        }),
    );

    server.register_tool(
        "nde_app_list",
        "List installed and available applications in NDE-OS.",
        json!({
            "type": "object",
            "properties": {
                "catalog": { "type": "boolean", "description": "Show full catalog (default: false)" }
            }
        }),
    );

    server.register_tool(
        "nde_skill_list",
        "List available agent skills (SKILL.md files).",
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query to filter skills" }
            }
        }),
    );

    // ── OpenViking context tools ────────────────────────────────────────
    server.register_tool(
        "nde_viking_find",
        "Semantic search across the OpenViking context database. Searches memories, resources, and skills using directory-recursive retrieval.",
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query for semantic retrieval" }
            },
            "required": ["query"]
        }),
    );

    server.register_tool(
        "nde_viking_read",
        "Read content from the OpenViking context database at a viking:// URI. Supports tiered access: L0 (abstract ~100 tokens), L1 (overview ~2k tokens), L2 (full content).",
        json!({
            "type": "object",
            "properties": {
                "uri": { "type": "string", "description": "Viking URI (e.g., viking://resources/docs/api.md)" },
                "tier": { "type": "string", "enum": ["abstract", "overview", "full"], "description": "Content tier: abstract (L0), overview (L1), or full (L2, default)" }
            },
            "required": ["uri"]
        }),
    );

    server.register_tool(
        "nde_viking_ls",
        "List contents at a viking:// URI. Browse the virtual filesystem of agent context (resources, user memories, agent skills).",
        json!({
            "type": "object",
            "properties": {
                "uri": { "type": "string", "description": "Viking URI to list (e.g., viking://resources/, viking://user/memories/)" },
                "recursive": { "type": "boolean", "description": "List recursively (default: false)" }
            },
            "required": ["uri"]
        }),
    );
}

/// Get the list of built-in MCP server info for the API.
pub fn builtin_server_info() -> Vec<serde_json::Value> {
    vec![
        json!({
            "name": "nde-os-tools",
            "description": "NDE-OS built-in sandboxed tools — filesystem, shell, code analysis, web browsing, git, memory, knowledge graph, OpenViking context database, and system management",
            "transport": "stdio",
            "status": "running",
            "tools_count": 22,
            "version": "0.2.0"
        }),
    ]
}

/// Get MCP tool definitions for the API.
pub fn builtin_tool_definitions() -> Vec<serde_json::Value> {
    let server = create_default_server();
    // Use handle_request to get tools list
    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    match server.handle_request(&serde_json::to_string(&req).unwrap_or_default()) {
        Ok(resp) => {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&resp) {
                parsed["result"]["tools"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
        Err(_) => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_server() {
        let server = create_default_server();
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        });
        let resp = server.handle_request(&serde_json::to_string(&req).unwrap()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();
        assert!(tools.len() >= 22, "Expected 22+ tools, got {}", tools.len());
    }

    #[test]
    fn test_builtin_server_info() {
        let info = builtin_server_info();
        assert_eq!(info.len(), 1);
        assert_eq!(info[0]["name"], "nde-os-tools");
    }

    #[test]
    fn test_builtin_tool_definitions() {
        let tools = builtin_tool_definitions();
        assert!(tools.len() >= 22);
    }

    #[test]
    fn test_executable_server_creation() {
        let tmp = std::env::temp_dir().join("nde_mcp_test_workspace");
        std::fs::create_dir_all(&tmp).unwrap();
        let server = create_executable_server(&tmp).unwrap();

        // Verify tools are registered
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        });
        let resp = server.handle_request(&serde_json::to_string(&req).unwrap()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();
        assert!(tools.len() >= 22, "Expected 22+ tools, got {}", tools.len());

        // Clean up
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_executable_server_file_read() {
        let tmp = std::env::temp_dir().join("nde_mcp_test_file_read");
        std::fs::create_dir_all(&tmp).unwrap();

        // Write a test file
        std::fs::write(tmp.join("hello.txt"), "Hello from MCP!").unwrap();

        let server = create_executable_server(&tmp).unwrap();

        // Call nde_file_read
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "nde_file_read",
                "arguments": { "path": "hello.txt" }
            }
        });
        let resp = server.handle_request(&serde_json::to_string(&req).unwrap()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();

        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Hello from MCP!"), "Expected file content, got: {}", text);
        assert!(parsed["result"]["isError"].is_null(), "Should not be an error");

        // Clean up
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_executable_server_file_write_and_read() {
        let tmp = std::env::temp_dir().join("nde_mcp_test_file_write");
        std::fs::create_dir_all(&tmp).unwrap();

        let server = create_executable_server(&tmp).unwrap();

        // Write via MCP
        let write_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "nde_file_write",
                "arguments": { "path": "test_output.txt", "content": "Written via MCP gateway!" }
            }
        });
        let resp = server.handle_request(&serde_json::to_string(&write_req).unwrap()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert!(parsed["result"]["isError"].is_null(), "Write should not error");

        // Read it back via MCP
        let read_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "nde_file_read",
                "arguments": { "path": "test_output.txt" }
            }
        });
        let resp = server.handle_request(&serde_json::to_string(&read_req).unwrap()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Written via MCP gateway!"), "Round-trip failed: {}", text);

        // Clean up
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_executable_server_shell() {
        let tmp = std::env::temp_dir().join("nde_mcp_test_shell");
        std::fs::create_dir_all(&tmp).unwrap();

        let server = create_executable_server(&tmp).unwrap();

        let cmd = if cfg!(windows) { "echo hello_mcp" } else { "echo hello_mcp" };
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "nde_shell",
                "arguments": { "command": cmd }
            }
        });
        let resp = server.handle_request(&serde_json::to_string(&req).unwrap()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("hello_mcp"), "Shell should echo, got: {}", text);

        // Clean up
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_sandbox_escape_blocked() {
        let tmp = std::env::temp_dir().join("nde_mcp_test_sandbox_escape");
        std::fs::create_dir_all(&tmp).unwrap();

        let server = create_executable_server(&tmp).unwrap();

        // Attempt path traversal
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "nde_file_read",
                "arguments": { "path": "../../../etc/passwd" }
            }
        });
        let resp = server.handle_request(&serde_json::to_string(&req).unwrap()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        let is_error = parsed["result"]["isError"].as_bool().unwrap_or(false);
        assert!(is_error, "Path traversal should be blocked");

        // Clean up
        std::fs::remove_dir_all(&tmp).ok();
    }
}
