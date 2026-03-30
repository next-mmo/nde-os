use anyhow::Result;
use serde::Deserialize;
use serde_json::json;

use crate::sandbox::Sandbox;
use crate::tools::ToolRegistry;

/// MCP Server — exposes NDE-OS tools as an MCP server.
/// Other AI agents (Claude Code, Cursor, etc.) can connect to use NDE-OS sandboxed tools.
///
/// Supports two modes:
///   1. **Registry mode** (production): holds a ToolRegistry + Sandbox, executes tools for real.
///   2. **Schema-only mode** (listing): tool schemas only, no execution backend.
pub struct McpServer {
    tools: Vec<McpExposedTool>,
    /// Real execution backend — when present, tools/call dispatches to ToolRegistry.
    executor: Option<ToolExecutor>,
}

struct ToolExecutor {
    registry: ToolRegistry,
    sandbox: Sandbox,
    runtime: tokio::runtime::Runtime,
}

#[derive(Debug, Clone)]
struct McpExposedTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

/// Maps MCP-exposed tool names (e.g. `nde_file_read`) to internal ToolRegistry
/// names (e.g. `file_read`). The convention is:
///   - `nde_kanban_*`  → handled by kanban module (before this function is called)
///   - `nde_shell`     → `shell_exec` (special alias)
///   - `nde_*`         → strip `nde_` prefix
///   - everything else → pass through as-is
fn mcp_name_to_registry_name(mcp_name: &str) -> String {
    // Explicit aliases where MCP name and registry name diverge
    match mcp_name {
        "nde_shell" => "shell_exec".to_string(),
        _ => {
            if let Some(stripped) = mcp_name.strip_prefix("nde_") {
                stripped.to_string()
            } else {
                mcp_name.to_string()
            }
        }
    }
}

impl McpServer {
    /// Create a schema-only server (no execution backend).
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            executor: None,
        }
    }

    /// Create a server with a real execution backend.
    /// Tools will be dispatched to the ToolRegistry inside the given Sandbox.
    pub fn with_executor(registry: ToolRegistry, sandbox: Sandbox) -> Result<Self> {
        let runtime = tokio::runtime::Runtime::new()?;
        Ok(Self {
            tools: Vec::new(),
            executor: Some(ToolExecutor {
                registry,
                sandbox,
                runtime,
            }),
        })
    }

    /// Register a tool to expose via MCP.
    pub fn register_tool(
        &mut self,
        name: &str,
        description: &str,
        input_schema: serde_json::Value,
    ) {
        self.tools.push(McpExposedTool {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
        });
    }

    /// Handle a JSON-RPC request and return a response.
    pub fn handle_request(&self, request: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct JsonRpcRequest {
            #[allow(dead_code)]
            jsonrpc: String,
            id: Option<serde_json::Value>,
            method: String,
            #[serde(default)]
            params: Option<serde_json::Value>,
        }

        let req: JsonRpcRequest = serde_json::from_str(request)?;

        let response = match req.method.as_str() {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "nde-os",
                        "version": "0.2.0"
                    }
                }
            }),

            "tools/list" => {
                let tools: Vec<serde_json::Value> = self
                    .tools
                    .iter()
                    .map(|t| {
                        json!({
                            "name": t.name,
                            "description": t.description,
                            "inputSchema": t.input_schema,
                        })
                    })
                    .collect();

                json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "result": { "tools": tools }
                })
            }

            "tools/call" => {
                let tool_name = req
                    .params
                    .as_ref()
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");

                let arguments = req
                    .params
                    .as_ref()
                    .and_then(|p| p.get("arguments"))
                    .cloned()
                    .unwrap_or_else(|| json!({}));

                self.execute_tool(tool_name, &arguments, &req.id, &req.params)
            }

            "notifications/initialized" => {
                // No response needed for notifications
                return Ok(String::new());
            }

            _ => json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", req.method)
                }
            }),
        };

        Ok(serde_json::to_string(&response)?)
    }

    /// Execute a tool call, routing to the appropriate handler.
    fn execute_tool(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        id: &Option<serde_json::Value>,
        raw_params: &Option<serde_json::Value>,
    ) -> serde_json::Value {
        // 1. Kanban tools — always handled by the kanban module
        if tool_name.starts_with("nde_kanban_") {
            let empty = json!({});
            let params = raw_params.as_ref().unwrap_or(&empty);
            return match crate::mcp::kanban::execute(tool_name, params) {
                Ok(result) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": result }]
                    }
                }),
                Err(e) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": format!("Kanban tool error: {}", e)
                    }
                }),
            };
        }

        // 2. All other tools — dispatch to ToolRegistry if available
        if let Some(executor) = &self.executor {
            let registry_name = mcp_name_to_registry_name(tool_name);
            let tool_call = crate::llm::ToolCall {
                id: "mcp".to_string(),
                name: registry_name.clone(),
                arguments: arguments.clone(),
            };

            match executor
                .runtime
                .block_on(executor.registry.execute(&tool_call, &executor.sandbox))
            {
                Ok(result) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": result }]
                    }
                }),
                Err(e) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": format!("Error: {}", e) }],
                        "isError": true
                    }
                }),
            }
        } else {
            // Schema-only mode — no executor attached
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{
                        "type": "text",
                        "text": format!("Tool '{}' is registered but no executor backend is attached. Start the MCP server with a workspace to enable tool execution.", tool_name)
                    }],
                    "isError": true
                }
            })
        }
    }

    /// Run the MCP server on stdio (blocking).
    pub async fn run_stdio(&self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);

        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                break; // EOF
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match self.handle_request(line) {
                Ok(response) => {
                    if !response.is_empty() {
                        stdout
                            .write_all(format!("{}\n", response).as_bytes())
                            .await?;
                        stdout.flush().await?;
                    }
                }
                Err(e) => {
                    let error_resp = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {
                            "code": -32603,
                            "message": format!("Internal error: {}", e)
                        }
                    });
                    stdout
                        .write_all(format!("{}\n", serde_json::to_string(&error_resp)?).as_bytes())
                        .await?;
                    stdout.flush().await?;
                }
            }
        }

        Ok(())
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}
