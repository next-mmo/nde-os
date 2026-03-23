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
