//! Git operations tool — run git commands within the sandbox workspace.
//!
//! Provides safe git operations: status, diff, log, branch, add, commit, checkout.
//! All operations are sandboxed to the workspace directory.

use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Command;

pub struct GitTool;

#[async_trait]
impl Tool for GitTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "git".into(),
            description: "Run git operations in the workspace. Supports: status, diff, log, branch, add, commit, checkout, stash, remote, tag. Use this to manage version control.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "enum": ["status", "diff", "log", "branch", "add", "commit", "checkout", "stash", "remote", "tag", "init", "pull", "push", "merge", "rebase", "reset", "show"],
                        "description": "The git command to run"
                    },
                    "args": {
                        "type": "string",
                        "description": "Additional arguments for the command (e.g., file paths, branch names, commit messages)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' argument"))?;

        let extra_args = args.get("args")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Validate command is allowed
        let allowed = [
            "status", "diff", "log", "branch", "add", "commit", "checkout",
            "stash", "remote", "tag", "init", "pull", "push", "merge",
            "rebase", "reset", "show",
        ];
        if !allowed.contains(&command) {
            return Err(anyhow::anyhow!("Disallowed git command: {}", command));
        }

        // Block dangerous operations
        if command == "push" && extra_args.contains("--force") {
            return Err(anyhow::anyhow!("Force push is not allowed for safety"));
        }
        if command == "reset" && extra_args.contains("--hard") {
            return Err(anyhow::anyhow!("Hard reset is not allowed for safety"));
        }

        let workspace = sandbox.resolve(std::path::Path::new("."))?;

        // Build git command
        let mut cmd = Command::new("git");
        cmd.arg(command);
        cmd.current_dir(&workspace);

        // Add extra args (split by whitespace, respecting quoted strings)
        if !extra_args.is_empty() {
            // Special handling for commit -m
            if command == "commit" && extra_args.contains("-m") {
                // Pass the whole args string to git
                for arg in shell_split(extra_args) {
                    cmd.arg(arg);
                }
            } else {
                for arg in extra_args.split_whitespace() {
                    cmd.arg(arg);
                }
            }
        }

        // Add safe defaults
        match command {
            "log" => {
                if !extra_args.contains("--") {
                    // Default to last 20 commits, one-line format
                    if !extra_args.contains("-n") && !extra_args.contains("--max-count") {
                        cmd.arg("-n").arg("20");
                    }
                    if !extra_args.contains("--format") && !extra_args.contains("--oneline") && !extra_args.contains("--pretty") {
                        cmd.arg("--oneline");
                    }
                }
            }
            "diff" => {
                if !extra_args.contains("--stat") && !extra_args.contains("--name-only") {
                    // Add stat for overview
                    cmd.arg("--stat");
                }
            }
            _ => {}
        }

        // Execute with timeout
        let output = cmd.output()
            .map_err(|e| anyhow::anyhow!("Failed to run git {}: {}", command, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();
        result.push_str(&format!("$ git {} {}\n\n", command, extra_args));

        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !stdout.is_empty() {
                result.push('\n');
            }
            result.push_str(&stderr);
        }

        if output.status.success() {
            if result.trim().ends_with(&format!("git {} {}", command, extra_args).trim()) {
                result.push_str("(no output)");
            }
        } else {
            result.push_str(&format!("\nExit code: {}", output.status.code().unwrap_or(-1)));
        }

        // Truncate large output
        if result.len() > 50_000 {
            result.truncate(50_000);
            result.push_str("\n...[truncated]");
        }

        Ok(result)
    }
}

/// Split a string respecting quoted segments.
fn shell_split(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = None;

    for ch in s.chars() {
        match (ch, in_quote) {
            ('"', None) => in_quote = Some('"'),
            ('"', Some('"')) => in_quote = None,
            ('\'', None) => in_quote = Some('\''),
            ('\'', Some('\'')) => in_quote = None,
            (' ', None) => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_split() {
        let args = shell_split("-m \"Initial commit\" --author test");
        assert_eq!(args, vec!["-m", "Initial commit", "--author", "test"]);
    }

    #[test]
    fn test_shell_split_single_quotes() {
        let args = shell_split("-m 'hello world'");
        assert_eq!(args, vec!["-m", "hello world"]);
    }
}
