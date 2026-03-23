use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};

use super::{LlmResponse, Usage};

// ── Stream chunk types ───────────────────────────────────────────────────────

/// A single chunk from a streaming LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamChunk {
    /// A text delta from the assistant.
    TextDelta { content: String },

    /// A tool call being built incrementally.
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments_delta: String,
    },

    /// Stream is complete — carries the final aggregated response.
    Done {
        content: Option<String>,
        usage: Option<Usage>,
    },

    /// An error occurred mid-stream.
    Error { message: String },
}

/// Pinned stream of chunks.
pub type ChunkStream = Pin<Box<dyn Stream<Item = anyhow::Result<StreamChunk>> + Send>>;

// ── Model info ───────────────────────────────────────────────────────────────

/// Information about a model available on a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_window: usize,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub provider: String,
}

// ── SSE parsing helpers ──────────────────────────────────────────────────────

/// Parse a single SSE line from an OpenAI-compatible streaming response.
/// Returns `None` for keep-alive or `[DONE]` lines.
pub fn parse_sse_line(line: &str) -> Option<serde_json::Value> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(':') {
        return None;
    }
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return None;
    }
    serde_json::from_str(data).ok()
}

/// Accumulator for building a complete `LlmResponse` from stream chunks.
#[derive(Default)]
pub struct StreamAccumulator {
    pub content: String,
    pub tool_calls: Vec<ToolCallAccum>,
    pub usage: Option<Usage>,
}

#[derive(Default, Clone)]
pub struct ToolCallAccum {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl StreamAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, chunk: &StreamChunk) {
        match chunk {
            StreamChunk::TextDelta { content } => {
                self.content.push_str(content);
            }
            StreamChunk::ToolCallDelta {
                index,
                id,
                name,
                arguments_delta,
            } => {
                while self.tool_calls.len() <= *index {
                    self.tool_calls.push(ToolCallAccum::default());
                }
                let tc = &mut self.tool_calls[*index];
                if let Some(id) = id {
                    tc.id.clone_from(id);
                }
                if let Some(name) = name {
                    tc.name.clone_from(name);
                }
                tc.arguments.push_str(arguments_delta);
            }
            StreamChunk::Done { usage, .. } => {
                self.usage = usage.clone();
            }
            StreamChunk::Error { .. } => {}
        }
    }

    pub fn into_response(self) -> LlmResponse {
        let tool_calls: Vec<super::ToolCall> = self
            .tool_calls
            .into_iter()
            .filter(|tc| !tc.name.is_empty())
            .map(|tc| super::ToolCall {
                id: tc.id,
                name: tc.name,
                arguments: serde_json::from_str(&tc.arguments)
                    .unwrap_or(serde_json::Value::Object(Default::default())),
            })
            .collect();

        let stop_reason = if !tool_calls.is_empty() {
            super::StopReason::ToolUse
        } else {
            super::StopReason::EndTurn
        };

        LlmResponse {
            content: if self.content.is_empty() {
                None
            } else {
                Some(self.content)
            },
            tool_calls,
            stop_reason,
            usage: self.usage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_line() {
        assert!(parse_sse_line("").is_none());
        assert!(parse_sse_line(": keep-alive").is_none());
        assert!(parse_sse_line("data: [DONE]").is_none());

        let val = parse_sse_line(r#"data: {"choices":[{"delta":{"content":"Hi"}}]}"#);
        assert!(val.is_some());
    }

    #[test]
    fn test_accumulator() {
        let mut acc = StreamAccumulator::new();
        acc.push(&StreamChunk::TextDelta {
            content: "Hello ".into(),
        });
        acc.push(&StreamChunk::TextDelta {
            content: "world!".into(),
        });
        acc.push(&StreamChunk::Done {
            content: None,
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
            }),
        });

        let resp = acc.into_response();
        assert_eq!(resp.content.as_deref(), Some("Hello world!"));
        assert!(resp.tool_calls.is_empty());
    }
}
