//! Provider trait + shared playlist/stream types.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// What kind of stream the source exposes. Controls the download strategy
/// (HLS remux via ffmpeg vs direct HTTP copy).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StreamKind {
    /// HLS playlist (`.m3u8`) — must be remuxed to MP4 with ffmpeg.
    Hls,
    /// Direct progressive download (MP4, WebM, …).
    Http,
}

/// A single downloadable item inside a playlist (episode, chapter, clip, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistItem {
    /// Provider-specific identifier (e.g. chapter id).
    pub id: String,
    /// Display title ("Episode 1").
    pub title: String,
    /// 1-based episode index inside the playlist.
    pub index: u32,
}

/// A resolved playlist — metadata + ordered list of items. Stream URLs are
/// fetched lazily via [`MediaProvider::resolve_stream`] so picking a single
/// episode does not hit N upstream endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    /// Provider id ("short_drama").
    pub provider: String,
    /// Raw source URL the user pasted.
    pub source_url: String,
    /// Provider-native id for the playlist (e.g. drama id).
    pub playlist_id: String,
    /// Display title of the whole playlist.
    pub title: String,
    /// Optional cover / poster URL.
    pub cover: Option<String>,
    /// Short synopsis, if available.
    pub synopsis: Option<String>,
    /// Items in playback order.
    pub items: Vec<PlaylistItem>,
    /// Free-form provider hints (category, language, …) echoed back when
    /// resolving individual streams.
    pub context: serde_json::Value,
}

/// A concrete stream URL + hints for a single item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub url: String,
    pub kind: StreamKind,
    pub quality: Option<String>,
    /// Headers to send when fetching the stream (Referer, User-Agent, …).
    #[serde(default)]
    pub headers: Vec<(String, String)>,
}

/// A provider implements URL matching + playlist/stream resolution for one
/// site (quickplay, dramabox, …). Add new providers by implementing this
/// trait and registering in [`providers::default_providers`].
#[async_trait]
pub trait MediaProvider: Send + Sync {
    /// Stable id used as folder name + in job records.
    fn id(&self) -> &'static str;

    /// Human-readable name shown in the UI.
    fn display_name(&self) -> &'static str;

    /// True if this provider can resolve `url`.
    fn matches(&self, url: &str) -> bool;

    /// Fetch the full playlist (metadata + items) without stream URLs.
    async fn resolve_playlist(&self, url: &str) -> Result<Playlist>;

    /// Resolve the stream URL for a single item.
    async fn resolve_stream(&self, playlist: &Playlist, item_id: &str) -> Result<StreamInfo>;
}
