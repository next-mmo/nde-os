//! Shield Browser browsing tool — uses anti-detect headless browser via CDP.
//!
//! Unlike `web_browse` which uses plain HTTP requests, this tool launches
//! a real headless Shield Browser (Camoufox/Wayfern) with fingerprint
//! spoofing. This bypasses bot detection, renders JavaScript, and provides
//! fully rendered page content.
//!
//! Use cases:
//! - Sites that block simple HTTP scrapers
//! - JS-rendered content (SPAs, dynamic news feeds)
//! - Anti-detect browsing for research

use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::shield::browser::BrowserEngine;
use crate::shield::cdp::CdpClient;
use crate::shield::engine::EngineManager;
use crate::shield::launcher::BrowserLauncher;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;

pub struct ShieldBrowseTool;

#[async_trait]
impl Tool for ShieldBrowseTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "shield_browse".into(),
            description: "Browse a web page using the Shield Browser (anti-detect headless browser with fingerprint spoofing). \
                Unlike web_browse which uses simple HTTP requests, this launches a real browser that: \
                (1) renders JavaScript fully, (2) bypasses bot detection and CAPTCHAs, \
                (3) spoofs browser fingerprints at the C++ level. \
                Use this for sites that block scrapers, require JS rendering, or have anti-bot protection. \
                Note: slower than web_browse (~10-15s), use only when regular browsing fails.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to browse with the shielded browser"
                    },
                    "extract": {
                        "type": "string",
                        "enum": ["text", "links", "all"],
                        "description": "What to extract: 'text' (page body text), 'links' (all hyperlinks), 'all' (both). Default: 'all'",
                        "default": "all"
                    },
                    "wait_secs": {
                        "type": "integer",
                        "description": "Extra seconds to wait after page load for JS to render (default: 2)",
                        "default": 2
                    },
                    "max_length": {
                        "type": "integer",
                        "description": "Maximum characters to return (default: 30000)",
                        "default": 30000
                    },
                    "js_expression": {
                        "type": "string",
                        "description": "Optional: custom JavaScript expression to evaluate on the page and return its result"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument"))?;

        let extract = args
            .get("extract")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        let wait_secs = args
            .get("wait_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(2);

        let max_length = args
            .get("max_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(30000) as usize;

        let js_expression = args
            .get("js_expression")
            .and_then(|v| v.as_str());

        // Determine the data directory from sandbox workspace
        let data_dir = sandbox.root();

        // Find a downloaded engine (prefer Camoufox for anti-detect)
        let engine_mgr = EngineManager::new(data_dir);
        let (engine, version) = find_available_engine(&engine_mgr)?;

        tracing::info!(
            "shield_browse: using {} v{} for URL: {}",
            engine.display_name(),
            version,
            url
        );

        // Launch headless browser
        let launcher = BrowserLauncher::new(data_dir);
        let session = launcher
            .launch_headless(&engine, &version, None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to launch headless browser: {}", e))?;

        // Connect CDP — ensure cleanup on error
        let result = browse_with_cdp(&session, url, extract, wait_secs, max_length, js_expression).await;

        // Always clean up
        session.shutdown().await;

        result
    }
}

/// The actual browsing logic via CDP, separated for clean error handling.
async fn browse_with_cdp(
    session: &crate::shield::launcher::HeadlessSession,
    url: &str,
    extract: &str,
    extra_wait_secs: u64,
    max_length: usize,
    js_expression: Option<&str>,
) -> Result<String> {
    // Connect to the browser via CDP
    let cdp = CdpClient::connect(session.cdp_port).await?;

    // Navigate and wait for load
    let timeout = std::time::Duration::from_secs(30);
    cdp.navigate_and_wait(url, timeout).await?;

    // Extra wait for JS content to settle
    if extra_wait_secs > 0 {
        tokio::time::sleep(std::time::Duration::from_secs(extra_wait_secs)).await;
    }

    let mut output = String::new();

    // Get page metadata
    let title = cdp.get_title().await.unwrap_or_default();
    let current_url = cdp.get_url().await.unwrap_or_else(|_| url.to_string());

    output.push_str(&format!("URL: {}\n", current_url));
    if !title.is_empty() {
        output.push_str(&format!("Title: {}\n", title));
    }
    output.push_str("Browser: Shield (anti-detect, JS-rendered)\n\n");

    // If custom JS expression is provided, execute it
    if let Some(expr) = js_expression {
        let custom_result = cdp.evaluate(expr).await.unwrap_or_else(|e| format!("Error: {}", e));
        output.push_str("## Custom JS Result\n\n");
        output.push_str(&custom_result);
        return Ok(truncate_text(&output, max_length));
    }

    match extract {
        "text" => {
            let text = cdp.get_body_text().await?;
            output.push_str(&clean_body_text(&text));
        }
        "links" => {
            let links_json = cdp.get_links().await?;
            if let Ok(links) = serde_json::from_str::<Vec<serde_json::Value>>(&links_json) {
                for link in links.iter().take(100) {
                    let text = link.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let href = link.get("href").and_then(|v| v.as_str()).unwrap_or("");
                    if !text.is_empty() && !href.is_empty() {
                        output.push_str(&format!("- [{}]({})\n", text, href));
                    }
                }
            }
        }
        _ => {
            // "all" — text + links
            let text = cdp.get_body_text().await?;
            let cleaned = clean_body_text(&text);
            let text_budget = if max_length > 2000 { max_length - 2000 } else { max_length };

            output.push_str("## Content\n\n");
            output.push_str(&truncate_text(&cleaned, text_budget));

            let links_json = cdp.get_links().await?;
            if let Ok(links) = serde_json::from_str::<Vec<serde_json::Value>>(&links_json) {
                if !links.is_empty() {
                    output.push_str("\n\n## Links\n\n");
                    for link in links.iter().take(50) {
                        let link_text = link.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let href = link.get("href").and_then(|v| v.as_str()).unwrap_or("");
                        if !link_text.is_empty() && !href.is_empty() {
                            output.push_str(&format!("- [{}]({})\n", link_text, href));
                        }
                    }
                }
            }
        }
    }

    Ok(truncate_text(&output, max_length))
}

/// Find the first available (downloaded) browser engine.
/// Prefers Camoufox for better anti-detect, falls back to Wayfern.
fn find_available_engine(engine_mgr: &EngineManager) -> Result<(BrowserEngine, String)> {
    let downloaded = engine_mgr.list_downloaded()?;

    // Prefer Camoufox (Firefox-based, better anti-detect)
    if let Some((engine, version)) = downloaded
        .iter()
        .find(|(e, _)| *e == BrowserEngine::Camoufox)
    {
        return Ok((engine.clone(), version.clone()));
    }

    // Fall back to Wayfern (Chromium-based)
    if let Some((engine, version)) = downloaded
        .iter()
        .find(|(e, _)| *e == BrowserEngine::Wayfern)
    {
        return Ok((engine.clone(), version.clone()));
    }

    anyhow::bail!(
        "No Shield Browser engine is downloaded. \
         Install one from the Shield Browser desktop app first \
         (Camoufox recommended for anti-detect browsing)."
    )
}

/// Clean up raw body text from a browser.
fn clean_body_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();

    // Remove consecutive blank lines
    let mut result = String::new();
    let mut prev_blank = false;
    for line in &lines {
        if line.is_empty() {
            if !prev_blank {
                result.push('\n');
            }
            prev_blank = true;
        } else {
            result.push_str(line);
            result.push('\n');
            prev_blank = false;
        }
    }

    result.trim().to_string()
}

/// Truncate text to max length with indicator.
fn truncate_text(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        format!(
            "{}...\n\n[Truncated at {} chars, total: {}]",
            &text[..max],
            max,
            text.len()
        )
    }
}
