use super::{LlmProvider, LlmResponse, Message, StopReason, ToolCall, ToolDef, Usage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct OpenAiCompatProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    api_key: String,
}

impl OpenAiCompatProvider {
    pub fn new(base_url: &str, model: &str, api_key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub fn from_env(model: &str) -> Result<Self> {
        let key = std::env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY not set")?;
        Ok(Self::new("https://api.openai.com/v1", model, &key))
    }
}

// ── OpenAI API types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OaiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OaiTool>,
}

#[derive(Serialize, Deserialize)]
struct OaiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OaiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OaiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OaiFunction,
}

#[derive(Serialize, Deserialize)]
struct OaiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
struct OaiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OaiFunctionCall,
}

#[derive(Serialize, Deserialize, Clone)]
struct OaiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<OaiUsage>,
}

#[derive(Deserialize)]
struct Choice {
    message: OaiMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OaiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

// ── Conversions ──────────────────────────────────────────────────────────────

fn to_oai_messages(messages: &[Message]) -> Vec<OaiMessage> {
    messages.iter().map(|m| match m {
        Message::System { content } => OaiMessage {
            role: "system".into(), content: Some(content.clone()),
            tool_calls: None, tool_call_id: None,
        },
        Message::User { content } => OaiMessage {
            role: "user".into(), content: Some(content.clone()),
            tool_calls: None, tool_call_id: None,
        },
        Message::Assistant { content, tool_calls } => {
            let tc = if tool_calls.is_empty() { None } else {
                Some(tool_calls.iter().map(|tc| OaiToolCall {
                    id: tc.id.clone(),
                    call_type: "function".into(),
                    function: OaiFunctionCall {
                        name: tc.name.clone(),
                        arguments: tc.arguments.to_string(),
                    },
                }).collect())
            };
            OaiMessage {
                role: "assistant".into(), content: content.clone(),
                tool_calls: tc, tool_call_id: None,
            }
        }
        Message::Tool { tool_call_id, content } => OaiMessage {
            role: "tool".into(), content: Some(content.clone()),
            tool_calls: None, tool_call_id: Some(tool_call_id.clone()),
        },
    }).collect()
}

fn to_oai_tools(tools: &[ToolDef]) -> Vec<OaiTool> {
    tools.iter().map(|t| OaiTool {
        tool_type: "function".into(),
        function: OaiFunction {
            name: t.name.clone(),
            description: t.description.clone(),
            parameters: t.parameters.clone(),
        },
    }).collect()
}

// ── Provider impl ────────────────────────────────────────────────────────────

#[async_trait]
impl LlmProvider for OpenAiCompatProvider {
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse> {
        let body = ChatRequest {
            model: self.model.clone(),
            messages: to_oai_messages(messages),
            tools: to_oai_tools(tools),
        };

        let resp = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to connect to OpenAI-compatible API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI API error {}: {}", status, text));
        }

        let data: ChatResponse = resp.json().await
            .context("Failed to parse OpenAI response")?;

        let choice = data.choices.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("No choices in OpenAI response"))?;

        let tool_calls: Vec<ToolCall> = choice.message.tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| {
                let args = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                ToolCall {
                    id: tc.id,
                    name: tc.function.name,
                    arguments: args,
                }
            })
            .collect();

        let stop_reason = match choice.finish_reason.as_deref() {
            Some("tool_calls") => StopReason::ToolUse,
            Some("length") => StopReason::MaxTokens,
            _ if !tool_calls.is_empty() => StopReason::ToolUse,
            _ => StopReason::EndTurn,
        };

        Ok(LlmResponse {
            content: choice.message.content,
            tool_calls,
            stop_reason,
            usage: data.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
            }),
        })
    }

    fn name(&self) -> &str { "openai_compat" }
}
