//! NDE Agent translation provider.
//!
//! Calls the local NDE-OS server's **direct LLM endpoint** (`/api/llm/chat`)
//! for translation instead of going through the agent runtime's tool-calling
//! loop — this avoids the empty-response bug where the agent runtime
//! triggers tools and returns `""` instead of the translated text.
//!
//! Falls back to `/api/agent/chat` if `/api/llm/chat` is not available.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use super::TranslationProvider;

/// NDE Agent-backed translation.
pub struct NdeAgentProvider {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct NdeEnvelope<T> {
    success: bool,
    message: String,
    data: T,
}

#[derive(Debug, Deserialize)]
struct NdeChatData {
    response: String,
    #[allow(dead_code)]
    conversation_id: Option<String>,
}

impl NdeAgentProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("failed to build HTTP client"),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn build_prompt(text: &str, _source_lang: &str, target_lang: &str) -> String {
        let lowered = target_lang.to_lowercase();
        let target_name = match lowered.as_str() {
            "km" | "khmer" => "Khmer (ភាសាខ្មែរ)",
            "en" | "english" => "English",
            "zh" | "chinese" => "Chinese",
            "ja" | "japanese" => "Japanese",
            "ko" | "korean" => "Korean",
            "th" | "thai" => "Thai",
            "fr" | "french" => "French",
            "de" | "german" => "German",
            "es" | "spanish" => "Spanish",
            other => other,
        };

        // `/no_think` disables chain-of-thought for Qwen3-family hybrid
        // reasoning models (harmless for other models). Without it, local
        // GGUF reasoning models burn their whole token budget inside a
        // `<think>…</think>` block and return empty content.
        format!(
            "Translate this subtitle text into {}. /no_think\n\
             Return ONLY the translated text, nothing else — \
             no quotes, no explanation, no romanization, no prefix, no reasoning.\n\n{}",
            target_name, text
        )
    }

    /// Try the direct LLM autocomplete endpoint first (bypasses agent tools).
    async fn translate_via_autocomplete(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/api/agent/autocomplete", self.base_url);

        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "prefix": prompt,
                "suffix": "",
            }))
            .send()
            .await
            .context("Failed to reach NDE autocomplete endpoint")?;

        if !resp.status().is_success() {
            anyhow::bail!("Autocomplete endpoint returned {}", resp.status());
        }

        #[derive(Deserialize)]
        struct AcData {
            completion: String,
        }

        let envelope: NdeEnvelope<AcData> = resp
            .json()
            .await
            .context("Failed to parse autocomplete response")?;

        if !envelope.success {
            anyhow::bail!("Autocomplete failed: {}", envelope.message);
        }

        let text = envelope.data.completion.trim().to_string();
        if text.is_empty() {
            anyhow::bail!("Autocomplete returned empty response");
        }
        Ok(text)
    }

    /// Fallback: use the agent chat endpoint.
    async fn translate_via_chat(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/api/agent/chat", self.base_url);

        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "message": prompt,
            }))
            .send()
            .await
            .context("Failed to contact NDE Agent for translation")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("NDE Agent HTTP {status}: {body}");
        }

        let envelope: NdeEnvelope<NdeChatData> = resp
            .json()
            .await
            .context("Failed to parse NDE Agent response")?;

        if !envelope.success {
            anyhow::bail!("NDE Agent translation failed: {}", envelope.message);
        }

        let translated = envelope.data.response.trim().to_string();
        if translated.is_empty() {
            anyhow::bail!("NDE Agent returned empty translation");
        }

        Ok(translated)
    }
}

#[async_trait]
impl TranslationProvider for NdeAgentProvider {
    fn name(&self) -> &str {
        "nde_agent"
    }

    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String> {
        let prompt = Self::build_prompt(text, source_lang, target_lang);

        debug!("NDE Agent translate: \"{}\"", &text[..text.len().min(40)]);

        // Strategy: autocomplete (direct LLM) → agent chat (fallback).
        // The autocomplete endpoint calls the LLM directly without the
        // agent tool-calling loop, so it reliably returns the translated text.
        match self.translate_via_autocomplete(&prompt).await {
            Ok(text) => {
                debug!("Translation via autocomplete succeeded");
                Ok(text)
            }
            Err(ac_err) => {
                debug!("Autocomplete failed ({ac_err}), falling back to agent chat");
                self.translate_via_chat(&prompt).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    /// Spin up a throwaway HTTP server that responds with canned JSON.
    /// Returns the `http://127.0.0.1:{port}` base URL.
    ///
    /// `router` receives `(method, path, body)` and returns a `(status, response_body)`.
    fn mock_server(
        router: impl Fn(&str, &str, &str) -> (u16, String) + Send + 'static,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{port}");

        std::thread::spawn(move || {
            // Handle up to 4 requests then exit.
            for stream in listener.incoming().take(4) {
                let mut stream = match stream {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                // Read the full raw request
                let reader_stream = stream.try_clone().unwrap();
                reader_stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .ok();
                let mut reader = BufReader::new(reader_stream);

                // Read headers until blank line
                let mut header_block = String::new();
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            if line == "\r\n" || line == "\n" {
                                break;
                            }
                            header_block.push_str(&line);
                        }
                        Err(_) => break,
                    }
                }

                // Parse request line
                let request_line = header_block.lines().next().unwrap_or("GET / HTTP/1.1");
                let parts: Vec<&str> = request_line.split_whitespace().collect();
                let method = parts.first().unwrap_or(&"GET").to_string();
                let path = parts.get(1).unwrap_or(&"/").to_string();

                // Get content-length
                let content_length: usize = header_block
                    .lines()
                    .find(|l| l.to_lowercase().starts_with("content-length:"))
                    .and_then(|l| l.split(':').nth(1))
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0);

                // Read body bytes
                let mut body_buf = vec![0u8; content_length];
                if content_length > 0 {
                    use std::io::Read;
                    let _ = reader.read_exact(&mut body_buf);
                }
                let body = String::from_utf8_lossy(&body_buf).to_string();

                let (status, resp_body) = router(&method, &path, &body);

                let response = format!(
                    "HTTP/1.1 {status} OK\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\
                     Connection: close\r\n\
                     \r\n\
                     {resp_body}",
                    resp_body.len(),
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });

        base_url
    }

    #[tokio::test]
    async fn test_translate_via_autocomplete_success() {
        let base = mock_server(|_method, path, _body| {
            if path.contains("/api/agent/autocomplete") {
                (200, r#"{"success":true,"message":"ok","data":{"completion":"សួស្តីពិភពលោក"}}"#.to_string())
            } else {
                (500, r#"{"success":false,"message":"should not reach","data":null}"#.to_string())
            }
        });

        let provider = NdeAgentProvider::new(&base);
        let result = provider.translate("Hello world", "en", "km").await;

        assert!(result.is_ok(), "Expected success, got: {:?}", result);
        let text = result.unwrap();
        assert!(!text.is_empty(), "Translation should not be empty");
        assert!(text.contains("សួស្តី"), "Should contain Khmer text, got: {text}");
    }

    #[tokio::test]
    async fn test_translate_fallback_to_chat_when_autocomplete_fails() {
        let base = mock_server(|_method, path, _body| {
            if path.contains("/api/agent/autocomplete") {
                // Autocomplete returns empty → triggers fallback
                (200, r#"{"success":true,"message":"ok","data":{"completion":""}}"#.to_string())
            } else if path.contains("/api/agent/chat") {
                // Chat returns the translation
                (200, r#"{"success":true,"message":"ok","data":{"response":"សួស្តីពិភពលោក","conversation_id":"test-123"}}"#.to_string())
            } else {
                (500, r#"{"success":false,"message":"unknown","data":null}"#.to_string())
            }
        });

        let provider = NdeAgentProvider::new(&base);
        let result = provider.translate("Hello world", "en", "km").await;

        assert!(result.is_ok(), "Fallback should succeed, got: {:?}", result);
        let text = result.unwrap();
        assert!(text.contains("សួស្តី"), "Should contain Khmer from chat fallback, got: {text}");
    }

    #[tokio::test]
    async fn test_translate_error_when_both_endpoints_return_empty() {
        let base = mock_server(|_method, path, _body| {
            if path.contains("/api/agent/autocomplete") {
                (200, r#"{"success":true,"message":"ok","data":{"completion":""}}"#.to_string())
            } else if path.contains("/api/agent/chat") {
                // Chat also returns empty — the old bug
                (200, r#"{"success":true,"message":"ok","data":{"response":"","conversation_id":"test-123"}}"#.to_string())
            } else {
                (500, r#"{"success":false,"message":"unknown","data":null}"#.to_string())
            }
        });

        let provider = NdeAgentProvider::new(&base);
        let result = provider.translate("Hello world", "en", "km").await;

        assert!(result.is_err(), "Should fail when both endpoints return empty");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("empty"),
            "Error should mention empty response, got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_translate_fallback_when_autocomplete_endpoint_unavailable() {
        let base = mock_server(|_method, path, _body| {
            if path.contains("/api/agent/autocomplete") {
                // 404 — endpoint doesn't exist
                (404, r#"{"success":false,"message":"not found","data":null}"#.to_string())
            } else if path.contains("/api/agent/chat") {
                (200, r#"{"success":true,"message":"ok","data":{"response":"Bonjour le monde","conversation_id":"test-456"}}"#.to_string())
            } else {
                (500, r#"{"success":false,"message":"unknown","data":null}"#.to_string())
            }
        });

        let provider = NdeAgentProvider::new(&base);
        let result = provider.translate("Hello world", "en", "fr").await;

        assert!(result.is_ok(), "Should fallback to chat, got: {:?}", result);
        assert_eq!(result.unwrap(), "Bonjour le monde");
    }

    #[test]
    fn test_build_prompt_khmer() {
        let prompt = NdeAgentProvider::build_prompt("Hello", "en", "km");
        assert!(prompt.contains("Khmer (ភាសាខ្មែរ)"));
        assert!(prompt.contains("Hello"));
        assert!(prompt.contains("ONLY the translated text"));
    }

    #[test]
    fn test_build_prompt_unknown_lang() {
        let prompt = NdeAgentProvider::build_prompt("Test", "en", "sw");
        assert!(prompt.contains("sw")); // Swahili → raw code used
        assert!(prompt.contains("Test"));
    }
}
