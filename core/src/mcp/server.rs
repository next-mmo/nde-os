use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// MCP Server — exposes NDE-OS tools as an MCP server.
/// Other AI agents (Claude Code, Cursor, etc.) can connect to use NDE-OS sandboxed tools.
pub struct McpServer {
    tools: Vec<McpExposedTool>,
}

#[derive(Debug, Clone)]
struct McpExposedTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

impl McpServer {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
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

                if tool_name.starts_with("nde_kanban_") {
                    let empty = json!({});
                    let params = req.params.as_ref().unwrap_or(&empty);
                    match crate::mcp::kanban::execute(tool_name, params) {
                        Ok(result) => json!({
                            "jsonrpc": "2.0",
                            "id": req.id,
                            "result": {
                                "content": [{
                                    "type": "text",
                                    "text": result
                                }]
                            }
                        }),
                        Err(e) => json!({
                            "jsonrpc": "2.0",
                            "id": req.id,
                            "error": {
                                "code": -32603,
                                "message": format!("Kanban tool error: {}", e)
                            }
                        })
                    }
                } else {
                    // Will be connected to ToolRegistry in future
                    json!({
                        "jsonrpc": "2.0",
                        "id": req.id,
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": format!("Tool '{}' execution pending — connect to ToolRegistry", tool_name)
                            }]
                        }
                    })
                }
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
                        .write_all(
                            format!("{}\n", serde_json::to_string(&error_resp)?).as_bytes(),
                        )
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
