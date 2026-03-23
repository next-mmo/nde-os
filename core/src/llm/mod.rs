pub mod anthropic;
pub mod codex_oauth;
pub mod gguf;
pub mod manager;
pub mod ollama;
pub mod openai_compat;
pub mod streaming;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use self::streaming::ChunkStream;

// ── Message types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: Option<String>, #[serde(default)] tool_calls: Vec<ToolCall> },
    Tool { tool_call_id: String, content: String },
}

impl Message {
    pub fn system(content: &str) -> Self {
        Self::System { content: content.to_string() }
    }
    pub fn user(content: &str) -> Self {
        Self::User { content: content.to_string() }
    }
    pub fn assistant_text(content: &str) -> Self {
        Self::Assistant { content: Some(content.to_string()), tool_calls: vec![] }
    }
    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self::Assistant { content: None, tool_calls }
    }
    pub fn tool_result(id: &str, content: &str) -> Self {
        Self::Tool { tool_call_id: id.to_string(), content: content.to_string() }
    }
}

// ── Tool call types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

// ── LLM Response ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub stop_reason: StopReason,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

// ── Provider trait ───────────────────────────────────────────────────────────

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Non-streaming chat completion.
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
    ) -> Result<LlmResponse>;

    /// Streaming chat completion — returns a stream of chunks.
    /// Default implementation falls back to non-streaming.
    async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
    ) -> Result<ChunkStream> {
        let resp = self.chat(messages, tools).await?;
        let stream = async_stream::stream! {
            if let Some(content) = &resp.content {
                yield Ok(streaming::StreamChunk::TextDelta {
                    content: content.clone(),
                });
            }
            yield Ok(streaming::StreamChunk::Done {
                content: resp.content.clone(),
                usage: resp.usage.clone(),
            });
        };
        Ok(Box::pin(stream))
    }

    fn name(&self) -> &str;
}

// ── Provider factory ─────────────────────────────────────────────────────────

pub fn create_provider(
    provider: &str,
    model: &str,
    base_url: Option<&str>,
    api_key: Option<&str>,
) -> Result<Box<dyn LlmProvider>> {
    match provider {
        "gguf" | "llama-cpp" | "llama.cpp" => {
            // GGUF provider: local llama.cpp server with auto-download.
            // base_url can point to an explicit .gguf file path, or None for default TinyLlama.
            let data_dir = std::env::var("AI_LAUNCHER_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    if cfg!(windows) {
                        std::env::var("LOCALAPPDATA")
                            .map(|p| std::path::PathBuf::from(p).join("ai-launcher"))
                            .unwrap_or_else(|_| std::path::PathBuf::from("C:\\ai-launcher-data"))
                    } else {
                        std::env::var("HOME")
                            .map(std::path::PathBuf::from)
                            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
                            .join(".ai-launcher")
                    }
                });
            Ok(Box::new(gguf::GgufProvider::new(
                &data_dir,
                model,
                base_url, // Reuse base_url as optional model path
                None,     // Default port
            )))
        }
        "ollama" => {
            let url = base_url.unwrap_or("http://localhost:11434");
            Ok(Box::new(ollama::OllamaProvider::new(url, model)))
        }
        "openai" => {
            let url = base_url.unwrap_or("https://api.openai.com/v1");
            let key = api_key.ok_or_else(|| anyhow::anyhow!("API key required for OpenAI provider"))?;
            Ok(Box::new(openai_compat::OpenAiCompatProvider::new(url, model, key)))
        }
        "anthropic" | "claude" => {
            let url = base_url.unwrap_or("https://api.anthropic.com");
            let key = api_key
                .map(|k| k.to_string())
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                .ok_or_else(|| anyhow::anyhow!("API key required for Anthropic (set ANTHROPIC_API_KEY or api_key)"))?;
            Ok(Box::new(anthropic::AnthropicProvider::new(url, model, &key)))
        }
        "codex" => {
            let url = base_url.unwrap_or("https://api.openai.com/v1");
            let key = api_key
                .map(|k| k.to_string())
                .or_else(|| std::env::var("CODEX_API_KEY").ok())
                .ok_or_else(|| anyhow::anyhow!("API key required for Codex (set CODEX_API_KEY or api_key)"))?;
            Ok(Box::new(openai_compat::OpenAiCompatProvider::new(url, model, &key)))
        }
        "groq" => {
            let url = base_url.unwrap_or("https://api.groq.com/openai/v1");
            let key = api_key
                .map(|k| k.to_string())
                .or_else(|| std::env::var("GROQ_API_KEY").ok())
                .ok_or_else(|| anyhow::anyhow!("API key required for Groq (set GROQ_API_KEY or api_key)"))?;
            Ok(Box::new(openai_compat::OpenAiCompatProvider::new(url, model, &key)))
        }
        "together" => {
            let url = base_url.unwrap_or("https://api.together.xyz/v1");
            let key = api_key
                .map(|k| k.to_string())
                .or_else(|| std::env::var("TOGETHER_API_KEY").ok())
                .ok_or_else(|| anyhow::anyhow!("API key required for Together (set TOGETHER_API_KEY or api_key)"))?;
            Ok(Box::new(openai_compat::OpenAiCompatProvider::new(url, model, &key)))
        }
        "openai_compat" | "openai-compat" | "custom" => {
            let url = base_url.ok_or_else(|| anyhow::anyhow!("base_url required for openai_compat provider"))?;
            let key = api_key.ok_or_else(|| anyhow::anyhow!("api_key required for openai_compat provider"))?;
            Ok(Box::new(openai_compat::OpenAiCompatProvider::new(url, model, key)))
        }
        "codex_oauth" | "codex-oauth" => {
            let data_dir = std::env::var("AI_LAUNCHER_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    if cfg!(windows) {
                        std::env::var("LOCALAPPDATA")
                            .map(|p| std::path::PathBuf::from(p).join("ai-launcher"))
                            .unwrap_or_else(|_| std::path::PathBuf::from("C:\\ai-launcher-data"))
                    } else {
                        std::env::var("HOME")
                            .map(std::path::PathBuf::from)
                            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
                            .join(".ai-launcher")
                    }
                });
            Ok(Box::new(codex_oauth::CodexOAuthProvider::new(model, &data_dir)?))
        }
        other => Err(anyhow::anyhow!(
            "Unknown LLM provider: '{}'. Supported: gguf, ollama, openai, anthropic, codex, codex_oauth, groq, together, openai_compat",
            other
        )),
    }
}

/// Verify that a provider config is usable before adding it.
/// Returns Ok(()) if the provider can run, or an error describing why not.
pub async fn verify_provider_config(config: &manager::ProviderConfig) -> anyhow::Result<()> {
    match config.provider_type.as_str() {
        // GGUF (local): verify model file + server binary
        "gguf" | "llama-cpp" | "llama.cpp" => {
            let data_dir = std::env::var("AI_LAUNCHER_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    if cfg!(windows) {
                        std::env::var("LOCALAPPDATA")
                            .map(|p| std::path::PathBuf::from(p).join("ai-launcher"))
                            .unwrap_or_else(|_| std::path::PathBuf::from("C:\\ai-launcher-data"))
                    } else {
                        std::env::var("HOME")
                            .map(std::path::PathBuf::from)
                            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
                            .join(".ai-launcher")
                    }
                });
            let provider = gguf::GgufProvider::new(
                &data_dir,
                &config.model,
                config.base_url.as_deref(),
                None,
            );
            let result = provider.verify().await;
            if result.ok {
                tracing::info!("GGUF verify OK: model_exists={}, server={}", result.model_exists, result.server_available);
                Ok(())
            } else {
                Err(anyhow::anyhow!("{}", result.error.unwrap_or_else(|| "GGUF verification failed".into())))
            }
        }

        // Ollama: verify server reachable + model available
        "ollama" => {
            let url = config.base_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
            let client = reqwest::Client::new();
            match tokio::time::timeout(
                std::time::Duration::from_secs(3),
                client.get(format!("{}/api/tags", url)).send()
            ).await {
                Ok(Ok(resp)) if resp.status().is_success() => {
                    if let Ok(body) = resp.text().await {
                        let model_name = &config.model;
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                            let models = json["models"].as_array();
                            let found = models.map(|arr| arr.iter().any(|m| {
                                let n = m["name"].as_str().unwrap_or_default();
                                n == model_name || n.starts_with(&format!("{}:", model_name))
                            })).unwrap_or(false);
                            if !found {
                                let available: Vec<String> = models.map(|arr| {
                                    arr.iter().filter_map(|m| m["name"].as_str().map(|s| s.to_string())).collect()
                                }).unwrap_or_default();
                                return Err(anyhow::anyhow!(
                                    "Ollama is running but model '{}' is not pulled. Available: {}. Run: ollama pull {}",
                                    model_name,
                                    if available.is_empty() { "(none)".into() } else { available.join(", ") },
                                    model_name
                                ));
                            }
                        }
                    }
                    Ok(())
                }
                Ok(Ok(resp)) => Err(anyhow::anyhow!("Ollama server returned HTTP {}", resp.status())),
                Ok(Err(e)) => Err(anyhow::anyhow!("Cannot connect to Ollama at {}: {}", url, e)),
                Err(_) => Err(anyhow::anyhow!("Ollama connection timed out at {} — is Ollama running?", url)),
            }
        }

        // All other API providers: test with a ping
        _ => {
            let api_key = config.api_key.clone().or_else(|| {
                config.api_key_env.as_ref().and_then(|env| std::env::var(env).ok())
            });

            let provider = create_provider(
                &config.provider_type,
                &config.model,
                config.base_url.as_deref(),
                api_key.as_deref(),
            ).map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;

            let msg = vec![Message::user("ping")];
            let tools = vec![];
            match tokio::time::timeout(std::time::Duration::from_secs(5), provider.chat(&msg, &tools)).await {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(e)) => {
                    let err_str = e.to_string().to_lowercase();
                    if err_str.contains("404") || err_str.contains("not found") {
                        tracing::warn!("Provider ping: potential model issue (letting through): {}", e);
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!("API Error: {}", e))
                    }
                }
                Err(_) => Err(anyhow::anyhow!("Connection timed out (5s)")),
            }
        }
    }
}
