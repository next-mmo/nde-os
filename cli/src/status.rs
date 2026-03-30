use anyhow::Result;
use colored::Colorize;

/// Show system status dashboard.
pub async fn show_status(client: &reqwest::Client, api: &str) -> Result<()> {
    println!("{}", "╔══════════════════════════════════════╗".cyan());
    println!("{}", "║        NDE-OS System Status           ║".cyan());
    println!("{}", "╚══════════════════════════════════════╝".cyan());

    // System info
    println!("\n{}", "System".bold().underline());
    match client.get(format!("{}/api/system", api)).send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let d = &json["data"];
                println!("  OS:          {}", d["os"].as_str().unwrap_or("?"));
                println!("  Arch:        {}", d["arch"].as_str().unwrap_or("?"));
                println!(
                    "  Python:      {}",
                    d["python_version"].as_str().unwrap_or("not found")
                );
                println!(
                    "  GPU:         {}",
                    if d["gpu_detected"].as_bool() == Some(true) {
                        "detected".green()
                    } else {
                        "none".yellow()
                    }
                );
                println!(
                    "  Apps:        {} total, {} running",
                    d["total_apps"].as_u64().unwrap_or(0),
                    d["running_apps"].as_u64().unwrap_or(0)
                );
            }
        }
        Err(e) => {
            eprintln!("  {} {}", "Error:".red(), e);
        }
    }

    // Resources
    println!("\n{}", "Resources".bold().underline());
    match client
        .get(format!("{}/api/system/resources", api))
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let d = &json["data"];
                let mem_pct = d["memory_percent"].as_u64().unwrap_or(0);
                let disk_pct = d["disk_percent"].as_u64().unwrap_or(0);

                let mem_bar = progress_bar(mem_pct as f64 / 100.0, 20);
                let disk_bar = progress_bar(disk_pct as f64 / 100.0, 20);

                println!("  Memory: {} {}%", mem_bar, mem_pct);
                println!("  Disk:   {} {}%", disk_bar, disk_pct);
            }
        }
        Err(_) => {
            println!("  {}", "Could not fetch resource info".dimmed());
        }
    }

    // Health
    println!("\n{}", "Health".bold().underline());
    match client.get(format!("{}/api/health", api)).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("  API: {}", "● Online".green());
            } else {
                println!("  API: {}", "● Error".red());
            }
        }
        Err(_) => {
            println!("  API: {}", "● Offline".red());
        }
    }

    Ok(())
}

fn progress_bar(fraction: f64, width: usize) -> String {
    let filled = (fraction * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));
    if fraction > 0.9 {
        bar.red().to_string()
    } else if fraction > 0.7 {
        bar.yellow().to_string()
    } else {
        bar.green().to_string()
    }
}
