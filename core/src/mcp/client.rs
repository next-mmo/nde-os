use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

/// MCP Client — connects to external MCP servers via stdio transport.
/// Discovers tools from the server and makes them available to the agent.
pub struct McpClient {
    servers: HashMap<String, McpServerConnection>,
}

struct McpServerConnection {
    config: McpServerConfig,
    process: Option<Child>,
    tools: Vec<McpToolDef>,
    request_id: u64,
}

/// Configuration for an MCP server to connect to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

/// Tool definition from an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server_name: String,
}

/// MCP JSON-RPC request.
#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

/// MCP JSON-RPC response.
#[derive(Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    /// Load server configs from a TOML file.
    pub fn from_config_file(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read MCP config file")?;

        #[derive(Deserialize)]
        struct McpConfig {
            #[serde(default)]
            servers: Vec<McpServerConfig>,
        }

        let config: McpConfig = toml::from_str(&content).context("Failed to parse MCP config")?;

        let mut client = Self::new();
        for server_config in config.servers {
            client.add_server(server_config);
        }
        Ok(client)
    }

    /// Add a server configuration (does not start it yet).
    pub fn add_server(&mut self, config: McpServerConfig) {
        let name = config.name.clone();
        self.servers.insert(
            name,
            McpServerConnection {
                config,
                process: None,
                tools: Vec::new(),
                request_id: 0,
            },
        );
    }

    /// Start an MCP server process and discover its tools.
    pub async fn connect(&mut self, server_name: &str) -> Result<Vec<McpToolDef>> {
        let conn = self
            .servers
            .get_mut(server_name)
            .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not configured", server_name))?;

        // Spawn the server process
        let mut cmd = Command::new(&conn.config.command);
        cmd.args(&conn.config.args);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());

        for (k, v) in &conn.config.env {
            cmd.env(k, v);
        }
        if let Some(cwd) = &conn.config.cwd {
            cmd.current_dir(cwd);
        }

        let mut child = cmd.spawn().with_context(|| {
            format!(
                "Failed to spawn MCP server '{}': {}",
                server_name, conn.config.command
            )
        })?;

        // Initialize — send initialize request
        let stdin = child.stdin.as_mut().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);

        conn.request_id += 1;
        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: conn.request_id,
            method: "initialize".into(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "nde-os",
                    "version": "0.2.0"
                }
            })),
        };

        let req_line = serde_json::to_string(&init_req)? + "\n";
        stdin.write_all(req_line.as_bytes()).await?;
        stdin.flush().await?;

        // Read response
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let _resp: JsonRpcResponse = serde_json::from_str(line.trim())?;

        // Send initialized notification
        let notif = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let notif_line = serde_json::to_string(&notif)? + "\n";
        stdin.write_all(notif_line.as_bytes()).await?;
        stdin.flush().await?;

        // List tools
        conn.request_id += 1;
        let tools_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: conn.request_id,
            method: "tools/list".into(),
            params: None,
        };

        let req_line = serde_json::to_string(&tools_req)? + "\n";
        stdin.write_all(req_line.as_bytes()).await?;
        stdin.flush().await?;

        line.clear();
        reader.read_line(&mut line).await?;
        let tools_resp: JsonRpcResponse = serde_json::from_str(line.trim())?;

        if let Some(error) = tools_resp.error {
            return Err(anyhow::anyhow!(
                "MCP tools/list error {}: {}",
                error.code,
                error.message
            ));
        }

        let tools: Vec<McpToolDef> = if let Some(result) = tools_resp.result {
            let tools_array = result
                .get("tools")
                .and_then(|t| t.as_array())
                .cloned()
                .unwrap_or_default();

            tools_array
                .into_iter()
                .filter_map(|t| {
                    Some(McpToolDef {
                        name: t.get("name")?.as_str()?.to_string(),
                        description: t
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                        input_schema: t
                            .get("inputSchema")
                            .cloned()
                            .unwrap_or(serde_json::json!({})),
                        server_name: server_name.to_string(),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        conn.tools = tools.clone();
        conn.process = Some(child);

        tracing::info!(
            server = server_name,
            tool_count = tools.len(),
            "Connected to MCP server"
        );

        Ok(tools)
    }

    /// Call a tool on an MCP server.
    pub async fn call_tool(
        &mut self,
        server_name: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<String> {
        let conn = self
            .servers
            .get_mut(server_name)
            .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not found", server_name))?;

        let child = conn
            .process
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not connected", server_name))?;

        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("MCP server stdin unavailable"))?;

        conn.request_id += 1;
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: conn.request_id,
            method: "tools/call".into(),
            params: Some(serde_json::json!({
                "name": tool_name,
                "arguments": arguments
            })),
        };

        let req_line = serde_json::to_string(&req)? + "\n";
        stdin.write_all(req_line.as_bytes()).await?;
        stdin.flush().await?;

        // Read response — this is simplified, needs proper framing
        let stdout = child.stdout.as_mut().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let resp: JsonRpcResponse = serde_json::from_str(line.trim())?;

        if let Some(error) = resp.error {
            return Err(anyhow::anyhow!(
                "MCP tool error {}: {}",
                error.code,
                error.message
            ));
        }

        let content = resp
            .result
            .and_then(|r| {
                r.get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|block| block.get("text"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "No content returned".to_string());

        Ok(content)
    }

    /// Get all discovered tools from all connected servers.
    pub fn all_tools(&self) -> Vec<McpToolDef> {
        self.servers
            .values()
            .flat_map(|conn| conn.tools.clone())
            .collect()
    }

    /// Convert MCP tools to agent ToolDefs.
    pub fn as_tool_defs(&self) -> Vec<crate::llm::ToolDef> {
        self.all_tools()
            .into_iter()
            .map(|t| crate::llm::ToolDef {
                name: format!("mcp_{}_{}", t.server_name.replace('-', "_"), t.name),
                description: format!("[MCP:{}] {}", t.server_name, t.description),
                parameters: t.input_schema,
            })
            .collect()
    }

    /// Disconnect all servers.
    pub async fn disconnect_all(&mut self) {
        for (name, conn) in &mut self.servers {
            if let Some(mut child) = conn.process.take() {
                let _ = child.kill().await;
                tracing::info!(server = %name, "Disconnected MCP server");
            }
            conn.tools.clear();
        }
    }

    /// List configured server names.
    pub fn server_names(&self) -> Vec<String> {
        self.servers.keys().cloned().collect()
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}
