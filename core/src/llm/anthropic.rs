use super::streaming::{ChunkStream, StreamChunk};
use super::{LlmProvider, LlmResponse, Message, StopReason, ToolCall, ToolDef, Usage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

pub struct AnthropicProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    api_key: String,
    max_tokens: u32,
}

impl AnthropicProvider {
    pub fn new(base_url: &str, model: &str, api_key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key: api_key.to_string(),
            max_tokens: 4096,
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    fn build_request(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        stream: bool,
    ) -> AnthropicRequest {
        let (system, msgs) = to_anthropic_messages(messages);
        let tool_defs: Vec<AnthropicTool> = tools
            .iter()
            .map(|t| AnthropicTool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.parameters.clone(),
            })
            .collect();

        AnthropicRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system,
            messages: msgs,
            tools: if tool_defs.is_empty() {
                None
            } else {
                Some(tool_defs)
            },
            stream,
        }
    }
}

// ── Anthropic API types ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    stream: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct AnthropicMessage {
    role: String,
    content: AnthropicContent,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum AnthropicContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// ── Streaming event types ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct StreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    delta: Option<StreamDelta>,
    #[serde(default)]
    content_block: Option<ContentBlockStart>,
    #[serde(default)]
    message: Option<StreamMessage>,
}

#[derive(Deserialize)]
struct StreamDelta {
    #[serde(rename = "type")]
    #[serde(default)]
    delta_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    partial_json: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct ContentBlockStart {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct StreamMessage {
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

// ── Conversions ──────────────────────────────────────────────────────────────

fn to_anthropic_messages(messages: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {
    let mut system = None;
    let mut result = Vec::new();

    for msg in messages {
        match msg {
            Message::System { content } => {
                system = Some(content.clone());
            }
            Message::User { content } => {
                result.push(AnthropicMessage {
                    role: "user".into(),
                    content: AnthropicContent::Text(content.clone()),
                });
            }
            Message::Assistant {
                content,
                tool_calls,
            } => {
                if tool_calls.is_empty() {
                    result.push(AnthropicMessage {
                        role: "assistant".into(),
                        content: AnthropicContent::Text(content.clone().unwrap_or_default()),
                    });
                } else {
                    let mut blocks: Vec<ContentBlock> = Vec::new();
                    if let Some(text) = content {
                        if !text.is_empty() {
                            blocks.push(ContentBlock::Text { text: text.clone() });
                        }
                    }
                    for tc in tool_calls {
                        blocks.push(ContentBlock::ToolUse {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            input: tc.arguments.clone(),
                        });
                    }
                    result.push(AnthropicMessage {
                        role: "assistant".into(),
                        content: AnthropicContent::Blocks(blocks),
                    });
                }
            }
            Message::Tool {
                tool_call_id,
                content,
            } => {
                result.push(AnthropicMessage {
                    role: "user".into(),
                    content: AnthropicContent::Blocks(vec![ContentBlock::ToolResult {
                        tool_use_id: tool_call_id.clone(),
                        content: content.clone(),
                    }]),
                });
            }
        }
    }

    (system, result)
}

fn parse_response(data: AnthropicResponse) -> LlmResponse {
    let mut content_text = String::new();
    let mut tool_calls = Vec::new();

    for block in data.content {
        match block {
            ContentBlock::Text { text } => content_text.push_str(&text),
            ContentBlock::ToolUse { id, name, input } => {
                tool_calls.push(ToolCall {
                    id,
                    name,
                    arguments: input,
                });
            }
            _ => {}
        }
    }

    let stop_reason = match data.stop_reason.as_deref() {
        Some("tool_use") => StopReason::ToolUse,
        Some("max_tokens") => StopReason::MaxTokens,
        _ if !tool_calls.is_empty() => StopReason::ToolUse,
        _ => StopReason::EndTurn,
    };

    LlmResponse {
        content: if content_text.is_empty() {
            None
        } else {
            Some(content_text)
        },
        tool_calls,
        stop_reason,
        usage: data.usage.map(|u| Usage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
        }),
    }
}

// ── Provider impl ────────────────────────────────────────────────────────────

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse> {
        let body = self.build_request(messages, tools, false);

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to connect to Anthropic API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic API error {}: {}", status, text));
        }

        let data: AnthropicResponse = resp
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        Ok(parse_response(data))
    }

    async fn chat_stream(&self, messages: &[Message], tools: &[ToolDef]) -> Result<ChunkStream> {
        let body = self.build_request(messages, tools, true);

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to connect to Anthropic API for streaming")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Anthropic streaming API error {}: {}",
                status,
                text
            ));
        }

        let byte_stream = resp.bytes_stream();

        let stream = async_stream::stream! {
            let mut buffer = String::new();
            let mut current_tool_index: Option<usize> = None;
            let mut tool_count: usize = 0;

            futures::pin_mut!(byte_stream);

            while let Some(chunk_result) = byte_stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(anyhow::anyhow!("Stream read error: {}", e));
                        break;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                // Process complete SSE events (double newline separated)
                while let Some(pos) = buffer.find("\n\n") {
                    let event_text = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    // Parse SSE event
                    let mut event_type = String::new();
                    let mut event_data = String::new();

                    for line in event_text.lines() {
                        if let Some(et) = line.strip_prefix("event: ") {
                            event_type = et.to_string();
                        } else if let Some(d) = line.strip_prefix("data: ") {
                            event_data = d.to_string();
                        }
                    }

                    if event_data.is_empty() {
                        continue;
                    }

                    let event: StreamEvent = match serde_json::from_str(&event_data) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    match event_type.as_str() {
                        "content_block_start" => {
                            if let Some(block) = &event.content_block {
                                if block.block_type == "tool_use" {
                                    current_tool_index = Some(tool_count);
                                    yield Ok(StreamChunk::ToolCallDelta {
                                        index: tool_count,
                                        id: block.id.clone(),
                                        name: block.name.clone(),
                                        arguments_delta: String::new(),
                                    });
                                    tool_count += 1;
                                }
                            }
                        }
                        "content_block_delta" => {
                            if let Some(delta) = &event.delta {
                                if let Some(text) = &delta.text {
                                    yield Ok(StreamChunk::TextDelta {
                                        content: text.clone(),
                                    });
                                }
                                if let Some(json) = &delta.partial_json {
                                    if let Some(idx) = current_tool_index {
                                        yield Ok(StreamChunk::ToolCallDelta {
                                            index: idx,
                                            id: None,
                                            name: None,
                                            arguments_delta: json.clone(),
                                        });
                                    }
                                }
                            }
                        }
                        "content_block_stop" => {
                            current_tool_index = None;
                        }
                        "message_delta" => {
                            // Final message stats
                        }
                        "message_stop" => {
                            let usage = event
                                .message
                                .and_then(|m| m.usage)
                                .map(|u| Usage {
                                    prompt_tokens: u.input_tokens,
                                    completion_tokens: u.output_tokens,
                                });
                            yield Ok(StreamChunk::Done {
                                content: None,
                                usage,
                            });
                        }
                        _ => {}
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn name(&self) -> &str {
        "anthropic"
    }
}
