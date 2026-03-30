use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;

/// Makes HTTP requests from within the sandbox (GET/POST).
pub struct HttpFetchTool;

#[async_trait]
impl Tool for HttpFetchTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "http_fetch".into(),
            description: "Make an HTTP request (GET or POST). Returns status code and response body. Useful for calling APIs, downloading data, or checking service health.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to request"
                    },
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "DELETE"],
                        "description": "HTTP method (default: GET)",
                        "default": "GET"
                    },
                    "body": {
                        "type": "string",
                        "description": "Request body (for POST/PUT)"
                    },
                    "headers": {
                        "type": "object",
                        "description": "Additional headers as key-value pairs"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Request timeout in seconds (default: 30)",
                        "default": 30
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, _sandbox: &Sandbox) -> Result<String> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument"))?;

        let method = args
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let timeout_secs = args
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        // Block requests to internal/private networks for security
        if is_private_url(url) {
            return Err(anyhow::anyhow!(
                "Blocked: cannot access private/internal network addresses from sandbox"
            ));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()?;

        let mut request = match method.as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            _ => return Err(anyhow::anyhow!("Unsupported method: {}", method)),
        };

        // Add custom headers
        if let Some(headers) = args.get("headers").and_then(|h| h.as_object()) {
            for (key, value) in headers {
                if let Some(val) = value.as_str() {
                    request = request.header(key.as_str(), val);
                }
            }
        }

        // Add body
        if let Some(body) = args.get("body").and_then(|b| b.as_str()) {
            request = request.body(body.to_string());
        }

        let response = request
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

        let status = response.status();
        let headers_str: String = response
            .headers()
            .iter()
            .take(10) // Limit header output
            .map(|(k, v)| format!("  {}: {}", k, v.to_str().unwrap_or("?")))
            .collect::<Vec<_>>()
            .join("\n");

        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "[binary or unreadable response]".into());

        // Truncate large responses
        let body_display = if body.len() > 50_000 {
            format!(
                "{}...\n\n[truncated, {} total bytes]",
                &body[..50_000],
                body.len()
            )
        } else {
            body
        };

        Ok(format!(
            "HTTP {} {}\nStatus: {}\nHeaders:\n{}\n\nBody:\n{}",
            method, url, status, headers_str, body_display
        ))
    }
}

/// Check if a URL targets a private/internal network.
fn is_private_url(url: &str) -> bool {
    let lower = url.to_lowercase();

    // Allow localhost for local AI services (Ollama, SD, etc.)
    // This is an NDE-OS specific decision — apps run on localhost
    if lower.contains("localhost") || lower.contains("127.0.0.1") {
        return false;
    }

    // Block other private ranges
    let blocked_prefixes = [
        "http://10.",
        "https://10.",
        "http://172.16.",
        "https://172.16.",
        "http://172.17.",
        "https://172.17.",
        "http://172.18.",
        "https://172.18.",
        "http://172.19.",
        "https://172.19.",
        "http://172.20.",
        "https://172.20.",
        "http://172.21.",
        "https://172.21.",
        "http://172.22.",
        "https://172.22.",
        "http://172.23.",
        "https://172.23.",
        "http://172.24.",
        "https://172.24.",
        "http://172.25.",
        "https://172.25.",
        "http://172.26.",
        "https://172.26.",
        "http://172.27.",
        "https://172.27.",
        "http://172.28.",
        "https://172.28.",
        "http://172.29.",
        "https://172.29.",
        "http://172.30.",
        "https://172.30.",
        "http://172.31.",
        "https://172.31.",
        "http://192.168.",
        "https://192.168.",
        "http://0.0.0.0",
        "https://0.0.0.0",
    ];

    blocked_prefixes.iter().any(|p| lower.starts_with(p))
}
