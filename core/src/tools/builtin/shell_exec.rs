use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Command;

pub struct ShellExecTool;

#[async_trait]
impl Tool for ShellExecTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "shell_exec".into(),
            description: "Execute a shell command inside the sandbox workspace. The command runs with the sandbox environment variables.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 30)",
                        "default": 30
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

        let timeout_secs = args.get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        let env_vars = sandbox.env_vars();

        // Platform-specific shell
        let mut child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", command])
                .current_dir(sandbox.root())
                .envs(env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| anyhow::anyhow!("Failed to spawn command: {}", e))?
        } else {
            Command::new("sh")
                .args(["-c", command])
                .current_dir(sandbox.root())
                .envs(env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| anyhow::anyhow!("Failed to spawn command: {}", e))?
        };

        // Wait with timeout
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let stdout = child.stdout.take()
                        .map(|mut s| {
                            let mut buf = String::new();
                            std::io::Read::read_to_string(&mut s, &mut buf).ok();
                            buf
                        })
                        .unwrap_or_default();

                    let stderr = child.stderr.take()
                        .map(|mut s| {
                            let mut buf = String::new();
                            std::io::Read::read_to_string(&mut s, &mut buf).ok();
                            buf
                        })
                        .unwrap_or_default();

                    let mut output = String::new();
                    if !stdout.is_empty() {
                        output.push_str(&stdout);
                    }
                    if !stderr.is_empty() {
                        if !output.is_empty() { output.push('\n'); }
                        output.push_str("[stderr] ");
                        output.push_str(&stderr);
                    }
                    if !status.success() {
                        output.push_str(&format!("\n[exit code: {}]", status.code().unwrap_or(-1)));
                    }

                    // Truncate
                    if output.len() > 50_000 {
                        return Ok(format!("{}\n... [truncated, {} total bytes]", &output[..50_000], output.len()));
                    }
                    return Ok(output);
                }
                Ok(None) => {
                    if start.elapsed().as_secs() >= timeout_secs {
                        child.kill().ok();
                        return Ok(format!("[timeout after {}s]", timeout_secs));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => return Err(anyhow::anyhow!("Failed to wait for command: {}", e)),
            }
        }
    }
}
