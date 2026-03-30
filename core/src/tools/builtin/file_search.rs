use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct FileSearchTool;

#[async_trait]
impl Tool for FileSearchTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "file_search".into(),
            description: "Search for text patterns across files in the sandbox. Returns matching lines with file paths and line numbers.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Text pattern to search for (case-insensitive)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search in (default: sandbox root)",
                        "default": "."
                    },
                    "extensions": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "File extensions to include (e.g. [\"py\", \"rs\", \"js\"]). Empty = all files."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of matching lines to return (default: 50)",
                        "default": 50
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' argument"))?;

        let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let extensions: Vec<String> = args
            .get("extensions")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let full_path = sandbox.resolve(Path::new(path_str))?;
        let query_lower = query.to_lowercase();

        let mut results = Vec::new();
        search_dir(
            &full_path,
            sandbox.root(),
            &query_lower,
            &extensions,
            max_results,
            &mut results,
        )?;

        if results.is_empty() {
            Ok(format!("No matches found for '{}'", query))
        } else {
            let total = results.len();
            let truncated = total >= max_results;
            let mut output = results.join("\n");
            if truncated {
                output.push_str(&format!(
                    "\n\n... [showing {}/{} max results]",
                    total, max_results
                ));
            }
            Ok(output)
        }
    }
}

fn search_dir(
    dir: &Path,
    root: &Path,
    query: &str,
    extensions: &[String],
    max_results: usize,
    results: &mut Vec<String>,
) -> Result<()> {
    if results.len() >= max_results {
        return Ok(());
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()), // Skip unreadable dirs
    };

    for entry in entries.filter_map(|e| e.ok()) {
        if results.len() >= max_results {
            break;
        }

        let path = entry.path();
        let ft = entry
            .file_type()
            .unwrap_or_else(|_| fs::metadata(&path).unwrap().file_type());

        if ft.is_dir() {
            // Skip hidden directories
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with('.') {
                search_dir(&path, root, query, extensions, max_results, results)?;
            }
        } else if ft.is_file() {
            // Check extension filter
            if !extensions.is_empty() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if !extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                    continue;
                }
            }

            // Skip binary files (check first 512 bytes)
            if is_likely_binary(&path) {
                continue;
            }

            let rel = path.strip_prefix(root).unwrap_or(&path);
            search_file(&path, rel, query, max_results, results)?;
        }
    }

    Ok(())
}

fn search_file(
    path: &Path,
    rel_path: &Path,
    query: &str,
    max_results: usize,
    results: &mut Vec<String>,
) -> Result<()> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Ok(()),
    };

    let reader = BufReader::new(file);
    for (i, line) in reader.lines().enumerate() {
        if results.len() >= max_results {
            break;
        }
        if let Ok(line) = line {
            if line.to_lowercase().contains(query) {
                let display_path = rel_path.to_string_lossy();
                let trimmed = line.trim();
                // Truncate very long lines
                let content = if trimmed.len() > 200 {
                    format!("{}...", &trimmed[..200])
                } else {
                    trimmed.to_string()
                };
                results.push(format!("{}:{}: {}", display_path, i + 1, content));
            }
        }
    }

    Ok(())
}

fn is_likely_binary(path: &Path) -> bool {
    let mut buf = [0u8; 512];
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return true,
    };
    let n = match std::io::Read::read(&mut &file, &mut buf) {
        Ok(n) => n,
        Err(_) => return true,
    };
    buf[..n].contains(&0)
}
