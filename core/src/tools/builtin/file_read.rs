use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "file_read".into(),
            description: "Read the contents of a file inside the sandbox.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file within the sandbox workspace"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let full_path = sandbox.resolve(std::path::Path::new(path_str))?;

        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path_str, e))?;

        // Truncate very large files
        if content.len() > 100_000 {
            Ok(format!(
                "{}\n\n... [truncated, {} total bytes]",
                &content[..100_000],
                content.len()
            ))
        } else {
            Ok(content)
        }
    }
}
