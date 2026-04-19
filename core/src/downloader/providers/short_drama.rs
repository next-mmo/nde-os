//! QuickPlay short-drama provider (https://m.quickplay.my.id).
//!
//! The public proxy exposes:
//!   - GET /api/proxy/detail?id={id}&category_p={cat}&lang={lang}
//!   - GET /api/proxy/video?id={id}&chapterId={cid}&category_p={cat}&lang={lang}
//!
//! Accepts URLs of the form:
//!   https://m.quickplay.my.id/drama/{id}?category={cat}&lang={lang}
//!   https://quickplay.my.id/drama/{id}
//! Category + lang default to `reelshort` / `en` when omitted.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

use crate::downloader::provider::{
    MediaProvider, Playlist, PlaylistItem, StreamInfo, StreamKind,
};

const API_BASE: &str = "https://m.quickplay.my.id/api/proxy";
const DEFAULT_CATEGORY: &str = "reelshort";
const DEFAULT_LANG: &str = "en";

pub struct ShortDramaProvider {
    client: reqwest::Client,
}

impl ShortDramaProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(concat!(
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15 ",
                    "(KHTML, like Gecko) Version/17.0 Safari/605.1.15"
                ))
                .build()
                .expect("reqwest client"),
        }
    }
}

impl Default for ShortDramaProvider {
    fn default() -> Self {
        Self::new()
    }
}

// ─── URL parsing ───────────────────────────────────────────────────────────

/// Parsed QuickPlay URL: drama id + optional category/lang.
#[derive(Debug, Clone)]
struct ParsedUrl {
    id: String,
    category: String,
    lang: String,
}

fn parse_url(url: &str) -> Option<ParsedUrl> {
    // Strip scheme so the first segment is the host.
    let without_scheme = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let (path_part, query_part) = match without_scheme.split_once('?') {
        Some((p, q)) => (p, q),
        None => (without_scheme, ""),
    };

    let mut segments = path_part.split('/');
    let host = segments.next()?;
    if !host.ends_with("quickplay.my.id") {
        return None;
    }

    // First path segment must be "drama", second is the id.
    let first = segments.next()?;
    if first != "drama" {
        return None;
    }
    let id = segments.next()?.trim();
    if id.is_empty() {
        return None;
    }

    let mut category = DEFAULT_CATEGORY.to_string();
    let mut lang = DEFAULT_LANG.to_string();
    for pair in query_part.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.split_once('=') {
            Some(kv) => kv,
            None => continue,
        };
        let decoded = urlencoding::decode(v).map(|s| s.to_string()).unwrap_or_else(|_| v.to_string());
        match k {
            "category" | "category_p" => category = decoded,
            "lang" => lang = decoded,
            _ => {}
        }
    }

    Some(ParsedUrl {
        id: id.to_string(),
        category,
        lang,
    })
}

// ─── API response shapes ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    success: bool,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<String>,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct DetailData {
    id: String,
    title: String,
    #[serde(default)]
    cover: Option<String>,
    #[serde(default)]
    synopsis: Option<String>,
    #[serde(default)]
    chapters: Vec<ChapterEntry>,
}

#[derive(Debug, Deserialize)]
struct ChapterEntry {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    episode: u32,
    #[serde(default)]
    index: u32,
}

#[derive(Debug, Deserialize)]
struct VideoData {
    #[serde(default)]
    streams: Vec<VideoStream>,
}

#[derive(Debug, Clone, Deserialize)]
struct VideoStream {
    url: String,
    #[serde(default)]
    quality: Option<String>,
}

// ─── Provider impl ─────────────────────────────────────────────────────────

#[async_trait]
impl MediaProvider for ShortDramaProvider {
    fn id(&self) -> &'static str {
        "short_drama"
    }

    fn display_name(&self) -> &'static str {
        "QuickPlay Short Drama"
    }

    fn matches(&self, url: &str) -> bool {
        parse_url(url).is_some()
    }

    async fn resolve_playlist(&self, url: &str) -> Result<Playlist> {
        let parsed = parse_url(url).ok_or_else(|| anyhow!("Not a QuickPlay drama URL: {}", url))?;

        let detail_url = format!(
            "{}/detail?id={}&category_p={}&lang={}",
            API_BASE,
            urlencoding::encode(&parsed.id),
            urlencoding::encode(&parsed.category),
            urlencoding::encode(&parsed.lang),
        );

        let envelope: Envelope<DetailData> = self
            .client
            .get(&detail_url)
            .send()
            .await
            .with_context(|| format!("GET {}", detail_url))?
            .error_for_status()
            .with_context(|| format!("upstream error for {}", detail_url))?
            .json()
            .await
            .with_context(|| "decode /detail response")?;

        if !envelope.success {
            let msg = envelope
                .message
                .or(envelope.error)
                .unwrap_or_else(|| "unknown upstream error".to_string());
            return Err(anyhow!("QuickPlay /detail failed: {}", msg));
        }
        let data = envelope
            .data
            .ok_or_else(|| anyhow!("QuickPlay /detail returned no data"))?;

        // Sort by display index and renumber to 1..=N so filenames are clean.
        let mut chapters = data.chapters;
        chapters.sort_by_key(|c| (c.index, c.episode));
        let items: Vec<PlaylistItem> = chapters
            .into_iter()
            .enumerate()
            .map(|(i, c)| PlaylistItem {
                id: c.id,
                title: if c.name.is_empty() {
                    format!("Episode {}", i + 1)
                } else {
                    c.name
                },
                index: (i + 1) as u32,
            })
            .collect();

        Ok(Playlist {
            provider: self.id().to_string(),
            source_url: url.to_string(),
            playlist_id: data.id,
            title: data.title,
            cover: data.cover,
            synopsis: data.synopsis,
            items,
            context: serde_json::json!({
                "category": parsed.category,
                "lang": parsed.lang,
            }),
        })
    }

    async fn resolve_stream(&self, playlist: &Playlist, item_id: &str) -> Result<StreamInfo> {
        let category = playlist
            .context
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_CATEGORY);
        let lang = playlist
            .context
            .get("lang")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_LANG);

        let video_url = format!(
            "{}/video?id={}&chapterId={}&category_p={}&lang={}",
            API_BASE,
            urlencoding::encode(&playlist.playlist_id),
            urlencoding::encode(item_id),
            urlencoding::encode(category),
            urlencoding::encode(lang),
        );

        let envelope: Envelope<VideoData> = self
            .client
            .get(&video_url)
            .send()
            .await
            .with_context(|| format!("GET {}", video_url))?
            .error_for_status()
            .with_context(|| format!("upstream error for {}", video_url))?
            .json()
            .await
            .with_context(|| "decode /video response")?;

        if !envelope.success {
            let msg = envelope
                .message
                .or(envelope.error)
                .unwrap_or_else(|| "unknown upstream error".to_string());
            return Err(anyhow!("QuickPlay /video failed: {}", msg));
        }
        let data = envelope
            .data
            .ok_or_else(|| anyhow!("QuickPlay /video returned no data"))?;
        if data.streams.is_empty() {
            return Err(anyhow!("QuickPlay /video returned no streams"));
        }

        // Prefer the first variant whose URL looks like an HLS playlist
        // (upstream returns multiple fallbacks, all 720p).
        let pick = data
            .streams
            .iter()
            .find(|s| s.url.contains(".m3u8"))
            .cloned()
            .unwrap_or_else(|| data.streams[0].clone());

        Ok(StreamInfo {
            url: pick.url,
            kind: StreamKind::Hls,
            quality: pick.quality,
            headers: vec![
                (
                    "Referer".to_string(),
                    "https://m.quickplay.my.id/".to_string(),
                ),
                (
                    "Origin".to_string(),
                    "https://m.quickplay.my.id".to_string(),
                ),
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_url() {
        let p = parse_url(
            "https://m.quickplay.my.id/drama/69c23f5368325d2264072f05?category=reelshort&lang=en",
        )
        .unwrap();
        assert_eq!(p.id, "69c23f5368325d2264072f05");
        assert_eq!(p.category, "reelshort");
        assert_eq!(p.lang, "en");
    }

    #[test]
    fn parses_url_without_query() {
        let p = parse_url("https://quickplay.my.id/drama/abc123").unwrap();
        assert_eq!(p.id, "abc123");
        assert_eq!(p.category, "reelshort");
        assert_eq!(p.lang, "en");
    }

    #[test]
    fn parses_url_with_trailing_slash_and_extra_params() {
        let p = parse_url(
            "https://m.quickplay.my.id/drama/xyz?category=dramabox&lang=id&foo=bar",
        )
        .unwrap();
        assert_eq!(p.id, "xyz");
        assert_eq!(p.category, "dramabox");
        assert_eq!(p.lang, "id");
    }

    #[test]
    fn rejects_non_quickplay_urls() {
        assert!(parse_url("https://example.com/drama/abc").is_none());
        assert!(parse_url("https://m.quickplay.my.id/").is_none());
        assert!(parse_url("https://m.quickplay.my.id/drama/").is_none());
    }

    #[test]
    fn matches_uses_parse_url() {
        let p = ShortDramaProvider::new();
        assert!(p.matches("https://m.quickplay.my.id/drama/69c23f5368325d2264072f05"));
        assert!(p.matches("https://quickplay.my.id/drama/abc"));
        assert!(!p.matches("https://youtube.com/watch?v=abc"));
    }
}
