/// Real SSE streaming handler for agent tasks.
///
/// Replaces the fake word-split streaming with actual token-level SSE
/// powered by the AgentManager and executor.
use ai_launcher_core::agent::manager::AgentManager;
use ai_launcher_core::agent::models::TaskFilter;
use ai_launcher_core::agent::protocol::AgentEvent;
use serde::Deserialize;
use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Request, Response};

use crate::response::*;

#[derive(Deserialize)]
pub struct StreamChatRequest {
    pub message: String,
    pub conversation_id: Option<String>,
}

#[derive(Deserialize)]
pub struct SpawnTaskRequest {
    pub message: String,
}

/// POST /api/agent/tasks — spawn a new task, return task_id.
pub fn spawn_task(
    req: &mut Request,
    rt: &tokio::runtime::Runtime,
    manager: &Arc<tokio::sync::Mutex<AgentManager>>,
) -> Response<Cursor<Vec<u8>>> {
    let body = match read_body(req) {
        Some(b) => b,
        None => return err(400, "Missing request body"),
    };

    let spawn_req: SpawnTaskRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
    };

    let manager = manager.clone();
    let result = rt.block_on(async {
        let mgr = manager.lock().await;
        mgr.spawn(&spawn_req.message).await
    });

    match result {
        Ok(task_id) => json_resp(
            201,
            &serde_json::json!({
                "success": true,
                "message": "Task spawned",
                "data": { "task_id": task_id }
            }),
        ),
        Err(e) => err(500, &format!("Failed to spawn task: {}", e)),
    }
}

/// GET /api/agent/tasks/{id}/stream — real SSE stream for a task.
pub fn stream_task(
    task_id: &str,
    rt: &tokio::runtime::Runtime,
    manager: &Arc<tokio::sync::Mutex<AgentManager>>,
) -> Response<Cursor<Vec<u8>>> {
    let manager = manager.clone();
    let task_id = task_id.to_string();

    let result = rt.block_on(async {
        let mgr = manager.lock().await;
        let mut rx = mgr.subscribe();
        drop(mgr); // Release lock so events can flow

        let mut sse_data = Vec::new();
        let timeout = tokio::time::Duration::from_secs(300); // 5 min max

        loop {
            match tokio::time::timeout(timeout, rx.recv()).await {
                Ok(Ok(event)) => {
                    // Only forward events for this task
                    if event.task_id() != task_id {
                        continue;
                    }

                    let is_terminal = event.is_terminal();
                    write!(sse_data, "{}", event.to_sse()).ok();

                    if is_terminal {
                        write!(sse_data, "data: [DONE]\n\n").ok();
                        break;
                    }
                }
                Ok(Err(_)) => {
                    // Channel lagged or closed
                    write!(
                        sse_data,
                        "data: {}\n\ndata: [DONE]\n\n",
                        serde_json::json!({"type": "error", "message": "Event stream closed"})
                    )
                    .ok();
                    break;
                }
                Err(_) => {
                    // Timeout
                    write!(
                        sse_data,
                        "data: {}\n\ndata: [DONE]\n\n",
                        serde_json::json!({"type": "error", "message": "Stream timeout"})
                    )
                    .ok();
                    break;
                }
            }
        }

        sse_data
    });

    Response::from_data(result)
        .with_header(Header::from_bytes("Content-Type", "text/event-stream").unwrap())
        .with_header(Header::from_bytes("Cache-Control", "no-cache").unwrap())
        .with_header(Header::from_bytes("Connection", "keep-alive").unwrap())
        .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
}

/// POST /api/agent/tasks/{id}/cancel — cancel a running task.
pub fn cancel_task(
    task_id: &str,
    rt: &tokio::runtime::Runtime,
    manager: &Arc<tokio::sync::Mutex<AgentManager>>,
) -> Response<Cursor<Vec<u8>>> {
    let manager = manager.clone();
    let task_id = task_id.to_string();

    let result = rt.block_on(async {
        let mgr = manager.lock().await;
        mgr.cancel(&task_id).await
    });

    match result {
        Ok(()) => json_resp(
            200,
            &serde_json::json!({
                "success": true,
                "message": "Task cancelled"
            }),
        ),
        Err(e) => err(500, &format!("Failed to cancel: {}", e)),
    }
}

/// GET /api/agent/tasks — list all tasks.
pub fn list_tasks(
    manager: &Arc<tokio::sync::Mutex<AgentManager>>,
    rt: &tokio::runtime::Runtime,
) -> Response<Cursor<Vec<u8>>> {
    let manager = manager.clone();
    let result = rt.block_on(async {
        let mgr = manager.lock().await;
        mgr.list_tasks(&TaskFilter {
            limit: Some(50),
            ..Default::default()
        })
    });

    match result {
        Ok(tasks) => ok(&format!("{} task(s)", tasks.len()), tasks),
        Err(e) => err(500, &format!("Failed to list tasks: {}", e)),
    }
}

/// GET /api/agent/tasks/{id} — get task status.
pub fn get_task(
    task_id: &str,
    manager: &Arc<tokio::sync::Mutex<AgentManager>>,
    rt: &tokio::runtime::Runtime,
) -> Response<Cursor<Vec<u8>>> {
    let manager = manager.clone();
    let result = rt.block_on(async {
        let mgr = manager.lock().await;
        mgr.get_task(task_id)
    });

    match result {
        Ok(Some(task)) => ok("Task details", task),
        Ok(None) => err(404, &format!("Task {} not found", task_id)),
        Err(e) => err(500, &format!("Failed to get task: {}", e)),
    }
}

/// POST /api/agent/chat/stream — backward-compatible streaming chat.
/// Spawns a task, collects all events, returns as SSE.
pub fn handle_stream_chat(
    req: &mut Request,
    rt: &tokio::runtime::Runtime,
    agent_state: &Mutex<crate::agent_handler::AgentState>,
    llm_manager: &Arc<Mutex<ai_launcher_core::llm::manager::LlmManager>>,
    manager: Option<&Arc<tokio::sync::Mutex<AgentManager>>>,
) -> Response<Cursor<Vec<u8>>> {
    // If AgentManager is available, use the new task-based streaming
    if let Some(manager) = manager {
        let body = match read_body(req) {
            Some(b) => b,
            None => return err(400, "Missing request body"),
        };

        let chat_req: StreamChatRequest = match serde_json::from_str(&body) {
            Ok(r) => r,
            Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
        };

        // Save to conversation store
        let conv_id = {
            let state = agent_state.lock().unwrap();
            let conv_id = match chat_req.conversation_id {
                Some(id) => id,
                None => state
                    .memory
                    .conversations
                    .create_conversation(
                        &chat_req.message.chars().take(50).collect::<String>(),
                        "nde-chat",
                    )
                    .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string()),
            };
            let _ = state.memory.conversations.save_message(
                &conv_id,
                "user",
                Some(&chat_req.message),
                None,
                None,
            );
            conv_id
        };

        let manager = manager.clone();
        let message = chat_req.message.clone();

        let result = rt.block_on(async {
            let mgr = manager.lock().await;
            let mut rx = mgr.subscribe();
            let task_id = mgr.spawn(&message).await?;
            drop(mgr);

            let mut sse_data = Vec::new();
            let mut final_output = String::new();
            let timeout = tokio::time::Duration::from_secs(300);

            loop {
                match tokio::time::timeout(timeout, rx.recv()).await {
                    Ok(Ok(event)) => {
                        if event.task_id() != task_id {
                            continue;
                        }

                        let is_terminal = event.is_terminal();

                        // Capture final output for conversation
                        if let AgentEvent::TaskCompleted { ref output, .. } = event {
                            final_output = output.clone();
                        }

                        write!(sse_data, "{}", event.to_sse()).ok();

                        if is_terminal {
                            // Add conversation_id to done event
                            let done = serde_json::json!({
                                "type": "done",
                                "content": final_output,
                                "conversation_id": conv_id,
                            });
                            write!(
                                sse_data,
                                "data: {}\n\ndata: [DONE]\n\n",
                                serde_json::to_string(&done).unwrap_or_default()
                            )
                            .ok();
                            break;
                        }
                    }
                    Ok(Err(_)) => break,
                    Err(_) => break,
                }
            }

            // Save assistant response
            if !final_output.is_empty() {
                let state = agent_state.lock().unwrap();
                let _ = state.memory.conversations.save_message(
                    &conv_id,
                    "assistant",
                    Some(&final_output),
                    None,
                    None,
                );
            }

            Ok::<Vec<u8>, anyhow::Error>(sse_data)
        });

        return match result {
            Ok(sse_data) => Response::from_data(sse_data)
                .with_header(Header::from_bytes("Content-Type", "text/event-stream").unwrap())
                .with_header(Header::from_bytes("Cache-Control", "no-cache").unwrap())
                .with_header(Header::from_bytes("Connection", "keep-alive").unwrap())
                .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap()),
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
        };
    }

    // Fallback: old behavior (no AgentManager)
    fallback_stream_chat(req, rt, agent_state, llm_manager)
}

/// Fallback for when AgentManager is not available (legacy compat).
fn fallback_stream_chat(
    req: &mut Request,
    runtime: &tokio::runtime::Runtime,
    agent_state: &Mutex<crate::agent_handler::AgentState>,
    llm_manager: &Arc<Mutex<ai_launcher_core::llm::manager::LlmManager>>,
) -> Response<Cursor<Vec<u8>>> {
    let body = match read_body(req) {
        Some(b) => b,
        None => return err(400, "Missing request body"),
    };

    let chat_req: StreamChatRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
    };

    let state = agent_state.lock().unwrap();
    let mut config = state.config.clone();
    crate::agent_handler::sync_model_config(&mut config, llm_manager);

    let message = chat_req.message.clone();

    let conv_id = match chat_req.conversation_id {
        Some(id) => id,
        None => state
            .memory
            .conversations
            .create_conversation(
                &chat_req.message.chars().take(50).collect::<String>(),
                "nde-chat",
            )
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string()),
    };

    let _ = state.memory.conversations.save_message(
        &conv_id,
        "user",
        Some(&chat_req.message),
        None,
        None,
    );

    let result: Result<String, anyhow::Error> = runtime.block_on(async {
        match ai_launcher_core::agent::AgentRuntime::from_config(config) {
            Ok(mut agent) => agent.run(&message).await,
            Err(e) => Err(anyhow::anyhow!("Init error: {}", e)),
        }
    });

    match result {
        Ok(response_text) => {
            state
                .memory
                .conversations
                .save_message(&conv_id, "assistant", Some(&response_text), None, None)
                .ok();

            let mut sse_data = Vec::new();
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

            let done = serde_json::json!({
                "type": "done",
                "content": response_text,
                "conversation_id": conv_id,
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
                "conversation_id": conv_id,
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
