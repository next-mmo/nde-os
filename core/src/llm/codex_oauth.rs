//! Codex OAuth — "Sign in with ChatGPT" PKCE flow, token storage, and LLM provider.

use super::{LlmProvider, LlmResponse, Message, ToolDef};
use anyhow::{Context, Result};
use async_trait::async_trait;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

// ── OAuth constants (matches Codex CLI) ──────────────────────────────────────

const AUTH_DOMAIN: &str = "https://auth.openai.com";
const AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CLIENT_ID: &str = "app_sZxMBRNokCMBZwFUhGzMDGMr";
const AUDIENCE: &str = "https://api.openai.com/v1";
const SCOPE: &str = "openid profile email offline_access";
const CALLBACK_PORT: u16 = 14551;

fn redirect_uri() -> String {
    format!("http://localhost:{}/auth/callback", CALLBACK_PORT)
}

// ── PKCE ─────────────────────────────────────────────────────────────────────

/// PKCE code verifier + challenge (S256).
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

impl PkceChallenge {
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen::<u8>()).collect();
        let verifier = base64url_encode(&bytes);

        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let digest = hasher.finalize();
        let challenge = base64url_encode(&digest);

        Self { verifier, challenge }
    }
}

fn base64url_encode(data: &[u8]) -> String {
    use sha2::digest::generic_array::GenericArray;
    let _ = GenericArray::<u8, sha2::digest::consts::U0>::default(); // unused — just for the import
    let encoded = data
        .iter()
        .fold(String::new(), |mut acc, b| {
            use std::fmt::Write;
            write!(acc, "{:02x}", b).ok();
            acc
        });
    // Proper base64url without padding
    let b64 = base64_url_from_bytes(data);
    b64
}

/// Raw base64url encoding (no padding, URL-safe alphabet).
fn base64_url_from_bytes(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };
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

// ── OAuth Flow ───────────────────────────────────────────────────────────────

/// State for an in-progress OAuth flow.
#[derive(Debug, Clone)]
pub struct OAuthFlowState {
    pub pkce: PkceChallenge,
    pub state: String,
    pub auth_url: String,
}

/// Start a new OAuth flow — returns the auth URL and PKCE state.
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

/// Exchange an authorization code for tokens.
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
        return Err(anyhow::anyhow!("Token exchange failed ({}): {}", status, text));
    }

    resp.json::<TokenResponse>()
        .await
        .context("Failed to parse token response")
}

/// Refresh an access token using a refresh token.
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
        return Err(anyhow::anyhow!("Token refresh failed ({}): {}", status, text));
    }

    resp.json::<TokenResponse>()
        .await
        .context("Failed to parse refresh response")
}

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

// ── Token Storage ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodexTokenStore {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64, // Unix timestamp
    pub email: Option<String>,
    pub plan_type: Option<String>,
}

impl CodexTokenStore {
    pub fn token_path(data_dir: &Path) -> PathBuf {
        data_dir.join("codex_tokens.json")
    }

    pub fn load(data_dir: &Path) -> Result<Self> {
        let path = Self::token_path(data_dir);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("No Codex tokens at {}", path.display()))?;
        serde_json::from_str(&content).context("Invalid token file")
    }

    pub fn save(&self, data_dir: &Path) -> Result<()> {
        let path = Self::token_path(data_dir);
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now >= self.expires_at - 60 // 60s buffer
    }

    /// Get a valid access token, auto-refreshing if expired.
    pub async fn get_valid_token(&mut self, data_dir: &Path) -> Result<String> {
        if !self.is_expired() {
            return Ok(self.access_token.clone());
        }

        tracing::info!("Codex OAuth token expired, refreshing...");
        let resp = refresh_token(&self.refresh_token).await?;
        self.access_token = resp.access_token;
        if let Some(rt) = resp.refresh_token {
            self.refresh_token = rt;
        }
        self.expires_at = chrono::Utc::now().timestamp() + resp.expires_in as i64;
        self.save(data_dir)?;
        Ok(self.access_token.clone())
    }

    /// Build from a token response.
    pub fn from_response(resp: &TokenResponse) -> Self {
        // Try to decode email from id_token JWT (simple base64 decode of payload)
        let email = resp.id_token.as_ref().and_then(|t| decode_jwt_email(t));

        Self {
            access_token: resp.access_token.clone(),
            refresh_token: resp.refresh_token.clone().unwrap_or_default(),
            expires_at: chrono::Utc::now().timestamp() + resp.expires_in as i64,
            email,
            plan_type: None,
        }
    }
}

/// Decode email from a JWT id_token (no verification, just payload extraction).
fn decode_jwt_email(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    // Add padding for base64
    let payload = parts[1];
    let padded = match payload.len() % 4 {
        2 => format!("{}==", payload),
        3 => format!("{}=", payload),
        _ => payload.to_string(),
    };
    // base64url → standard base64
    let standard = padded.replace('-', "+").replace('_', "/");
    let bytes = base64_decode(&standard)?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    json.get("email").and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Simple base64 decoder for JWT payloads.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = Vec::new();
    let bytes: Vec<u8> = input.bytes().filter(|b| *b != b'=').collect();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = TABLE.iter().position(|&c| c == bytes[i])? as u32;
        let b1 = if i + 1 < bytes.len() { TABLE.iter().position(|&c| c == bytes[i + 1])? as u32 } else { 0 };
        let b2 = if i + 2 < bytes.len() { TABLE.iter().position(|&c| c == bytes[i + 2])? as u32 } else { 0 };
        let b3 = if i + 3 < bytes.len() { TABLE.iter().position(|&c| c == bytes[i + 3])? as u32 } else { 0 };
        let triple = (b0 << 18) | (b1 << 12) | (b2 << 6) | b3;
        output.push(((triple >> 16) & 0xFF) as u8);
        if i + 2 < bytes.len() { output.push(((triple >> 8) & 0xFF) as u8); }
        if i + 3 < bytes.len() { output.push((triple & 0xFF) as u8); }
        i += 4;
    }
    Some(output)
}

// ── OAuth Status ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexOAuthStatus {
    pub authenticated: bool,
    pub email: Option<String>,
    pub plan_type: Option<String>,
}

impl CodexOAuthStatus {
    pub fn from_store(data_dir: &Path) -> Self {
        match CodexTokenStore::load(data_dir) {
            Ok(store) if !store.access_token.is_empty() => Self {
                authenticated: true,
                email: store.email,
                plan_type: store.plan_type,
            },
            _ => Self {
                authenticated: false,
                email: None,
                plan_type: None,
            },
        }
    }
}

// ── LLM Provider ─────────────────────────────────────────────────────────────

/// Codex OAuth provider — wraps OpenAI-compat with OAuth token auth.
pub struct CodexOAuthProvider {
    client: reqwest::Client,
    model: String,
    data_dir: PathBuf,
    token_store: tokio::sync::Mutex<CodexTokenStore>,
}

impl CodexOAuthProvider {
    pub fn new(model: &str, data_dir: &Path) -> Result<Self> {
        let store = CodexTokenStore::load(data_dir)
            .unwrap_or_default();
        Ok(Self {
            client: reqwest::Client::new(),
            model: model.to_string(),
            data_dir: data_dir.to_path_buf(),
            token_store: tokio::sync::Mutex::new(store),
        })
    }
}

#[async_trait]
impl LlmProvider for CodexOAuthProvider {
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse> {
        // Get a valid token, refreshing if needed
        let mut store = self.token_store.lock().await;
        let token = store.get_valid_token(&self.data_dir).await
            .context("Failed to get valid Codex OAuth token — try signing in again")?;
        drop(store);

        // Delegate to openai_compat logic
        let provider = super::openai_compat::OpenAiCompatProvider::new(
            "https://api.openai.com/v1",
            &self.model,
            &token,
        );
        provider.chat(messages, tools).await
    }

    fn name(&self) -> &str { "codex_oauth" }
}

// ── Callback Server ──────────────────────────────────────────────────────────

/// Run a one-shot HTTP server to receive the OAuth callback.
/// Returns the authorization code on success.
pub async fn run_callback_server(expected_state: &str) -> Result<String> {
    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = TcpListener::bind(format!("127.0.0.1:{}", CALLBACK_PORT))
        .await
        .with_context(|| format!("Failed to bind callback server on port {}", CALLBACK_PORT))?;

    tracing::info!(port = CALLBACK_PORT, "OAuth callback server listening");

    let (mut stream, _) = listener.accept().await?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse GET /auth/callback?code=xxx&state=yyy
    let first_line = request.lines().next().unwrap_or("");
    let path_query = first_line.split_whitespace().nth(1).unwrap_or("");
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

    // Send success HTML response
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

// ── Convenience: full flow ───────────────────────────────────────────────────

/// Complete OAuth flow: start server, wait for callback, exchange code, persist tokens.
pub async fn complete_oauth_flow(flow: &OAuthFlowState, data_dir: &Path) -> Result<CodexTokenStore> {
    let code = run_callback_server(&flow.state).await?;
    let tokens = exchange_code(&code, &flow.pkce.verifier).await?;
    let store = CodexTokenStore::from_response(&tokens);
    store.save(data_dir)?;
    tracing::info!(email = ?store.email, "Codex OAuth completed");
    Ok(store)
}
