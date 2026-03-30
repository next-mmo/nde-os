//! Codex OAuth — "Sign in with ChatGPT" for NDE-OS.
//!
//! Two-tier auth strategy:
//!   1. Read from Codex CLI store (`~/.codex/auth.json`) if available
//!   2. Built-in PKCE OAuth flow (no external CLI needed)
//!
//! Tokens are saved in Codex CLI-compatible format so both paths interoperate.

use super::{LlmProvider, LlmResponse, Message, StopReason, ToolDef, Usage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

// ── OAuth constants (matches Codex CLI) ──────────────────────────────────────

const AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUDIENCE: &str = "https://api.openai.com/v1";
const SCOPE: &str = "openid profile email offline_access";
const CALLBACK_PORT: u16 = 1455;

fn redirect_uri() -> String {
    format!("http://localhost:{}/auth/callback", CALLBACK_PORT)
}

// ── PKCE ─────────────────────────────────────────────────────────────────────

fn base64url_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() {
            data[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < data.len() {
            data[i + 2] as u32
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if i + 1 < data.len() {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        }
        if i + 2 < data.len() {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        }
        i += 3;
    }
    result
}

#[derive(Debug, Clone)]
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

impl PkceChallenge {
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rand::Rng::gen::<u8>(&mut rng)).collect();
        let verifier = base64url_encode(&bytes);
        let digest = Sha256::digest(verifier.as_bytes());
        let challenge = base64url_encode(&digest);
        Self {
            verifier,
            challenge,
        }
    }
}

// ── Codex CLI Auth Store ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexCliAuth {
    pub auth_mode: Option<String>,
    #[serde(rename = "OPENAI_API_KEY")]
    pub openai_api_key: Option<String>,
    pub tokens: Option<CodexCliTokens>,
    pub last_refresh: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexCliTokens {
    pub id_token: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub account_id: Option<String>,
}

fn home_dir() -> Result<PathBuf> {
    let home = if cfg!(windows) {
        std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .context("Cannot determine home directory")?
    } else {
        std::env::var("HOME").context("Cannot determine home directory")?
    };
    Ok(PathBuf::from(home))
}

fn codex_auth_path() -> Result<PathBuf> {
    Ok(home_dir()?.join(".codex").join("auth.json"))
}

/// Read tokens from Codex CLI store (`~/.codex/auth.json`).
pub fn read_codex_auth() -> Result<CodexCliAuth> {
    let path = codex_auth_path()?;
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("No auth file at {}", path.display()))?;
    serde_json::from_str(&content).context("Failed to parse auth.json")
}

/// Save tokens in Codex CLI-compatible format.
fn save_codex_auth(auth: &CodexCliAuth) -> Result<()> {
    let path = codex_auth_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(auth)?;
    std::fs::write(&path, content)?;
    tracing::info!("Saved auth to {}", path.display());
    Ok(())
}

/// Get an access token — tries Codex CLI store first.
pub fn get_codex_access_token() -> Result<String> {
    let auth = read_codex_auth()?;
    let tokens = auth.tokens.context("No tokens")?;
    tokens
        .access_token
        .filter(|t| !t.is_empty())
        .context("No access token found")
}

// ── OAuth Flow State ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct OAuthFlowState {
    pub pkce: PkceChallenge,
    pub state: String,
    pub auth_url: String,
}

/// Build the authorization URL with PKCE.
pub fn start_oauth_flow() -> OAuthFlowState {
    let pkce = PkceChallenge::generate();
    let state = uuid::Uuid::new_v4().to_string();
    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}&scope={}&audience={}",
        AUTHORIZE_URL,
        CLIENT_ID,
        urlencoding::encode(&redirect_uri()),
        pkce.challenge,
        state,
        urlencoding::encode(SCOPE),
        urlencoding::encode(AUDIENCE),
    );
    OAuthFlowState {
        pkce,
        state,
        auth_url,
    }
}

// ── Token Exchange ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: u64,
    #[serde(default)]
    pub token_type: String,
    pub id_token: Option<String>,
}

pub async fn exchange_code(code: &str, verifier: &str) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("code", code),
            ("redirect_uri", &redirect_uri()),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .context("Failed to exchange OAuth code")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Token exchange failed ({}): {}",
            status,
            text
        ));
    }

    resp.json::<TokenResponse>()
        .await
        .context("Failed to parse token response")
}

pub async fn refresh_token(refresh: &str) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh),
        ])
        .send()
        .await
        .context("Failed to refresh token")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Token refresh failed ({}): {}",
            status,
            text
        ));
    }

    resp.json::<TokenResponse>()
        .await
        .context("Failed to parse refresh response")
}

// ── Callback Server (built-in PKCE flow) ─────────────────────────────────────

/// Run a one-shot HTTP server to receive the OAuth callback.
/// Uses SO_REUSEADDR + eviction + timeout for robustness.
pub async fn run_callback_server(expected_state: &str) -> Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let addr = format!("127.0.0.1:{}", CALLBACK_PORT);

    let std_listener = match std::net::TcpListener::bind(&addr) {
        Ok(s) => {
            s.set_nonblocking(true)?;
            s
        }
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            tracing::warn!(port = CALLBACK_PORT, "Port in use, evicting stale listener");
            if let Ok(mut evict) = std::net::TcpStream::connect(&addr) {
                use std::io::Write;
                let _ = evict.write_all(b"GET /evict HTTP/1.1\r\n\r\n");
                let _ = evict.flush();
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            let s = std::net::TcpListener::bind(&addr)
                .with_context(|| format!("Port {} still busy after eviction", CALLBACK_PORT))?;
            s.set_nonblocking(true)?;
            s
        }
        Err(e) => return Err(e.into()),
    };

    let listener = TcpListener::from_std(std_listener)?;
    tracing::info!(port = CALLBACK_PORT, "OAuth callback server listening");

    let accept_result =
        tokio::time::timeout(std::time::Duration::from_secs(300), listener.accept()).await;

    let (mut stream, _) = match accept_result {
        Ok(Ok(conn)) => conn,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("OAuth callback timed out (5 min)")),
    };

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = request.lines().next().unwrap_or("");
    let path_query = first_line.split_whitespace().nth(1).unwrap_or("");

    if path_query == "/evict" {
        return Err(anyhow::anyhow!("Evicted by new OAuth flow"));
    }

    let query = path_query.split('?').nth(1).unwrap_or("");
    let params: std::collections::HashMap<&str, &str> = query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            Some((parts.next()?, parts.next()?))
        })
        .collect();

    let code = params.get("code").copied().unwrap_or("");
    let state = params.get("state").copied().unwrap_or("");

    let html_body = r#"<html><body style="font-family:system-ui;text-align:center;padding:3rem">
        <h2>✓ Signed in successfully!</h2>
        <p>You can close this window and return to NDE-OS.</p>
        <script>setTimeout(()=>window.close(),2000)</script>
    </body></html>"#;

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html_body.len(),
        html_body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;

    if state != expected_state {
        return Err(anyhow::anyhow!("OAuth state mismatch"));
    }
    if code.is_empty() {
        return Err(anyhow::anyhow!("No authorization code in callback"));
    }

    Ok(code.to_string())
}

// ── Full OAuth Flow ──────────────────────────────────────────────────────────

/// Complete OAuth: callback server → code exchange → save in Codex CLI format.
pub async fn complete_oauth_flow(flow: &OAuthFlowState) -> Result<CodexCliAuth> {
    let code = run_callback_server(&flow.state).await?;
    let tokens = exchange_code(&code, &flow.pkce.verifier).await?;

    let auth = CodexCliAuth {
        auth_mode: Some("chatgpt".to_string()),
        openai_api_key: None,
        tokens: Some(CodexCliTokens {
            id_token: tokens.id_token,
            access_token: Some(tokens.access_token),
            refresh_token: tokens.refresh_token,
            account_id: None,
        }),
        last_refresh: Some(chrono::Utc::now().to_rfc3339()),
    };

    save_codex_auth(&auth)?;
    let email = auth
        .tokens
        .as_ref()
        .and_then(|t| t.id_token.as_deref())
        .and_then(decode_jwt_email);
    tracing::info!(
        ?email,
        "OAuth completed, tokens saved to ~/.codex/auth.json"
    );
    Ok(auth)
}

// ── Token Store (for compatibility) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodexTokenStore {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub email: Option<String>,
    pub plan_type: Option<String>,
}

impl CodexTokenStore {
    pub fn load(_data_dir: &Path) -> Result<Self> {
        let auth = read_codex_auth()?;
        let tokens = auth.tokens.context("No tokens")?;
        let email = tokens.id_token.as_deref().and_then(decode_jwt_email);
        Ok(Self {
            access_token: tokens.access_token.unwrap_or_default(),
            refresh_token: tokens.refresh_token.unwrap_or_default(),
            expires_at: 0,
            email,
            plan_type: if auth.auth_mode.as_deref() == Some("chatgpt") {
                Some("ChatGPT Plus/Pro".to_string())
            } else {
                auth.auth_mode
            },
        })
    }

    pub fn save(&self, _data_dir: &Path) -> Result<()> {
        Ok(())
    }
    pub fn is_expired(&self) -> bool {
        false
    }

    pub async fn get_valid_token(&mut self, _data_dir: &Path) -> Result<String> {
        // Try reading fresh from disk (Codex CLI may have refreshed)
        if let Ok(auth) = read_codex_auth() {
            if let Some(tokens) = auth.tokens {
                if let Some(token) = tokens.access_token {
                    self.access_token = token.clone();
                    return Ok(token);
                }
            }
        }
        // Try refreshing ourselves
        if !self.refresh_token.is_empty() {
            tracing::info!("Attempting token refresh...");
            match refresh_token(&self.refresh_token).await {
                Ok(resp) => {
                    self.access_token = resp.access_token.clone();
                    if let Some(rt) = &resp.refresh_token {
                        self.refresh_token = rt.clone();
                    }
                    // Save refreshed tokens in Codex CLI format
                    let auth = CodexCliAuth {
                        auth_mode: Some("chatgpt".to_string()),
                        openai_api_key: None,
                        tokens: Some(CodexCliTokens {
                            id_token: resp.id_token,
                            access_token: Some(resp.access_token),
                            refresh_token: resp.refresh_token,
                            account_id: None,
                        }),
                        last_refresh: Some(chrono::Utc::now().to_rfc3339()),
                    };
                    let _ = save_codex_auth(&auth);
                    return Ok(self.access_token.clone());
                }
                Err(e) => tracing::warn!("Token refresh failed: {}", e),
            }
        }
        Err(anyhow::anyhow!(
            "No valid token — sign in again via LLM Providers"
        ))
    }
}

// ── OAuth Status ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexOAuthStatus {
    pub authenticated: bool,
    pub email: Option<String>,
    pub plan_type: Option<String>,
}

impl CodexOAuthStatus {
    pub fn from_store(_data_dir: &Path) -> Self {
        match read_codex_auth() {
            Ok(auth) => {
                let has_token = auth
                    .tokens
                    .as_ref()
                    .and_then(|t| t.access_token.as_ref())
                    .map_or(false, |t| !t.is_empty());
                if has_token {
                    let email = auth
                        .tokens
                        .as_ref()
                        .and_then(|t| t.id_token.as_deref())
                        .and_then(decode_jwt_email);
                    let plan_type = if auth.auth_mode.as_deref() == Some("chatgpt") {
                        Some("ChatGPT Plus/Pro".to_string())
                    } else {
                        auth.auth_mode
                    };
                    Self {
                        authenticated: true,
                        email,
                        plan_type,
                    }
                } else {
                    Self {
                        authenticated: false,
                        email: None,
                        plan_type: None,
                    }
                }
            }
            _ => Self {
                authenticated: false,
                email: None,
                plan_type: None,
            },
        }
    }
}

// ── LLM Provider ─────────────────────────────────────────────────────────────

pub struct CodexOAuthProvider {
    model: String,
    data_dir: PathBuf,
    workspace_dir: PathBuf,
}

impl CodexOAuthProvider {
    pub fn new(model: &str, data_dir: &Path) -> Result<Self> {
        Ok(Self {
            model: model.to_string(),
            data_dir: data_dir.to_path_buf(),
            workspace_dir: data_dir.join("workspace"),
        })
    }
}

#[async_trait]
impl LlmProvider for CodexOAuthProvider {
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse> {
        if !CodexOAuthStatus::from_store(&self.data_dir).authenticated {
            return Err(anyhow::anyhow!(
                "Codex not authenticated — sign in via LLM Providers settings"
            ));
        }

        std::fs::create_dir_all(&self.workspace_dir).with_context(|| {
            format!(
                "Failed to initialize Codex workspace at {}",
                self.workspace_dir.display()
            )
        })?;

        let prompt = render_exec_prompt(messages, tools);
        let output = tokio::process::Command::new("codex")
            .arg("exec")
            .arg("--skip-git-repo-check")
            .arg("--ephemeral")
            .arg("--model")
            .arg(&self.model)
            .arg("--sandbox")
            .arg("workspace-write")
            .arg("-c")
            .arg("approval_policy=\"never\"")
            .arg("-c")
            .arg("mcp_servers.playwright.enabled=false")
            .arg("-c")
            .arg("mcp_servers.context7.enabled=false")
            .arg("-C")
            .arg(&self.workspace_dir)
            .arg("--json")
            .arg(&prompt)
            .output()
            .await
            .context("Failed to launch Codex CLI")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                format!("codex exited with status {}", output.status)
            };
            return Err(anyhow::anyhow!("Codex CLI error: {}", detail));
        }

        parse_exec_output(&output.stdout)
    }

    fn name(&self) -> &str {
        "codex_oauth"
    }
}

fn render_exec_prompt(messages: &[Message], tools: &[ToolDef]) -> String {
    let mut prompt = String::from(
        "You are acting as the NDE-OS chat assistant.\n\
Continue the conversation transcript below and return only the assistant's next reply.\n\
Use the current workspace when the user asks about files, code, or commands.\n\
Do not include role labels in your final answer.\n",
    );

    if !tools.is_empty() {
        prompt.push_str("\nAvailable NDE-OS tool names for context:\n");
        for tool in tools {
            prompt.push_str("- ");
            prompt.push_str(&tool.name);
            prompt.push('\n');
        }
    }

    prompt.push_str("\nConversation transcript:\n");
    for message in messages {
        match message {
            Message::System { content } => {
                prompt.push_str("\n[system]\n");
                prompt.push_str(content.trim());
                prompt.push('\n');
            }
            Message::User { content } => {
                prompt.push_str("\n[user]\n");
                prompt.push_str(content.trim());
                prompt.push('\n');
            }
            Message::Assistant {
                content,
                tool_calls,
            } => {
                if let Some(content) = content.as_deref() {
                    if !content.trim().is_empty() {
                        prompt.push_str("\n[assistant]\n");
                        prompt.push_str(content.trim());
                        prompt.push('\n');
                    }
                }
                if !tool_calls.is_empty() {
                    prompt.push_str("\n[assistant_tool_calls]\n");
                    for tool_call in tool_calls {
                        prompt.push_str("- ");
                        prompt.push_str(&tool_call.name);
                        prompt.push_str(": ");
                        prompt.push_str(&tool_call.arguments.to_string());
                        prompt.push('\n');
                    }
                }
            }
            Message::Tool {
                tool_call_id,
                content,
            } => {
                prompt.push_str("\n[tool_result ");
                prompt.push_str(tool_call_id);
                prompt.push_str("]\n");
                prompt.push_str(content.trim());
                prompt.push('\n');
            }
        }
    }

    prompt.push_str("\nReply with the next assistant message only.");
    prompt
}

#[derive(Deserialize)]
struct CodexExecEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    item: Option<CodexExecItem>,
    #[serde(default)]
    usage: Option<CodexExecUsage>,
}

#[derive(Deserialize)]
struct CodexExecItem {
    #[serde(rename = "type")]
    item_type: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Deserialize)]
struct CodexExecUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

fn parse_exec_output(stdout: &[u8]) -> Result<LlmResponse> {
    let mut content = String::new();
    let mut usage = None;

    for line in String::from_utf8_lossy(stdout).lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            continue;
        }

        let event: CodexExecEvent = match serde_json::from_str(trimmed) {
            Ok(event) => event,
            Err(_) => continue,
        };

        match event.event_type.as_str() {
            "item.completed" => {
                if let Some(item) = event.item {
                    if item.item_type == "agent_message" {
                        if let Some(text) = item.text {
                            if !content.is_empty() {
                                content.push_str("\n\n");
                            }
                            content.push_str(text.trim());
                        }
                    }
                }
            }
            "turn.completed" => {
                usage = event.usage.map(|u| Usage {
                    prompt_tokens: u.input_tokens,
                    completion_tokens: u.output_tokens,
                });
            }
            _ => {}
        }
    }

    if content.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "Codex CLI returned no assistant message in JSON output"
        ));
    }

    Ok(LlmResponse {
        content: Some(content),
        tool_calls: vec![],
        stop_reason: StopReason::EndTurn,
        usage,
    })
}

// ── JWT Email Extraction ─────────────────────────────────────────────────────

fn decode_jwt_email(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload = parts[1];
    let padded = match payload.len() % 4 {
        2 => format!("{}==", payload),
        3 => format!("{}=", payload),
        _ => payload.to_string(),
    };
    let standard = padded.replace('-', "+").replace('_', "/");
    let bytes = base64_decode(&standard)?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    json.get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = Vec::new();
    let bytes: Vec<u8> = input.bytes().filter(|b| *b != b'=').collect();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = TABLE.iter().position(|&c| c == bytes[i])? as u32;
        let b1 = if i + 1 < bytes.len() {
            TABLE.iter().position(|&c| c == bytes[i + 1])? as u32
        } else {
            0
        };
        let b2 = if i + 2 < bytes.len() {
            TABLE.iter().position(|&c| c == bytes[i + 2])? as u32
        } else {
            0
        };
        let b3 = if i + 3 < bytes.len() {
            TABLE.iter().position(|&c| c == bytes[i + 3])? as u32
        } else {
            0
        };
        let triple = (b0 << 18) | (b1 << 12) | (b2 << 6) | b3;
        output.push(((triple >> 16) & 0xFF) as u8);
        if i + 2 < bytes.len() {
            output.push(((triple >> 8) & 0xFF) as u8);
        }
        if i + 3 < bytes.len() {
            output.push((triple & 0xFF) as u8);
        }
        i += 4;
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_exec_prompt_includes_roles_and_tools() {
        let messages = vec![
            Message::system("System guidance"),
            Message::user("List files"),
            Message::assistant_tool_calls(vec![super::super::ToolCall {
                id: "tool-1".into(),
                name: "file_list".into(),
                arguments: serde_json::json!({ "path": "." }),
            }]),
            Message::tool_result("tool-1", "config\nlogs"),
        ];
        let tools = vec![ToolDef {
            name: "file_list".into(),
            description: "List files".into(),
            parameters: serde_json::json!({ "type": "object" }),
        }];

        let prompt = render_exec_prompt(&messages, &tools);

        assert!(prompt.contains("[system]"));
        assert!(prompt.contains("[user]"));
        assert!(prompt.contains("[assistant_tool_calls]"));
        assert!(prompt.contains("[tool_result tool-1]"));
        assert!(prompt.contains("file_list"));
    }

    #[test]
    fn test_parse_exec_output_extracts_message_and_usage() {
        let stdout = br#"
2026-03-30T18:32:39Z WARN something noisy
{"type":"thread.started","thread_id":"abc"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"OK"}}
{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":4}}
"#;

        let response = parse_exec_output(stdout).unwrap();

        assert_eq!(response.content.as_deref(), Some("OK"));
        assert!(response.tool_calls.is_empty());
        assert_eq!(response.usage.unwrap().prompt_tokens, 10);
    }

    #[test]
    fn test_parse_exec_output_requires_agent_message() {
        let stdout = br#"{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":4}}"#;
        let err = parse_exec_output(stdout).unwrap_err().to_string();
        assert!(err.contains("no assistant message"));
    }
}
