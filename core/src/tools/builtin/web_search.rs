//! Web search tool — searches the web via DuckDuckGo HTML (no API key).
//!
//! Extracts search results (title, URL, snippet) from DuckDuckGo's
//! HTML search page. Zero external API keys required.

use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "web_search".into(),
            description: "Search the web using DuckDuckGo. Returns a list of results with title, URL, and snippet. No API key required. Use this to find information, documentation, code examples, or answers to questions.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results to return (default: 10)",
                        "default": 10
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, _sandbox: &Sandbox) -> Result<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' argument"))?;

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let results = search_duckduckgo(query, max_results).await?;

        if results.is_empty() {
            return Ok(format!("No results found for: {}", query));
        }

        let mut output = format!("Search results for: {}\n\n", query);
        for (i, result) in results.iter().enumerate() {
            output.push_str(&format!(
                "{}. {}\n   {}\n   {}\n\n",
                i + 1,
                result.title,
                result.url,
                result.snippet
            ));
        }

        Ok(output)
    }
}

#[derive(Debug)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

async fn search_duckduckgo(query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
    let encoded_query = urlencoding::encode(query);
    let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Search request failed: {}", e))?;

    let html = response
        .text()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read search results: {}", e))?;

    let mut results = Vec::new();
    let lower = html.to_lowercase();

    // DuckDuckGo HTML uses class="result__a" for links and class="result__snippet" for snippets
    let mut search_pos = 0;

    while results.len() < max_results {
        // Find result link
        let link_marker = "class=\"result__a\"";
        let link_pos = match lower[search_pos..].find(link_marker) {
            Some(p) => search_pos + p,
            None => break,
        };

        // Extract href from the link
        let tag_start = html[..link_pos].rfind('<').unwrap_or(link_pos);
        let tag_end = match html[link_pos..].find('>') {
            Some(e) => link_pos + e,
            None => break,
        };

        let tag = &html[tag_start..=tag_end];
        let href = extract_href(tag).unwrap_or_default();

        // Extract link text (title)
        let title_start = tag_end + 1;
        let title_end = html[title_start..]
            .find("</a>")
            .map(|e| title_start + e)
            .unwrap_or(title_start);
        let title = strip_tags(&html[title_start..title_end]).trim().to_string();

        // Find snippet nearby
        let snippet_marker = "class=\"result__snippet\"";
        let snippet = if let Some(sp) = lower[tag_end..].find(snippet_marker) {
            let snippet_tag_end = html[tag_end + sp..].find('>').map(|e| tag_end + sp + e + 1);
            if let Some(s_start) = snippet_tag_end {
                let s_end = html[s_start..]
                    .find("</")
                    .map(|e| s_start + e)
                    .unwrap_or(s_start);
                strip_tags(&html[s_start..s_end]).trim().to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Resolve DuckDuckGo redirect URLs
        let resolved_url = resolve_ddg_url(&href);

        if !title.is_empty() && !resolved_url.is_empty() {
            results.push(SearchResult {
                title: decode_entities(&title),
                url: resolved_url,
                snippet: decode_entities(&snippet),
            });
        }

        search_pos = tag_end + 1;
    }

    // Fallback: try alternative parsing if no results found
    if results.is_empty() {
        results = parse_results_alternative(&html, max_results);
    }

    Ok(results)
}

/// Resolve DuckDuckGo redirect URL to the actual URL.
fn resolve_ddg_url(href: &str) -> String {
    if href.starts_with("//duckduckgo.com/l/?") || href.starts_with("https://duckduckgo.com/l/?") {
        // Extract uddg parameter
        if let Some(uddg_start) = href.find("uddg=") {
            let value_start = uddg_start + 5;
            let value_end = href[value_start..]
                .find('&')
                .map(|e| value_start + e)
                .unwrap_or(href.len());
            let encoded = &href[value_start..value_end];
            return urlencoding::decode(encoded)
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| encoded.to_string());
        }
    }

    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }

    if href.starts_with("//") {
        return format!("https:{}", href);
    }

    href.to_string()
}

/// Alternative result parser using different HTML patterns.
fn parse_results_alternative(html: &str, max_results: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let lower = html.to_lowercase();
    let mut pos = 0;

    // Try finding results by class="result" or class="web-result"
    while results.len() < max_results {
        let markers = ["class=\"result ", "class=\"web-result\""];
        let mut found = None;

        for marker in &markers {
            if let Some(p) = lower[pos..].find(marker) {
                found = Some(pos + p);
                break;
            }
        }

        let result_start = match found {
            Some(p) => p,
            None => break,
        };

        // Look for first <a href in this result block
        let block_end = std::cmp::min(result_start + 3000, html.len());
        let block = &html[result_start..block_end];

        if let Some(href_start) = block.to_lowercase().find("href=\"") {
            let val_start = href_start + 6;
            if let Some(val_end) = block[val_start..].find('"') {
                let href = &block[val_start..val_start + val_end];
                let url = resolve_ddg_url(href);

                // Extract text after the link
                if let Some(a_end) = block[val_start..].find('>') {
                    let text_start = val_start + a_end + 1;
                    let text_end = block[text_start..]
                        .find("</a>")
                        .map(|e| text_start + e)
                        .unwrap_or(text_start + 100);
                    let title =
                        strip_tags(&block[text_start..std::cmp::min(text_end, block.len())])
                            .trim()
                            .to_string();

                    if !title.is_empty() && url.starts_with("http") {
                        results.push(SearchResult {
                            title: decode_entities(&title),
                            url,
                            snippet: String::new(),
                        });
                    }
                }
            }
        }

        pos = result_start + 100;
    }

    results
}

fn extract_href(tag: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    if let Some(start) = lower.find("href=\"") {
        let val_start = start + 6;
        let val_end = tag[val_start..].find('"')?;
        return Some(tag[val_start..val_start + val_end].to_string());
    }
    if let Some(start) = lower.find("href='") {
        let val_start = start + 6;
        let val_end = tag[val_start..].find('\'')?;
        return Some(tag[val_start..val_start + val_end].to_string());
    }
    None
}

fn strip_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_ddg_url() {
        let url = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com&rut=abc";
        assert_eq!(resolve_ddg_url(url), "https://example.com");

        assert_eq!(
            resolve_ddg_url("https://example.com"),
            "https://example.com"
        );
    }

    #[test]
    fn test_strip_tags() {
        assert_eq!(strip_tags("<b>bold</b> text"), "bold text");
    }
}
