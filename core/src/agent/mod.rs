pub mod config;
pub mod executor;
pub mod guardian;
pub mod heartbeat;
pub mod manager;
pub mod models;
pub mod protocol;
pub mod scheduler;
pub mod store;

use crate::llm::{self, LlmProvider, LlmResponse, Message};
use crate::sandbox::Sandbox;
use crate::tools::ToolRegistry;
use anyhow::{anyhow, Result};
use config::AgentConfig;

// Re-exports for convenience
pub use manager::AgentManager;
pub use models::{AgentTask, TaskFilter, TaskState};
pub use protocol::AgentEvent;

/// Core agent runtime: state machine driving LLM <-> tool loop.
/// Kept for backward compat (CLI single-turn use).
/// For server/desktop, use `AgentManager` instead.
pub struct AgentRuntime {
    pub config: AgentConfig,
    provider: Box<dyn LlmProvider>,
    tools: ToolRegistry,
    sandbox: Sandbox,
    messages: Vec<Message>,
}

impl AgentRuntime {
    pub fn new(
        config: AgentConfig,
        provider: Box<dyn LlmProvider>,
        tools: ToolRegistry,
        sandbox: Sandbox,
    ) -> Self {
        let mut messages = Vec::new();
        if !config.system_prompt.is_empty() {
            messages.push(Message::system(&config.system_prompt));
        }
        Self {
            config,
            provider,
            tools,
            sandbox,
            messages,
        }
    }

    /// Build a runtime from config, auto-selecting provider and tools.
    pub fn from_config(config: AgentConfig) -> Result<Self> {
        let provider = llm::create_provider(
            &config.model_provider,
            &config.model_name,
            config.base_url.as_deref(),
            config.api_key.as_deref(),
        )?;

        let tools = crate::tools::builtin::default_registry();
        let sandbox = Sandbox::new(&config.workspace)?;
        sandbox.init_workspace()?;

        Ok(Self::new(config, provider, tools, sandbox))
    }

    /// Run the agent loop for a single user turn.
    /// Returns the final text response.
    pub async fn run(&mut self, user_message: &str) -> Result<String> {
        self.messages.push(Message::user(user_message));

        for iteration in 0..self.config.max_iterations {
            tracing::debug!(iteration, "Agent loop iteration");

            let defs = self.tools.definitions();
            let resp = self.provider.chat(&self.messages, &defs).await?;

            // Track usage
            if let Some(ref usage) = resp.usage {
                tracing::info!(
                    prompt_tokens = usage.prompt_tokens,
                    completion_tokens = usage.completion_tokens,
                    "LLM usage"
                );
            }

            self.append_assistant_message(&resp);

            // No tool calls -> return text
            if resp.tool_calls.is_empty() {
                return Ok(resp.content.unwrap_or_default());
            }

            // Execute tool calls
            for call in &resp.tool_calls {
                tracing::info!(tool = %call.name, "Executing tool");
                let result = self.tools.execute(call, &self.sandbox).await;
                let output = match result {
                    Ok(out) => out,
                    Err(e) => format!("Error: {}", e),
                };
                self.messages.push(Message::tool_result(&call.id, &output));
            }
        }

        Err(anyhow!(
            "Max iterations ({}) reached",
            self.config.max_iterations
        ))
    }

    /// Reset conversation history (keep system prompt).
    pub fn reset(&mut self) {
        self.messages.clear();
        if !self.config.system_prompt.is_empty() {
            self.messages
                .push(Message::system(&self.config.system_prompt));
        }
    }

    fn append_assistant_message(&mut self, resp: &LlmResponse) {
        if resp.tool_calls.is_empty() {
            self.messages.push(Message::assistant_text(
                resp.content.as_deref().unwrap_or(""),
            ));
        } else {
            self.messages
                .push(Message::assistant_tool_calls(resp.tool_calls.clone()));
        }
    }
}
