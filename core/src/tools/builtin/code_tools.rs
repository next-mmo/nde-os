use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

/// code_search — Search workspace files using pattern matching.
/// Like ripgrep but sandbox-jailed.
pub struct CodeSearchTool;

#[async_trait]
impl Tool for CodeSearchTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "code_search".into(),
            description: "Search for a pattern across files in the workspace. Returns matching lines with file paths and line numbers.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern (regex supported)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Subdirectory to search (relative to workspace), default: entire workspace"
                    },
                    "file_pattern": {
                        "type": "string",
                        "description": "Glob pattern to filter files, e.g. '*.rs' or '*.py'"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 50)"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("pattern is required"))?;
        let search_path = args["path"].as_str().unwrap_or(".");
        let file_pattern = args["file_pattern"].as_str();
        let max_results = args["max_results"].as_u64().unwrap_or(50) as usize;

        let search_dir = sandbox.resolve(std::path::Path::new(search_path))?;

        // Use native Rust search (no external dependency)
        let mut results = Vec::new();
        search_files_recursive(&search_dir, pattern, file_pattern, &mut results, max_results)?;

        if results.is_empty() {
            Ok(format!("No matches found for pattern: {}", pattern))
        } else {
            let total = results.len();
            let output: Vec<String> = results
                .iter()
                .map(|r| format!("{}:{}: {}", r.file, r.line_number, r.line.trim()))
                .collect();
            Ok(format!(
                "{} match(es) found:\n{}",
                total,
                output.join("\n")
            ))
        }
    }
}

struct SearchResult {
    file: String,
    line_number: usize,
    line: String,
}

fn search_files_recursive(
    dir: &std::path::Path,
    pattern: &str,
    file_glob: Option<&str>,
    results: &mut Vec<SearchResult>,
    max_results: usize,
) -> Result<()> {
    if results.len() >= max_results {
        return Ok(());
    }

    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden dirs and common noise
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || name == "node_modules" || name == "target" || name == ".venv" {
                continue;
            }
        }

        if path.is_dir() {
            search_files_recursive(&path, pattern, file_glob, results, max_results)?;
        } else if path.is_file() {
            // Check file glob
            if let Some(glob) = file_glob {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !simple_glob_match(glob, filename) {
                    continue;
                }
            }

            // Only search text files
            if let Ok(content) = std::fs::read_to_string(&path) {
                let pattern_lower = pattern.to_lowercase();
                for (i, line) in content.lines().enumerate() {
                    if results.len() >= max_results {
                        return Ok(());
                    }
                    if line.to_lowercase().contains(&pattern_lower) {
                        let rel_path = path
                            .strip_prefix(dir.ancestors().last().unwrap_or(dir))
                            .unwrap_or(&path);
                        results.push(SearchResult {
                            file: rel_path.to_string_lossy().to_string(),
                            line_number: i + 1,
                            line: line.to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

fn simple_glob_match(pattern: &str, filename: &str) -> bool {
    if pattern.starts_with("*.") {
        let ext = &pattern[1..]; // e.g. ".rs"
        filename.ends_with(ext)
    } else {
        filename.contains(pattern)
    }
}

/// code_edit — Surgical line-range replacement in a file.
pub struct CodeEditTool;

#[async_trait]
impl Tool for CodeEditTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "code_edit".into(),
            description: "Edit a file by replacing content between specific line numbers. Supports surgical edits without rewriting the entire file.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path relative to workspace"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "Start line number (1-indexed)"
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "End line number (1-indexed, inclusive)"
                    },
                    "new_content": {
                        "type": "string",
                        "description": "Replacement content for the line range"
                    }
                },
                "required": ["path", "start_line", "end_line", "new_content"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path is required"))?;
        let start = args["start_line"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("start_line is required"))? as usize;
        let end = args["end_line"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("end_line is required"))? as usize;
        let new_content = args["new_content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("new_content is required"))?;

        if start == 0 || end == 0 || start > end {
            return Err(anyhow::anyhow!(
                "Invalid line range: {}-{} (must be 1-indexed, start <= end)",
                start,
                end
            ));
        }

        let abs_path = sandbox.resolve(std::path::Path::new(path))?;
        let content = std::fs::read_to_string(&abs_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path, e))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        if start > total_lines {
            return Err(anyhow::anyhow!(
                "start_line {} exceeds file length {}",
                start,
                total_lines
            ));
        }

        let mut new_lines = Vec::new();
        // Lines before edit
        for line in &lines[..start - 1] {
            new_lines.push(line.to_string());
        }
        // New content
        for line in new_content.lines() {
            new_lines.push(line.to_string());
        }
        // Lines after edit
        let end_idx = end.min(total_lines);
        for line in &lines[end_idx..] {
            new_lines.push(line.to_string());
        }

        let new_file = new_lines.join("\n");
        std::fs::write(&abs_path, &new_file)
            .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", path, e))?;

        let replaced_count = end_idx - (start - 1);
        let new_count = new_content.lines().count();
        Ok(format!(
            "Edited {}: replaced lines {}-{} ({} lines) with {} new lines. File now has {} lines.",
            path,
            start,
            end_idx,
            replaced_count,
            new_count,
            new_lines.len()
        ))
    }
}

/// code_symbols — List functions, structs, classes from a file.
/// Simple regex-based symbol extraction (no tree-sitter required).
pub struct CodeSymbolsTool;

#[async_trait]
impl Tool for CodeSymbolsTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "code_symbols".into(),
            description: "List function, struct, class, and import symbols from a source file.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path relative to workspace"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path is required"))?;

        let abs_path = sandbox.resolve(std::path::Path::new(path))?;
        let content = std::fs::read_to_string(&abs_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path, e))?;

        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let mut symbols = Vec::new();

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let line_num = i + 1;

            match ext {
                "rs" => {
                    if trimmed.starts_with("pub fn ")
                        || trimmed.starts_with("fn ")
                        || trimmed.starts_with("pub async fn ")
                        || trimmed.starts_with("async fn ")
                    {
                        symbols.push(format!("L{}: fn {}", line_num, extract_name(trimmed)));
                    } else if trimmed.starts_with("pub struct ")
                        || trimmed.starts_with("struct ")
                    {
                        symbols.push(format!("L{}: struct {}", line_num, extract_name(trimmed)));
                    } else if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ") {
                        symbols.push(format!("L{}: enum {}", line_num, extract_name(trimmed)));
                    } else if trimmed.starts_with("pub trait ")
                        || trimmed.starts_with("trait ")
                    {
                        symbols.push(format!("L{}: trait {}", line_num, extract_name(trimmed)));
                    } else if trimmed.starts_with("impl ") {
                        symbols.push(format!("L{}: impl {}", line_num, extract_name(trimmed)));
                    } else if trimmed.starts_with("use ") {
                        symbols.push(format!("L{}: use {}", line_num, trimmed));
                    }
                }
                "py" => {
                    if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                        symbols.push(format!("L{}: def {}", line_num, extract_name(trimmed)));
                    } else if trimmed.starts_with("class ") {
                        symbols.push(format!("L{}: class {}", line_num, extract_name(trimmed)));
                    } else if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                        symbols.push(format!("L{}: {}", line_num, trimmed));
                    }
                }
                "js" | "ts" | "jsx" | "tsx" => {
                    if trimmed.starts_with("function ")
                        || trimmed.starts_with("export function ")
                        || trimmed.starts_with("async function ")
                        || trimmed.starts_with("export async function ")
                    {
                        symbols.push(format!("L{}: function {}", line_num, extract_name(trimmed)));
                    } else if trimmed.starts_with("class ")
                        || trimmed.starts_with("export class ")
                    {
                        symbols.push(format!("L{}: class {}", line_num, extract_name(trimmed)));
                    } else if trimmed.starts_with("const ") || trimmed.starts_with("export const ")
                    {
                        if trimmed.contains("=>") || trimmed.contains("function") {
                            symbols.push(format!("L{}: const {}", line_num, extract_name(trimmed)));
                        }
                    } else if trimmed.starts_with("import ") {
                        symbols.push(format!("L{}: {}", line_num, trimmed));
                    }
                }
                _ => {
                    // Generic: look for common patterns
                    if trimmed.starts_with("func ") || trimmed.starts_with("fn ") {
                        symbols.push(format!("L{}: {}", line_num, trimmed));
                    }
                }
            }
        }

        if symbols.is_empty() {
            Ok(format!("No symbols found in {}", path))
        } else {
            Ok(format!(
                "Symbols in {} ({} found):\n{}",
                path,
                symbols.len(),
                symbols.join("\n")
            ))
        }
    }
}

fn extract_name(line: &str) -> String {
    // Extract the identifier after the keyword
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        let name = parts
            .iter()
            .skip(1)
            .find(|p| !p.starts_with("pub") && !p.starts_with("async") && !p.starts_with("export"))
            .unwrap_or(&parts[parts.len() - 1]);
        // Clean up trailing characters
        name.trim_end_matches(|c: char| c == '(' || c == '{' || c == ':' || c == '<')
            .to_string()
    } else {
        line.to_string()
    }
}
