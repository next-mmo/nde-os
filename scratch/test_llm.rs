use anyhow::Result;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();
    let body = json!({
        "model": "Qwen3.5-9B",
        "messages": [
            {"role": "user", "content": "Translate to Khmer: Hello"}
        ]
    });
    let res = client.post("http://127.0.0.1:8090/v1/chat/completions")
        .json(&body)
        .send()
        .await?;
    let text = res.text().await?;
    println!("Response: {}", text);
    Ok(())
}

