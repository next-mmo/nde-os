use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use std::fs;
use std::path::Path;

pub struct FileDeleteTool;

#[async_trait]
impl Tool for FileDeleteTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "file_delete".into(),
            description: "Delete a file or empty directory inside the sandbox.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file or directory within the sandbox"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "If true, delete directory and all contents (default: false)",
                        "default": false
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let path_str = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let recursive = args.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let full_path = sandbox.resolve(Path::new(path_str))?;

        if !full_path.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {}", path_str));
        }

        if full_path.is_dir() {
            if recursive {
                fs::remove_dir_all(&full_path)?;
                Ok(format!("Deleted directory recursively: {}", path_str))
            } else {
                fs::remove_dir(&full_path)
                    .map_err(|e| anyhow::anyhow!("Failed to delete (directory not empty? use recursive=true): {}", e))?;
                Ok(format!("Deleted empty directory: {}", path_str))
            }
        } else {
            fs::remove_file(&full_path)?;
            Ok(format!("Deleted file: {}", path_str))
        }
    }
}
