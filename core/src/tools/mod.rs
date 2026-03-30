pub mod builtin;

use crate::llm::{ToolCall, ToolDef};
use crate::sandbox::Sandbox;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;

/// Trait for agent tools that can be called by the LLM.
#[async_trait]
pub trait Tool: Send + Sync {
    /// JSON Schema definition for the LLM to understand the tool.
    fn definition(&self) -> ToolDef;

    /// Execute the tool with the given arguments inside the sandbox.
    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String>;
}

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let def = tool.definition();
        self.tools.insert(def.name.clone(), tool);
    }

    /// Get tool definitions for the LLM.
    pub fn definitions(&self) -> Vec<ToolDef> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Execute a tool call from the LLM.
    pub async fn execute(&self, call: &ToolCall, sandbox: &Sandbox) -> Result<String> {
        let tool = self
            .tools
            .get(&call.name)
            .ok_or_else(|| anyhow!("Unknown tool: {}", call.name))?;
        tool.execute(call.arguments.clone(), sandbox).await
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
