//! Web browsing tool — fetches a URL and extracts clean readable text.
//!
//! Strips HTML tags, extracts title/headings/body text/links.
//! Designed for the agent to read web pages and extract information.

use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;

pub struct WebBrowseTool;

#[async_trait]
impl Tool for WebBrowseTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "web_browse".into(),
            description: "Browse a web page and extract its readable text content. Returns the page title, main text, headings, and links. Use this to read articles, documentation, or any web page.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to browse"
                    },
                    "extract": {
                        "type": "string",
                        "enum": ["text", "links", "all"],
                        "description": "What to extract: 'text' (readable content), 'links' (all hyperlinks), 'all' (both). Default: 'all'",
                        "default": "all"
                    },
                    "max_length": {
                        "type": "integer",
                        "description": "Maximum characters to return (default: 30000)",
                        "default": 30000
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

        let extract = args
            .get("extract")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        let max_length = args
            .get("max_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(30000) as usize;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch {}: {}", url, e))?;

        let status = response.status();
        if !status.is_success() {
            return Ok(format!("HTTP Error: {} for {}", status, url));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let html = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response: {}", e))?;

        // If not HTML, return raw text
        if !content_type.contains("html") {
            let truncated = truncate_text(&html, max_length);
            return Ok(format!(
                "URL: {}\nContent-Type: {}\n\n{}",
                url, content_type, truncated
            ));
        }

        let mut output = format!("URL: {}\n", url);

        // Extract title
        if let Some(title) = extract_tag_content(&html, "title") {
            output.push_str(&format!("Title: {}\n", clean_text(&title)));
        }

        // Extract meta description
        if let Some(desc) = extract_meta_description(&html) {
            output.push_str(&format!("Description: {}\n", desc));
        }

        output.push_str("\n");

        match extract {
            "text" => {
                let text = extract_readable_text(&html);
                output.push_str(&truncate_text(&text, max_length));
            }
            "links" => {
                let links = extract_links(&html, url);
                for (text, href) in links.iter().take(100) {
                    output.push_str(&format!("- [{}]({})\n", text, href));
                }
            }
            _ => {
                // All
                let text = extract_readable_text(&html);
                let text_section = truncate_text(&text, max_length - 2000);
                output.push_str("## Content\n\n");
                output.push_str(&text_section);

                let links = extract_links(&html, url);
                if !links.is_empty() {
                    output.push_str("\n\n## Links\n\n");
                    for (text, href) in links.iter().take(50) {
                        output.push_str(&format!("- [{}]({})\n", text, href));
                    }
                }
            }
        }

        Ok(truncate_text(&output, max_length))
    }
}

/// Extract content of a specific HTML tag.
fn extract_tag_content(html: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);

    let start = html.to_lowercase().find(&open)?;
    let after_open = html[start..].find('>')? + start + 1;
    let end = html[after_open..].to_lowercase().find(&close)? + after_open;

    Some(html[after_open..end].to_string())
}

/// Extract meta description.
fn extract_meta_description(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let idx = lower.find("name=\"description\"")?;

    // Search nearby for content="..."
    let region_start = if idx > 200 { idx - 200 } else { 0 };
    let region_end = std::cmp::min(idx + 500, html.len());
    let region = &html[region_start..region_end];

    let content_idx = region.to_lowercase().find("content=\"")?;
    let start = content_idx + 9;
    let end = region[start..].find('"')? + start;

    Some(region[start..end].to_string())
}

/// Extract readable text from HTML.
fn extract_readable_text(html: &str) -> String {
    let mut text = html.to_string();

    // Remove script and style tags and their content
    text = remove_tag_block(&text, "script");
    text = remove_tag_block(&text, "style");
    text = remove_tag_block(&text, "nav");
    text = remove_tag_block(&text, "footer");
    text = remove_tag_block(&text, "header");

    // Convert block elements to newlines
    let block_tags = [
        "p",
        "div",
        "br",
        "li",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "tr",
        "blockquote",
        "pre",
        "section",
        "article",
    ];
    for tag in &block_tags {
        let patterns = [
            format!("<{}>", tag),
            format!("<{} ", tag),
            format!("</{}>", tag),
        ];
        for pat in &patterns {
            text = text.replace(pat, "\n");
            text = text.replace(&pat.to_uppercase(), "\n");
        }
    }

    // Convert heading tags to markdown-style
    for i in 1..=6 {
        let prefix = "#".repeat(i);
        let tag = format!("h{}", i);
        if let Some(content) = extract_tag_content(&text, &tag) {
            let _clean = clean_text(&strip_tags(&content));
            text = text.replacen(&format!("<{}>", tag), &format!("\n{} ", prefix), 1);
        }
    }

    // Strip all remaining tags
    text = strip_tags(&text);

    // Decode common HTML entities
    text = decode_entities(&text);

    // Clean up whitespace
    clean_text(&text)
}

/// Remove a tag and all its content.
fn remove_tag_block(html: &str, tag: &str) -> String {
    let mut result = html.to_string();
    let lower_tag = tag.to_lowercase();

    loop {
        let lower = result.to_lowercase();
        let open = format!("<{}", lower_tag);
        let close = format!("</{}>", lower_tag);

        if let Some(start) = lower.find(&open) {
            if let Some(end_offset) = lower[start..].find(&close) {
                let end = start + end_offset + close.len();
                result = format!("{}{}", &result[..start], &result[end..]);
                continue;
            }
        }
        break;
    }

    result
}

/// Strip all HTML tags.
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

/// Decode common HTML entities.
fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "...")
}

/// Clean up whitespace in text.
fn clean_text(text: &str) -> String {
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

/// Extract links from HTML.
fn extract_links(html: &str, base_url: &str) -> Vec<(String, String)> {
    let mut links = Vec::new();
    let lower = html.to_lowercase();
    let mut search_from = 0;

    while let Some(a_start) = lower[search_from..].find("<a ") {
        let abs_start = search_from + a_start;
        let tag_end = match lower[abs_start..].find('>') {
            Some(e) => abs_start + e,
            None => break,
        };

        let tag = &html[abs_start..=tag_end];

        // Extract href
        if let Some(href) = extract_attribute(tag, "href") {
            // Find closing </a>
            let content_start = tag_end + 1;
            let content_end = lower[content_start..]
                .find("</a>")
                .map(|e| content_start + e)
                .unwrap_or(content_start);

            let link_text = clean_text(&strip_tags(&html[content_start..content_end]));

            // Resolve relative URLs
            let full_url = if href.starts_with("http://") || href.starts_with("https://") {
                href
            } else if href.starts_with('/') {
                // Get base domain
                if let Some(domain_end) = base_url.find("//").map(|i| {
                    base_url[i + 2..]
                        .find('/')
                        .map(|j| i + 2 + j)
                        .unwrap_or(base_url.len())
                }) {
                    format!("{}{}", &base_url[..domain_end], href)
                } else {
                    href
                }
            } else {
                href
            };

            if !link_text.is_empty()
                && !full_url.starts_with('#')
                && !full_url.starts_with("javascript:")
            {
                links.push((link_text, full_url));
            }
        }

        search_from = tag_end + 1;
    }

    links
}

/// Extract an attribute value from an HTML tag string.
fn extract_attribute(tag: &str, attr: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let search = format!("{}=\"", attr);
    if let Some(start) = lower.find(&search) {
        let value_start = start + search.len();
        if let Some(end) = tag[value_start..].find('"') {
            return Some(tag[value_start..value_start + end].to_string());
        }
    }

    let search_single = format!("{}='", attr);
    if let Some(start) = lower.find(&search_single) {
        let value_start = start + search_single.len();
        if let Some(end) = tag[value_start..].find('\'') {
            return Some(tag[value_start..value_start + end].to_string());
        }
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_tags() {
        assert_eq!(strip_tags("<p>Hello <b>world</b></p>"), "Hello world");
    }

    #[test]
    fn test_decode_entities() {
        assert_eq!(decode_entities("&amp; &lt; &gt;"), "& < >");
    }

    #[test]
    fn test_extract_tag_content() {
        let html = "<title>My Page</title>";
        assert_eq!(
            extract_tag_content(html, "title"),
            Some("My Page".to_string())
        );
    }

    #[test]
    fn test_remove_tag_block() {
        let html = "before<script>alert('xss')</script>after";
        assert_eq!(remove_tag_block(html, "script"), "beforeafter");
    }

    #[test]
    fn test_extract_attribute() {
        let tag = r#"<a href="https://example.com" class="link">"#;
        assert_eq!(
            extract_attribute(tag, "href"),
            Some("https://example.com".to_string())
        );
    }

    #[test]
    fn test_extract_links() {
        let html = r#"<a href="https://example.com">Example</a> <a href="/about">About</a>"#;
        let links = extract_links(html, "https://mysite.com");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].0, "Example");
        assert_eq!(links[0].1, "https://example.com");
    }
}
