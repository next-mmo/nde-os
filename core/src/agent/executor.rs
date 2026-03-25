//! Real streaming agent executor.
//!
//! Replaces the old sync `AgentRuntime::run()` with a proper streaming loop that:
//! - Uses `provider.chat_stream()` for token-level SSE
//! - Emits typed `AgentEvent` via `tokio::sync::mpsc`
//! - Integrates Guardian (injection scan, audit trail, compute metering)
//! - Supports CancellationToken for clean shutdown
//! - Saves checkpoints for crash recovery
//! - Retries transient LLM errors with exponential backoff

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::llm::streaming::{StreamAccumulator, StreamChunk};
use crate::llm::{LlmProvider, Message, ToolDef};
use crate::sandbox::Sandbox;
use crate::tools::ToolRegistry;

use super::config::AgentConfig;
use super::guardian::{Guardian, GuardianConfig};
use super::models::AgentTask;
use super::protocol::AgentEvent;
use super::store::TaskStore;

/// Configuration for the executor.
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// How many iterations between checkpoint saves.
    pub checkpoint_interval: u32,
    /// Max retries on transient LLM errors.
    pub max_retries: u32,
    /// Base delay for exponential backoff (ms).
    pub backoff_base_ms: u64,
    /// Guardian security config.
    pub guardian: GuardianConfig,
    /// Audit trail directory.
    pub audit_dir: std::path::PathBuf,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            checkpoint_interval: 3,
            max_retries: 3,
            backoff_base_ms: 500,
            guardian: GuardianConfig::default(),
            audit_dir: std::path::PathBuf::from("audit"),
        }
    }
}

/// Run the agent loop for a single task.
///
/// This is the core execution function. It:
/// 1. Scans input for injection
/// 2. Loops: call LLM (streaming) -> execute tools -> repeat
/// 3. Emits events for every stage
/// 4. Saves checkpoints periodically
/// 5. Respects cancellation token
/// 6. Retries on transient errors
pub async fn execute_task(
    task: &mut AgentTask,
    agent_config: &AgentConfig,
    exec_config: &ExecutorConfig,
    provider: Arc<dyn LlmProvider>,
    tools: &ToolRegistry,
    sandbox: &Sandbox,
    store: Option<&TaskStore>,
    event_tx: mpsc::Sender<AgentEvent>,
    cancel: CancellationToken,
    initial_messages: Option<Vec<Message>>,
) -> Result<String> {
    // Set up guardian
    let mut guardian = Guardian::new(
        &task.id,
        &exec_config.guardian,
        &exec_config.audit_dir,
    )?;
    guardian.start_metering();

    // Emit task started
    task.mark_running();
    let _ = event_tx.send(AgentEvent::started(&task.id)).await;

    // Scan input for injection
    if let Err(e) = guardian.check_input(&task.input) {
        task.mark_failed(e.to_string());
        let _ = event_tx
            .send(AgentEvent::failed(&task.id, &e.to_string(), 0, false))
            .await;
        return Err(e);
    }

    // Build message history
    let mut messages = match initial_messages {
        Some(msgs) => msgs,
        None => {
            let mut msgs = Vec::new();
            if !agent_config.system_prompt.is_empty() {
                msgs.push(Message::system(&agent_config.system_prompt));
            }
            msgs.push(Message::user(&task.input));
            msgs
        }
    };

    let tool_defs = tools.definitions();
    let start = Instant::now();

    // Agent loop
    for iteration in 0..agent_config.max_iterations {
        // Check cancellation
        if cancel.is_cancelled() {
            task.mark_cancelled();
            let _ = event_tx.send(AgentEvent::cancelled(&task.id)).await;
            return Err(anyhow!("Task cancelled"));
        }

        // Check budget
        if let Err(e) = guardian.check_budget() {
            task.mark_failed(e.to_string());
            let _ = event_tx
                .send(AgentEvent::failed(&task.id, &e.to_string(), 0, false))
                .await;
            return Err(e);
        }

        // Check timeout
        if task.timeout_secs > 0 && start.elapsed().as_secs() > task.timeout_secs {
            task.mark_timed_out();
            let _ = event_tx
                .send(AgentEvent::TaskTimedOut {
                    task_id: task.id.clone(),
                    timeout_secs: task.timeout_secs,
                    timestamp: chrono::Utc::now(),
                })
                .await;
            return Err(anyhow!("Task timed out after {}s", task.timeout_secs));
        }

        tracing::debug!(iteration, task_id = %task.id, "Agent loop iteration");

        // Call LLM with retry
        let response = call_llm_with_retry(
            &provider,
            &messages,
            &tool_defs,
            &task.id,
            &event_tx,
            &cancel,
            exec_config.max_retries,
            exec_config.backoff_base_ms,
        )
        .await?;

        // Track tokens
        if let Some(ref usage) = response.usage {
            let total = (usage.prompt_tokens + usage.completion_tokens) as u64;
            guardian.add_tokens(total);
            task.tokens_used += total;
        }

        task.iterations = iteration as u32 + 1;

        // Emit iteration event
        let _ = event_tx
            .send(AgentEvent::IterationComplete {
                task_id: task.id.clone(),
                iteration: task.iterations,
                tokens_used: task.tokens_used,
            })
            .await;

        // Append assistant message
        if response.tool_calls.is_empty() {
            messages.push(Message::assistant_text(
                response.content.as_deref().unwrap_or(""),
            ));
        } else {
            messages.push(Message::assistant_tool_calls(response.tool_calls.clone()));
        }

        // No tool calls -> done
        if response.tool_calls.is_empty() {
            let output = response.content.unwrap_or_default();
            let duration_ms = start.elapsed().as_millis() as u64;

            task.mark_completed(&output);
            guardian.record_action("task_completed", &format!("output_len={}", output.len()))?;

            let _ = event_tx
                .send(AgentEvent::completed(
                    &task.id,
                    &output,
                    task.tokens_used,
                    task.tool_calls_made,
                    task.iterations,
                    duration_ms,
                ))
                .await;

            // Final checkpoint
            if let Some(store) = store {
                let _ = store.save_checkpoint(&task.id, &messages);
                let _ = store.save_task(task);
            }

            return Ok(output);
        }

        // Execute tool calls
        for call in &response.tool_calls {
            // Check cancellation between tool calls
            if cancel.is_cancelled() {
                task.mark_cancelled();
                let _ = event_tx.send(AgentEvent::cancelled(&task.id)).await;
                return Err(anyhow!("Task cancelled"));
            }

            // Authorize via guardian
            guardian.authorize_tool(&call.name, &call.arguments)?;

            let _ = event_tx
                .send(AgentEvent::tool_start(
                    &task.id,
                    &call.name,
                    call.arguments.clone(),
                ))
                .await;

            let tool_start = Instant::now();
            let result = tools.execute(call, sandbox).await;
            let duration_ms = tool_start.elapsed().as_millis() as u64;

            let (output, is_error) = match result {
                Ok(out) => (out, false),
                Err(e) => (format!("Error: {}", e), true),
            };

            task.tool_calls_made += 1;

            // Record in audit trail
            guardian.record_tool_result(&call.name, &output, is_error, duration_ms)?;

            let _ = event_tx
                .send(AgentEvent::tool_result(
                    &task.id,
                    &call.name,
                    &output,
                    is_error,
                    duration_ms,
                ))
                .await;

            messages.push(Message::tool_result(&call.id, &output));
        }

        // Periodic checkpoint
        if exec_config.checkpoint_interval > 0
            && task.iterations % exec_config.checkpoint_interval == 0
        {
            if let Some(store) = store {
                let _ = store.save_checkpoint(&task.id, &messages);
                let _ = store.save_task(task);
            }
        }
    }

    // Max iterations exhausted
    let err_msg = format!("Max iterations ({}) reached", agent_config.max_iterations);
    task.mark_failed(&err_msg);
    let _ = event_tx
        .send(AgentEvent::failed(&task.id, &err_msg, 0, false))
        .await;

    if let Some(store) = store {
        let _ = store.save_task(task);
    }

    Err(anyhow!("{}", err_msg))
}

/// Call LLM with streaming and retry on transient errors.
async fn call_llm_with_retry(
    provider: &Arc<dyn LlmProvider>,
    messages: &[Message],
    tools: &[ToolDef],
    task_id: &str,
    event_tx: &mpsc::Sender<AgentEvent>,
    cancel: &CancellationToken,
    max_retries: u32,
    backoff_base_ms: u64,
) -> Result<crate::llm::LlmResponse> {
    let mut last_error = None;

    for attempt in 0..=max_retries {
        if cancel.is_cancelled() {
            return Err(anyhow!("Cancelled during LLM call"));
        }

        match try_stream_llm(provider, messages, tools, task_id, event_tx).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                let err_str = e.to_string();
                let is_transient = err_str.contains("429")
                    || err_str.contains("500")
                    || err_str.contains("502")
                    || err_str.contains("503")
                    || err_str.contains("timeout")
                    || err_str.contains("timed out");

                if !is_transient || attempt == max_retries {
                    return Err(e);
                }

                let delay = Duration::from_millis(backoff_base_ms * 2u64.pow(attempt));
                tracing::warn!(
                    attempt,
                    delay_ms = delay.as_millis() as u64,
                    error = %e,
                    "Transient LLM error, retrying"
                );
                last_error = Some(e);

                tokio::select! {
                    _ = tokio::time::sleep(delay) => {},
                    _ = cancel.cancelled() => {
                        return Err(anyhow!("Cancelled during retry backoff"));
                    }
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("LLM call failed")))
}

/// Attempt a single streaming LLM call.
async fn try_stream_llm(
    provider: &Arc<dyn LlmProvider>,
    messages: &[Message],
    tools: &[ToolDef],
    task_id: &str,
    event_tx: &mpsc::Sender<AgentEvent>,
) -> Result<crate::llm::LlmResponse> {
    let mut stream = provider.chat_stream(messages, tools).await?;
    let mut accumulator = StreamAccumulator::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;

        match &chunk {
            StreamChunk::TextDelta { content } => {
                let _ = event_tx
                    .send(AgentEvent::text_delta(task_id, content))
                    .await;
            }
            StreamChunk::Error { message } => {
                return Err(anyhow!("Stream error: {}", message));
            }
            _ => {}
        }

        accumulator.push(&chunk);
    }

    Ok(accumulator.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::models::AgentTask;
    use crate::llm::{LlmResponse, StopReason, Usage};

    /// A mock provider that returns a fixed response (for testing the executor loop).
    struct MockProvider {
        response: String,
    }

    #[async_trait::async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(
            &self,
            _messages: &[Message],
            _tools: &[ToolDef],
        ) -> Result<LlmResponse> {
            Ok(LlmResponse {
                content: Some(self.response.clone()),
                tool_calls: vec![],
                stop_reason: StopReason::EndTurn,
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                }),
            })
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_executor_basic() {
        let provider = Arc::new(MockProvider {
            response: "Hello from agent!".into(),
        });
        let tools = crate::tools::builtin::default_registry();
        let sandbox = Sandbox::new("test_workspace").unwrap();
        let _ = sandbox.init_workspace();

        let (tx, mut rx) = mpsc::channel(64);
        let cancel = CancellationToken::new();

        let config = AgentConfig::default();
        let exec_config = ExecutorConfig {
            audit_dir: tempfile::tempdir().unwrap().into_path(),
            ..Default::default()
        };
        let mut task = AgentTask::new("Hello agent");

        let result = execute_task(
            &mut task,
            &config,
            &exec_config,
            provider,
            &tools,
            &sandbox,
            None,
            tx,
            cancel,
            None,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello from agent!");
        assert_eq!(task.state, crate::agent::models::TaskState::Completed);

        // Should have received events
        let mut events = vec![];
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        assert!(events.len() >= 2); // At least started + completed
    }

    #[tokio::test]
    async fn test_executor_cancellation() {
        let provider = Arc::new(MockProvider {
            response: "Should not finish".into(),
        });
        let tools = crate::tools::builtin::default_registry();
        let sandbox = Sandbox::new("test_workspace").unwrap();
        let _ = sandbox.init_workspace();

        let (tx, _rx) = mpsc::channel(64);
        let cancel = CancellationToken::new();
        cancel.cancel(); // Cancel immediately

        let config = AgentConfig::default();
        let exec_config = ExecutorConfig {
            audit_dir: tempfile::tempdir().unwrap().into_path(),
            ..Default::default()
        };
        let mut task = AgentTask::new("Test cancel");

        let result = execute_task(
            &mut task,
            &config,
            &exec_config,
            provider,
            &tools,
            &sandbox,
            None,
            tx,
            cancel,
            None,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(task.state, crate::agent::models::TaskState::Cancelled);
    }
}
