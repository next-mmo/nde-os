use anyhow::Result;
use colored::Colorize;
use futures::StreamExt;
use std::io::{self, BufRead, Write};

/// Send a one-shot message to the agent.
pub async fn send_message(
    client: &reqwest::Client,
    api: &str,
    message: &str,
    stream: bool,
    conversation_id: Option<&str>,
    provider: Option<&str>,
) -> Result<()> {
    if stream {
        send_streaming(client, api, message, conversation_id, provider).await
    } else {
        send_blocking(client, api, message, conversation_id, provider).await
    }
}

async fn send_blocking(
    client: &reqwest::Client,
    api: &str,
    message: &str,
    conversation_id: Option<&str>,
    provider: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({"message": message});
    if let Some(cid) = conversation_id {
        body["conversation_id"] = serde_json::json!(cid);
    }
    if let Some(p) = provider {
        body["provider"] = serde_json::json!(p);
    }

    let resp: serde_json::Value = client
        .post(format!("{}/api/agent/chat", api))
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    if let Some(content) = resp["data"]["content"].as_str() {
        println!("\n{}", content);
    } else {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    }

    Ok(())
}

async fn send_streaming(
    client: &reqwest::Client,
    api: &str,
    message: &str,
    conversation_id: Option<&str>,
    provider: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({"message": message, "stream": true});
    if let Some(cid) = conversation_id {
        body["conversation_id"] = serde_json::json!(cid);
    }
    if let Some(p) = provider {
        body["provider"] = serde_json::json!(p);
    }

    let resp = client
        .post(format!("{}/api/agent/chat/stream", api))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        eprintln!("{}: {}", "Error".red(), text);
        return Ok(());
    }

    // Read SSE stream
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    print!("\n{} ", "AI:".cyan().bold());
    io::stdout().flush()?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process SSE events
        while let Some(pos) = buffer.find("\n\n") {
            let event = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            for line in event.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        println!();
                        return Ok(());
                    }
                    if let Ok(chunk) =
                        serde_json::from_str::<serde_json::Value>(data)
                    {
                        if let Some(text) = chunk.get("content").and_then(|c| c.as_str()) {
                            print!("{}", text);
                            io::stdout().flush()?;
                        }
                    }
                }
            }
        }
    }

    println!();
    Ok(())
}

/// Interactive REPL chat mode.
pub async fn interactive_repl(
    client: &reqwest::Client,
    api: &str,
    stream: bool,
    provider: Option<&str>,
) -> Result<()> {
    println!("{}", "╔══════════════════════════════════════╗".cyan());
    println!("{}", "║     NDE-OS Agent Chat (REPL)         ║".cyan());
    println!("{}", "║  Type 'exit' or Ctrl+C to quit       ║".cyan());
    println!("{}", "╚══════════════════════════════════════╝".cyan());

    let mut conversation_id: Option<String> = None;
    let stdin = io::stdin();

    loop {
        print!("\n{} ", "You>".green().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }
        if input == "exit" || input == "quit" || input == "/q" {
            println!("{}", "Goodbye!".dimmed());
            break;
        }
        if input == "/new" {
            conversation_id = None;
            println!("{}", "Started new conversation.".dimmed());
            continue;
        }
        if input == "/help" {
            println!("  {} — quit chat", "/q".bold());
            println!("  {} — new conversation", "/new".bold());
            println!("  {} — this help", "/help".bold());
            continue;
        }

        send_message(
            client,
            api,
            input,
            stream,
            conversation_id.as_deref(),
            provider,
        )
        .await?;
    }

    Ok(())
}
