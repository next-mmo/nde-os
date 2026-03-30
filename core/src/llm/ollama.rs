use super::{LlmProvider, LlmResponse, Message, StopReason, ToolCall, ToolDef, Usage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }
}

// ── Ollama API types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OllamaTool>,
}

#[derive(Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OllamaFunction,
}

#[derive(Serialize, Deserialize)]
struct OllamaFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
struct OllamaToolCall {
    id: Option<String>,
    function: OllamaFunctionCall,
}

#[derive(Serialize, Deserialize, Clone)]
struct OllamaFunctionCall {
    name: String,
    arguments: serde_json::Value,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

// ── Conversions ──────────────────────────────────────────────────────────────

fn to_ollama_messages(messages: &[Message]) -> Vec<OllamaMessage> {
    messages
        .iter()
        .map(|m| match m {
            Message::System { content } => OllamaMessage {
                role: "system".into(),
                content: content.clone(),
                tool_calls: None,
                tool_call_id: None,
            },
            Message::User { content } => OllamaMessage {
                role: "user".into(),
                content: content.clone(),
                tool_calls: None,
                tool_call_id: None,
            },
            Message::Assistant {
                content,
                tool_calls,
            } => {
                let tc = if tool_calls.is_empty() {
                    None
                } else {
                    Some(
                        tool_calls
                            .iter()
                            .map(|tc| OllamaToolCall {
                                id: Some(tc.id.clone()),
                                function: OllamaFunctionCall {
                                    name: tc.name.clone(),
                                    arguments: tc.arguments.clone(),
                                },
                            })
                            .collect(),
                    )
                };
                OllamaMessage {
                    role: "assistant".into(),
                    content: content.clone().unwrap_or_default(),
                    tool_calls: tc,
                    tool_call_id: None,
                }
            }
            Message::Tool {
                tool_call_id,
                content,
            } => OllamaMessage {
                role: "tool".into(),
                content: content.clone(),
                tool_calls: None,
                tool_call_id: Some(tool_call_id.clone()),
            },
        })
        .collect()
}

fn to_ollama_tools(tools: &[ToolDef]) -> Vec<OllamaTool> {
    tools
        .iter()
        .map(|t| OllamaTool {
            tool_type: "function".into(),
            function: OllamaFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            },
        })
        .collect()
}

// ── Provider impl ────────────────────────────────────────────────────────────

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse> {
        let body = OllamaRequest {
            model: self.model.clone(),
            messages: to_ollama_messages(messages),
            stream: false,
            tools: to_ollama_tools(tools),
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .context("Failed to connect to Ollama")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama API error {}: {}", status, text));
        }

        let data: OllamaResponse = resp
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        let tool_calls: Vec<ToolCall> = data
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(i, tc)| ToolCall {
                id: tc.id.unwrap_or_else(|| format!("call_{}", i)),
                name: tc.function.name,
                arguments: tc.function.arguments,
            })
            .collect();

        let stop_reason = if !tool_calls.is_empty() {
            StopReason::ToolUse
        } else {
            StopReason::EndTurn
        };

        let content = if data.message.content.is_empty() {
            None
        } else {
            Some(data.message.content)
        };

        Ok(LlmResponse {
            content,
            tool_calls,
            stop_reason,
            usage: Some(Usage {
                prompt_tokens: data.prompt_eval_count.unwrap_or(0),
                completion_tokens: data.eval_count.unwrap_or(0),
            }),
        })
    }

    fn name(&self) -> &str {
        "ollama"
    }
}
