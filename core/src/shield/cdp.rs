//! Minimal CDP (Chrome DevTools Protocol) client over WebSocket.
//!
//! Provides just enough CDP coverage to drive a headless Shield Browser
//! for automated page navigation and content extraction. Communicates
//! with Chromium/Camoufox via the `--remote-debugging-port` WebSocket.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{oneshot, Mutex};

// ─── CDP Message Types ─────────────────────────────────────────────

#[derive(Serialize)]
struct CdpRequest {
    id: i64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
struct CdpResponse {
    id: Option<i64>,
    result: Option<serde_json::Value>,
    error: Option<CdpError>,
    method: Option<String>,
    params: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
struct CdpError {
    code: i64,
    message: String,
}

// ─── CDP Client ────────────────────────────────────────────────────

/// A minimal CDP client that connects to a browser's debugging WebSocket.
///
/// Supports: `Page.navigate`, `Page.enable`, `Runtime.evaluate`,
/// `DOM.getDocument`, and waiting for page load events.
pub struct CdpClient {
    writer: Arc<Mutex<tokio::io::WriteHalf<tokio::net::TcpStream>>>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<CdpResponse>>>>,
    events: Arc<Mutex<Vec<CdpResponse>>>,
    next_id: AtomicI64,
    _reader_handle: tokio::task::JoinHandle<()>,
}

impl CdpClient {
    /// Connect to a browser's CDP WebSocket endpoint.
    ///
    /// Discovers the WebSocket URL by querying `http://127.0.0.1:{port}/json/version`,
    /// then performs a raw WebSocket upgrade handshake over TCP.
    pub async fn connect(cdp_port: u16) -> Result<Self> {
        let ws_url = Self::discover_ws_url(cdp_port).await?;
        tracing::info!(ws_url = %ws_url, "Connecting to CDP WebSocket");

        // Parse the WebSocket URL to get host, port, path
        let url_without_scheme = ws_url
            .strip_prefix("ws://")
            .context("CDP WebSocket URL must start with ws://")?;

        let (host_port, path) = url_without_scheme
            .split_once('/')
            .unwrap_or((url_without_scheme, ""));
        let path = format!("/{}", path);

        // Connect raw TCP
        let stream = TcpStream::connect(host_port)
            .await
            .context("Failed to connect to CDP WebSocket")?;

        // WebSocket upgrade handshake (RFC 6455)
        let key = base64_encode_ws_key();
        let upgrade_request = format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {}\r\n\
             Sec-WebSocket-Version: 13\r\n\
             \r\n",
            path, host_port, key
        );

        let (mut reader, mut writer) = tokio::io::split(stream);
        writer
            .write_all(upgrade_request.as_bytes())
            .await
            .context("Failed to send WebSocket upgrade")?;

        // Read upgrade response (consume headers)
        let mut response_buf = vec![0u8; 4096];
        let n = reader
            .read(&mut response_buf)
            .await
            .context("Failed to read WebSocket upgrade response")?;
        let response_str = String::from_utf8_lossy(&response_buf[..n]);

        if !response_str.contains("101") {
            anyhow::bail!(
                "WebSocket upgrade failed. Response: {}",
                response_str.lines().next().unwrap_or("(empty)")
            );
        }

        tracing::info!("CDP WebSocket connected");

        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<CdpResponse>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let events: Arc<Mutex<Vec<CdpResponse>>> = Arc::new(Mutex::new(Vec::new()));
        let writer = Arc::new(Mutex::new(writer));

        // Spawn reader task
        let pending_clone = Arc::clone(&pending);
        let events_clone = Arc::clone(&events);
        let reader_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 1024 * 1024]; // 1MB buffer
            let mut partial = Vec::new();

            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        partial.extend_from_slice(&buf[..n]);

                        // Try to extract complete WebSocket frames
                        while let Some((payload, consumed)) = extract_ws_frame(&partial) {
                            partial.drain(..consumed);

                            if let Ok(msg) =
                                serde_json::from_slice::<CdpResponse>(&payload)
                            {
                                if let Some(id) = msg.id {
                                    // Response to a request
                                    let mut pending = pending_clone.lock().await;
                                    if let Some(sender) = pending.remove(&id) {
                                        let _ = sender.send(msg);
                                    }
                                } else {
                                    // Event
                                    let mut evts = events_clone.lock().await;
                                    evts.push(msg);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("CDP reader error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            writer,
            pending,
            events,
            next_id: AtomicI64::new(1),
            _reader_handle: reader_handle,
        })
    }

    /// Send a CDP command and wait for the response.
    pub async fn send(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let request = CdpRequest {
            id,
            method: method.to_string(),
            params,
        };

        let json = serde_json::to_vec(&request)?;
        let frame = build_ws_frame(&json);

        // Register pending response
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }

        // Send
        {
            let mut writer = self.writer.lock().await;
            writer
                .write_all(&frame)
                .await
                .context("Failed to send CDP command")?;
        }

        // Wait for response with timeout
        let response = tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .context("CDP command timed out")?
            .context("CDP response channel closed")?;

        if let Some(err) = response.error {
            anyhow::bail!("CDP error ({}): {}", err.code, err.message);
        }

        Ok(response.result.unwrap_or(serde_json::Value::Null))
    }

    /// Wait for a specific CDP event to appear.
    pub async fn wait_for_event(
        &self,
        event_name: &str,
        timeout: std::time::Duration,
    ) -> Result<serde_json::Value> {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            // Check existing events
            {
                let mut events = self.events.lock().await;
                if let Some(idx) = events
                    .iter()
                    .position(|e| e.method.as_deref() == Some(event_name))
                {
                    let event = events.remove(idx);
                    return Ok(event.params.unwrap_or(serde_json::Value::Null));
                }
            }

            if tokio::time::Instant::now() >= deadline {
                anyhow::bail!("Timed out waiting for CDP event: {}", event_name);
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// Navigate to a URL and wait for the page to finish loading.
    pub async fn navigate_and_wait(
        &self,
        url: &str,
        timeout: std::time::Duration,
    ) -> Result<()> {
        // Enable Page events
        self.send("Page.enable", None).await?;

        // Navigate
        self.send(
            "Page.navigate",
            Some(serde_json::json!({ "url": url })),
        )
        .await?;

        // Wait for loadEventFired
        self.wait_for_event("Page.loadEventFired", timeout).await?;

        // Extra delay for JS-rendered content to settle
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        Ok(())
    }

    /// Execute JavaScript in the page and return the result as a string.
    pub async fn evaluate(&self, expression: &str) -> Result<String> {
        let result = self
            .send(
                "Runtime.evaluate",
                Some(serde_json::json!({
                    "expression": expression,
                    "returnByValue": true,
                })),
            )
            .await?;

        // Extract the value from the result
        let value = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Ok(value.to_string())
    }

    /// Get the page title.
    pub async fn get_title(&self) -> Result<String> {
        self.evaluate("document.title").await
    }

    /// Get the full text content of the page body.
    pub async fn get_body_text(&self) -> Result<String> {
        self.evaluate("document.body.innerText").await
    }

    /// Get the current page URL.
    pub async fn get_url(&self) -> Result<String> {
        self.evaluate("window.location.href").await
    }

    /// Get all links on the page as JSON string.
    pub async fn get_links(&self) -> Result<String> {
        self.evaluate(
            r#"JSON.stringify(
                Array.from(document.querySelectorAll('a[href]'))
                    .slice(0, 100)
                    .map(a => ({ text: a.innerText.trim().substring(0, 100), href: a.href }))
                    .filter(l => l.text.length > 0 && l.href.startsWith('http'))
            )"#,
        )
        .await
    }

    /// Discover the CDP WebSocket URL by querying the JSON endpoint.
    async fn discover_ws_url(port: u16) -> Result<String> {
        let url = format!("http://127.0.0.1:{}/json/version", port);

        // Retry a few times — the browser may take a moment to start
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        let mut last_err = None;
        for attempt in 0..10 {
            match client.get(&url).send().await {
                Ok(resp) => {
                    let json: serde_json::Value = resp.json().await?;
                    if let Some(ws_url) = json.get("webSocketDebuggerUrl").and_then(|v| v.as_str())
                    {
                        return Ok(ws_url.to_string());
                    }
                    anyhow::bail!("CDP /json/version missing webSocketDebuggerUrl");
                }
                Err(e) => {
                    last_err = Some(e);
                    if attempt < 9 {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }
        }

        Err(last_err
            .map(|e| anyhow::anyhow!("Failed to discover CDP WebSocket after 10 tries: {}", e))
            .unwrap_or_else(|| anyhow::anyhow!("Failed to discover CDP WebSocket")))
    }
}

// ─── WebSocket Frame Helpers (RFC 6455, minimal) ───────────────────

/// Generate a random base64-encoded WebSocket key.
fn base64_encode_ws_key() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let bytes: [u8; 16] = rng.random();

    // Simple base64 encoding
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let combined = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((combined >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((combined >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((combined >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(combined & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Build a masked WebSocket text frame (client → server must be masked per RFC 6455).
fn build_ws_frame(payload: &[u8]) -> Vec<u8> {
    use rand::RngExt;
    let mut rng = rand::rng();
    let mask_key: [u8; 4] = rng.random();

    let mut frame = Vec::new();

    // FIN + text opcode
    frame.push(0x81);

    // Payload length with mask bit set
    let len = payload.len();
    if len < 126 {
        frame.push(0x80 | (len as u8));
    } else if len < 65536 {
        frame.push(0x80 | 126);
        frame.push((len >> 8) as u8);
        frame.push((len & 0xFF) as u8);
    } else {
        frame.push(0x80 | 127);
        for i in (0..8).rev() {
            frame.push(((len >> (i * 8)) & 0xFF) as u8);
        }
    }

    // Masking key
    frame.extend_from_slice(&mask_key);

    // Masked payload
    for (i, byte) in payload.iter().enumerate() {
        frame.push(byte ^ mask_key[i % 4]);
    }

    frame
}

/// Extract a complete WebSocket frame from a buffer.
/// Returns `(payload_bytes, total_bytes_consumed)`.
fn extract_ws_frame(buf: &[u8]) -> Option<(Vec<u8>, usize)> {
    if buf.len() < 2 {
        return None;
    }

    let _opcode = buf[0] & 0x0F;
    let masked = (buf[1] & 0x80) != 0;
    let mut payload_len = (buf[1] & 0x7F) as usize;
    let mut offset = 2;

    if payload_len == 126 {
        if buf.len() < 4 {
            return None;
        }
        payload_len = ((buf[2] as usize) << 8) | (buf[3] as usize);
        offset = 4;
    } else if payload_len == 127 {
        if buf.len() < 10 {
            return None;
        }
        payload_len = 0;
        for i in 0..8 {
            payload_len = (payload_len << 8) | (buf[2 + i] as usize);
        }
        offset = 10;
    }

    let mask_len = if masked { 4 } else { 0 };
    let total_len = offset + mask_len + payload_len;

    if buf.len() < total_len {
        return None;
    }

    let mask_key = if masked {
        Some(&buf[offset..offset + 4])
    } else {
        None
    };
    let data_start = offset + mask_len;
    let mut payload = buf[data_start..data_start + payload_len].to_vec();

    if let Some(mask) = mask_key {
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
    }

    Some((payload, total_len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_frame_roundtrip() {
        let payload = b"hello world";
        let frame = build_ws_frame(payload);

        // Frame should be at least 2 (header) + 4 (mask) + payload bytes
        assert!(frame.len() >= 2 + 4 + payload.len());

        // First byte: FIN + text opcode
        assert_eq!(frame[0], 0x81);
    }

    #[test]
    fn test_extract_unmasked_frame() {
        // Build an unmasked server-to-client frame
        let payload = b"{\"id\":1,\"result\":{}}";
        let mut frame = vec![0x81]; // FIN + text
        frame.push(payload.len() as u8); // Length, no mask bit
        frame.extend_from_slice(payload);

        let (extracted, consumed) = extract_ws_frame(&frame).unwrap();
        assert_eq!(extracted, payload.to_vec());
        assert_eq!(consumed, frame.len());
    }

    #[test]
    fn test_base64_key_length() {
        let key = base64_encode_ws_key();
        // 16 bytes → ~24 base64 characters
        assert!(key.len() >= 20);
        assert!(key.len() <= 28);
    }
}
