use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

pub struct FilePatchTool;

#[async_trait]
impl Tool for FilePatchTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "file_patch".into(),
            description: "Apply a search-and-replace edit to a file inside the sandbox. More precise than rewriting the entire file.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file within the sandbox"
                    },
                    "search": {
                        "type": "string",
                        "description": "Exact text to find in the file"
                    },
                    "replace": {
                        "type": "string",
                        "description": "Replacement text"
                    },
                    "all": {
                        "type": "boolean",
                        "description": "Replace all occurrences (default: false, replaces only first)",
                        "default": false
                    }
                },
                "required": ["path", "search", "replace"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let path_str = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let search = args.get("search")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'search' argument"))?;

        let replace = args.get("replace")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'replace' argument"))?;

        let replace_all = args.get("all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let full_path = sandbox.resolve(Path::new(path_str))?;

        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path_str, e))?;

        if !content.contains(search) {
            return Err(anyhow::anyhow!(
                "Search text not found in {}. Verify the exact text to match.",
                path_str
            ));
        }

        let (new_content, count) = if replace_all {
            let count = content.matches(search).count();
            (content.replace(search, replace), count)
        } else {
            (content.replacen(search, replace, 1), 1)
        };

        std::fs::write(&full_path, &new_content)?;

        Ok(format!("Patched {}: {} replacement(s) applied", path_str, count))
    }
}
