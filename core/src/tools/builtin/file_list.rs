use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use std::fs;
use std::path::Path;

pub struct FileListTool;

#[async_trait]
impl Tool for FileListTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "file_list".into(),
            description: "List files and directories inside the sandbox. Returns name, type (file/dir), and size.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative directory path within the sandbox (default: root)",
                        "default": "."
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Whether to list recursively (default: false)",
                        "default": false
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Maximum recursion depth (default: 3)",
                        "default": 3
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let max_depth = args.get("max_depth").and_then(|v| v.as_u64()).unwrap_or(3) as usize;

        let full_path = sandbox.resolve(Path::new(path_str))?;

        if !full_path.is_dir() {
            return Err(anyhow::anyhow!("Not a directory: {}", path_str));
        }

        let mut entries = Vec::new();
        list_dir(
            &full_path,
            sandbox.root(),
            recursive,
            0,
            max_depth,
            &mut entries,
        )?;

        // Cap output
        if entries.len() > 500 {
            entries.truncate(500);
            entries.push("... [truncated at 500 entries]".into());
        }

        Ok(entries.join("\n"))
    }
}

fn list_dir(
    dir: &Path,
    root: &Path,
    recursive: bool,
    depth: usize,
    max_depth: usize,
    output: &mut Vec<String>,
) -> Result<()> {
    let indent = "  ".repeat(depth);

    let mut items: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    items.sort_by_key(|e| e.file_name());

    for entry in items {
        let ft = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().to_string();
        let _rel = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(&entry.path())
            .to_string_lossy()
            .to_string();

        if ft.is_dir() {
            output.push(format!("{}{}/", indent, name));
            if recursive && depth < max_depth {
                list_dir(&entry.path(), root, true, depth + 1, max_depth, output)?;
            }
        } else if ft.is_file() {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let size_str = format_size(size);
            output.push(format!("{}{} ({})", indent, name, size_str));
        }
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
