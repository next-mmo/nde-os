//! Kanban board tools for the agent ToolRegistry.
//!
//! Wraps the existing `mcp::kanban` logic as proper `Tool` implementations
//! so the LLM agent can natively create/list/update/delete Kanban tasks
//! through the executor's tool-calling loop — no frontend regex parsing needed.

use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

/// List all Kanban tasks.
pub struct KanbanGetTasksTool;

#[async_trait]
impl Tool for KanbanGetTasksTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "kanban_get_tasks".into(),
            description: "List all Kanban board tasks with their title, status, and filename."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn execute(&self, _args: serde_json::Value, _sandbox: &Sandbox) -> Result<String> {
        crate::mcp::kanban::execute("nde_kanban_get_tasks", &json!({}))
    }
}

/// Create a new Kanban task ticket.
pub struct KanbanCreateTaskTool;

#[async_trait]
impl Tool for KanbanCreateTaskTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "kanban_create_task".into(),
            description: "Create a new Kanban task ticket (markdown file in .agents/tasks/). Use tickets-writer format with Status, Description, Edge Cases, Task Checklist, and Definition of Done sections.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "The task title"
                    },
                    "content": {
                        "type": "string",
                        "description": "Full markdown content for the ticket. Use tickets-writer format: # Title, Status, Feature, Purpose, Description, Edge Cases, Task Checklist, Definition of Done. If not provided, a minimal skeleton is generated."
                    }
                },
                "required": ["title"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, _sandbox: &Sandbox) -> Result<String> {
        // If full content is provided, use the kanban create with content pass-through
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");

        // Build params compatible with kanban::execute
        let params = json!({
            "title": title,
            "description": content,
            "checklist": []
        });
        crate::mcp::kanban::execute("nde_kanban_create_task", &params)
    }
}

/// Update the status of a Kanban task.
pub struct KanbanUpdateTaskTool;

#[async_trait]
impl Tool for KanbanUpdateTaskTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "kanban_update_task".into(),
            description: "Update the status of a Kanban task. Valid statuses: Plan, YOLO mode, Done by AI, Verified, Re-open.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "The markdown filename of the task (e.g. 'my-task.md')"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["Plan", "YOLO mode", "Done by AI", "Verified", "Re-open"],
                        "description": "The new status"
                    }
                },
                "required": ["filename", "status"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, _sandbox: &Sandbox) -> Result<String> {
        crate::mcp::kanban::execute("nde_kanban_update_task", &args)
    }
}

/// Delete a Kanban task.
pub struct KanbanDeleteTaskTool;

#[async_trait]
impl Tool for KanbanDeleteTaskTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "kanban_delete_task".into(),
            description: "Delete a Kanban task ticket by filename.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "The markdown filename of the task to delete (e.g. 'my-task.md')"
                    }
                },
                "required": ["filename"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, _sandbox: &Sandbox) -> Result<String> {
        crate::mcp::kanban::execute("nde_kanban_delete_task", &args)
    }
}

/// Read full content of a Kanban task.
pub struct KanbanGetTaskContentTool;

#[async_trait]
impl Tool for KanbanGetTaskContentTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "kanban_get_task_content".into(),
            description: "Read the full markdown content of a Kanban task ticket.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "The markdown filename of the task (e.g. 'my-task.md')"
                    }
                },
                "required": ["filename"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, _sandbox: &Sandbox) -> Result<String> {
        crate::mcp::kanban::execute("nde_kanban_get_task_content", &args)
    }
}

/// Update the full content of a Kanban task.
pub struct KanbanUpdateTaskContentTool;

#[async_trait]
impl Tool for KanbanUpdateTaskContentTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "kanban_update_task_content".into(),
            description: "Update the full markdown content of a Kanban task ticket.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "The markdown filename of the task"
                    },
                    "content": {
                        "type": "string",
                        "description": "The new full markdown content"
                    }
                },
                "required": ["filename", "content"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, _sandbox: &Sandbox) -> Result<String> {
        crate::mcp::kanban::execute("nde_kanban_update_task_content", &args)
    }
}
