use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::system_metrics;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;

/// Returns system resource usage (CPU, memory, disk).
pub struct SystemInfoTool;

#[async_trait]
impl Tool for SystemInfoTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "system_info".into(),
            description: "Get system information: OS, architecture, memory usage, disk usage, and sandbox status.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn execute(&self, _args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let mut output = String::from("=== NDE-OS System Info ===\n\n");

        // OS info
        output.push_str(&format!(
            "Platform: {} {}\n",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));

        // Sandbox info
        output.push_str(&format!("Sandbox root: {}\n", sandbox.root().display()));

        let sandbox_usage = sandbox.disk_usage().unwrap_or(0);
        output.push_str(&format!(
            "Sandbox disk usage: {}\n",
            format_bytes(sandbox_usage)
        ));

        // System resources
        match system_metrics::snapshot_resource_usage(sandbox.root()) {
            Ok(usage) => {
                output.push_str(&format!(
                    "\nMemory: {} / {} ({}%)\n",
                    format_bytes(usage.memory_used_bytes),
                    format_bytes(usage.memory_total_bytes),
                    usage.memory_percent,
                ));
                output.push_str(&format!(
                    "Disk:   {} / {} ({}%) on {}\n",
                    format_bytes(usage.disk_used_bytes),
                    format_bytes(usage.disk_total_bytes),
                    usage.disk_percent,
                    usage.disk_mount_point,
                ));
            }
            Err(e) => {
                output.push_str(&format!("\nResource metrics unavailable: {}\n", e));
            }
        }

        // Sandbox verification
        let verify = sandbox.verify();
        output.push_str(&format!(
            "\nSandbox security:\n  Path traversal blocked: {}\n  Absolute escape blocked: {}\n  Symlink escape blocked: {}\n  Valid paths work: {}\n",
            if verify.path_traversal_blocked { "✅" } else { "❌" },
            if verify.absolute_escape_blocked { "✅" } else { "❌" },
            if verify.symlink_escape_blocked { "✅" } else { "❌" },
            if verify.valid_path_works { "✅" } else { "❌" },
        ));

        Ok(output)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
