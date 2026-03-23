/// Streaming SSE handler for agent chat.
/// POST /api/agent/chat/stream → text/event-stream
use ai_launcher_core::agent::AgentRuntime;
use serde::Deserialize;
use std::io::{Cursor, Write};
use tiny_http::{Header, Request, Response};

use crate::response::*;

#[derive(Deserialize)]
pub struct StreamChatRequest {
    pub message: String,
    pub conversation_id: Option<String>,
    pub provider: Option<String>,
}

/// Handle a streaming chat request — returns SSE stream.
/// This runs the agent, streams LLM tokens, and sends tool call results.
pub fn handle_stream_chat(
    req: &mut Request,
    runtime: &tokio::runtime::Runtime,
    agent_state: &crate::AgentState,
    llm_manager: &std::sync::Arc<std::sync::Mutex<ai_launcher_core::llm::manager::LlmManager>>,
) -> Response<Cursor<Vec<u8>>> {
    let body = match read_body(req) {
        Some(b) => b,
        None => return err(400, "Missing request body"),
    };

    let chat_req: StreamChatRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
    };

    let mut config = agent_state.config.clone();
    crate::agent_handler::sync_model_config(&mut config, llm_manager);

    let message = chat_req.message.clone();

    let result: Result<String, anyhow::Error> = runtime.block_on(async {
        match AgentRuntime::from_config(config) {
            Ok(mut agent) => agent.run(&message).await,
            Err(e) => Err(anyhow::anyhow!("Init error: {}", e)),
        }
    });

    match result {
        Ok(response_text) => {
            // Build SSE response
            let mut sse_data = Vec::new();

            // Split into word-level chunks for a streaming feel
            for word in response_text.split_inclusive(' ') {
                let chunk = serde_json::json!({
                    "type": "text_delta",
                    "content": word,
                });
                write!(
                    sse_data,
                    "data: {}\n\n",
                    serde_json::to_string(&chunk).unwrap_or_default()
                )
                .ok();
            }

            // Send done event
            let done = serde_json::json!({
                "type": "done",
                "content": response_text,
            });
            write!(
                sse_data,
                "data: {}\n\ndata: [DONE]\n\n",
                serde_json::to_string(&done).unwrap_or_default()
            )
            .ok();

            Response::from_data(sse_data)
                .with_header(Header::from_bytes("Content-Type", "text/event-stream").unwrap())
                .with_header(Header::from_bytes("Cache-Control", "no-cache").unwrap())
                .with_header(Header::from_bytes("Connection", "keep-alive").unwrap())
                .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
        }
        Err(e) => {
            let mut sse_data = Vec::new();
            let error_event = serde_json::json!({
                "type": "error",
                "message": e.to_string(),
            });
            write!(
                sse_data,
                "data: {}\n\ndata: [DONE]\n\n",
                serde_json::to_string(&error_event).unwrap_or_default()
            )
            .ok();

            Response::from_data(sse_data)
                .with_header(Header::from_bytes("Content-Type", "text/event-stream").unwrap())
                .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
        }
    }
}
