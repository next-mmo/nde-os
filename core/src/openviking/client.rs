//! Async HTTP client for the OpenViking REST API (`/api/v1/*`).
//!
//! Mirrors the `ov_cli` Rust binary's reqwest-based approach but exposes
//! a programmatic `VikingClient` struct instead of a CLI interface.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};

/// HTTP client for the OpenViking context database.
pub struct VikingClient {
    base_url: String,
    http: reqwest::Client,
}

impl VikingClient {
    /// Create a new client pointing at `base_url` (e.g. `http://localhost:1933`).
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
        }
    }

    // ── Health ───────────────────────────────────────────────────────────

    /// Check if the OpenViking server is reachable.
    pub async fn health(&self) -> Result<bool> {
        match self.http.get(format!("{}/health", self.base_url))
            .send()
            .await
        {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Get system status.
    pub async fn status(&self) -> Result<Value> {
        self.get("/api/v1/system/status").await
    }

    // ── Filesystem ──────────────────────────────────────────────────────

    /// List directory contents at a `viking://` URI.
    pub async fn ls(&self, uri: &str, recursive: bool) -> Result<Value> {
        let mut url = format!("{}/api/v1/fs/ls?uri={}", self.base_url, urlencoding::encode(uri));
        if recursive {
            url.push_str("&recursive=true");
        }
        self.get_raw(&url).await
    }

    /// Get directory tree.
    pub async fn tree(&self, uri: &str, depth: Option<u32>) -> Result<Value> {
        let mut url = format!("{}/api/v1/fs/tree?uri={}", self.base_url, urlencoding::encode(uri));
        if let Some(d) = depth {
            url.push_str(&format!("&depth={}", d));
        }
        self.get_raw(&url).await
    }

    /// Get metadata/status of a resource.
    pub async fn stat(&self, uri: &str) -> Result<Value> {
        let url = format!("{}/api/v1/fs/stat?uri={}", self.base_url, urlencoding::encode(uri));
        self.get_raw(&url).await
    }

    /// Create a directory.
    pub async fn mkdir(&self, uri: &str) -> Result<Value> {
        self.post("/api/v1/fs/mkdir", json!({ "uri": uri })).await
    }

    /// Delete a resource.
    pub async fn rm(&self, uri: &str) -> Result<Value> {
        let url = format!("{}/api/v1/fs?uri={}", self.base_url, urlencoding::encode(uri));
        let resp = self.http.delete(&url).send().await
            .context("DELETE request failed")?;
        Self::parse_response(resp).await
    }

    /// Move/rename a resource.
    pub async fn mv(&self, src: &str, dst: &str) -> Result<Value> {
        self.post("/api/v1/fs/mv", json!({ "source": src, "destination": dst })).await
    }

    // ── Content ─────────────────────────────────────────────────────────

    /// Read L0 abstract (~100 tokens summary).
    pub async fn abstract_(&self, uri: &str) -> Result<Value> {
        let url = format!("{}/api/v1/content/abstract?uri={}", self.base_url, urlencoding::encode(uri));
        self.get_raw(&url).await
    }

    /// Read L1 overview (~2k tokens).
    pub async fn overview(&self, uri: &str) -> Result<Value> {
        let url = format!("{}/api/v1/content/overview?uri={}", self.base_url, urlencoding::encode(uri));
        self.get_raw(&url).await
    }

    /// Read L2 full content.
    pub async fn read(&self, uri: &str) -> Result<Value> {
        let url = format!("{}/api/v1/content/read?uri={}", self.base_url, urlencoding::encode(uri));
        self.get_raw(&url).await
    }

    // ── Search ──────────────────────────────────────────────────────────

    /// Semantic search (directory-recursive retrieval).
    pub async fn find(&self, query: &str) -> Result<Value> {
        self.post("/api/v1/search/find", json!({ "query": query })).await
    }

    /// Context-aware search.
    pub async fn search(&self, query: &str, uri: Option<&str>) -> Result<Value> {
        let mut body = json!({ "query": query });
        if let Some(u) = uri {
            body["uri"] = Value::String(u.to_string());
        }
        self.post("/api/v1/search/search", body).await
    }

    /// Pattern search (like grep).
    pub async fn grep(&self, pattern: &str, uri: Option<&str>) -> Result<Value> {
        let mut body = json!({ "pattern": pattern });
        if let Some(u) = uri {
            body["uri"] = Value::String(u.to_string());
        }
        self.post("/api/v1/search/grep", body).await
    }

    /// File glob pattern matching.
    pub async fn glob(&self, pattern: &str, uri: Option<&str>) -> Result<Value> {
        let mut body = json!({ "pattern": pattern });
        if let Some(u) = uri {
            body["uri"] = Value::String(u.to_string());
        }
        self.post("/api/v1/search/glob", body).await
    }

    // ── Resources ───────────────────────────────────────────────────────

    /// Add a resource (URL or local path).
    pub async fn add_resource(&self, source: &str, wait: bool) -> Result<Value> {
        let mut body = json!({ "source": source });
        if wait {
            body["wait"] = Value::Bool(true);
        }
        self.post("/api/v1/resources", body).await
    }

    /// Add a skill.
    pub async fn add_skill(&self, name: &str, content: &str) -> Result<Value> {
        self.post("/api/v1/skills", json!({ "name": name, "content": content })).await
    }

    // ── Relations ───────────────────────────────────────────────────────

    /// Get relations for a URI.
    pub async fn relations(&self, uri: &str) -> Result<Value> {
        let url = format!("{}/api/v1/relations?uri={}", self.base_url, urlencoding::encode(uri));
        self.get_raw(&url).await
    }

    /// Create a relation link.
    pub async fn link(&self, source: &str, target: &str, link_type: Option<&str>) -> Result<Value> {
        let mut body = json!({ "source": source, "target": target });
        if let Some(lt) = link_type {
            body["type"] = Value::String(lt.to_string());
        }
        self.post("/api/v1/relations/link", body).await
    }

    // ── Sessions ────────────────────────────────────────────────────────

    /// Create a new session.
    pub async fn session_create(&self) -> Result<Value> {
        self.post("/api/v1/sessions", json!({})).await
    }

    /// List sessions.
    pub async fn session_list(&self) -> Result<Value> {
        self.get("/api/v1/sessions").await
    }

    /// Get session details.
    pub async fn session_get(&self, id: &str) -> Result<Value> {
        self.get(&format!("/api/v1/sessions/{}", id)).await
    }

    /// Add a message to a session.
    pub async fn session_add_message(&self, id: &str, role: &str, content: &str) -> Result<Value> {
        self.post(
            &format!("/api/v1/sessions/{}/messages", id),
            json!({ "role": role, "content": content }),
        ).await
    }

    /// Commit a session (triggers memory extraction).
    pub async fn session_commit(&self, id: &str) -> Result<Value> {
        self.post(&format!("/api/v1/sessions/{}/commit", id), json!({})).await
    }

    /// Delete a session.
    pub async fn session_delete(&self, id: &str) -> Result<Value> {
        let url = format!("{}/api/v1/sessions/{}", self.base_url, id);
        let resp = self.http.delete(&url).send().await
            .context("DELETE request failed")?;
        Self::parse_response(resp).await
    }

    // ── Internal helpers ────────────────────────────────────────────────

    async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        self.get_raw(&url).await
    }

    async fn get_raw(&self, url: &str) -> Result<Value> {
        let resp = self.http.get(url).send().await
            .with_context(|| format!("GET {} failed", url))?;
        Self::parse_response(resp).await
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.http.post(&url)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("POST {} failed", url))?;
        Self::parse_response(resp).await
    }

    async fn parse_response(resp: reqwest::Response) -> Result<Value> {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!(
                "OpenViking API error (HTTP {}): {}",
                status.as_u16(),
                body.chars().take(500).collect::<String>()
            ));
        }

        serde_json::from_str(&body)
            .with_context(|| format!("Failed to parse OpenViking response: {}", body.chars().take(200).collect::<String>()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = VikingClient::new("http://localhost:1933");
        assert_eq!(client.base_url, "http://localhost:1933");
    }

    #[test]
    fn test_trailing_slash_stripped() {
        let client = VikingClient::new("http://localhost:1933/");
        assert_eq!(client.base_url, "http://localhost:1933");
    }

    #[tokio::test]
    async fn test_health_returns_false_when_offline() {
        // No server running → health should return false, not error
        let client = VikingClient::new("http://127.0.0.1:19339");
        let ok = client.health().await.unwrap();
        assert!(!ok);
    }
}
