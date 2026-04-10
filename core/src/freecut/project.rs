//! Project, timeline, track, and item models — mirrors FreeCut's TypeScript types
//! with full serde round-trip support.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Project ───────────────────────────────────────────────────────────────────

/// Top-level project metadata + timeline state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Total project duration in frames.
    pub duration: u32,
    /// Schema version for migrations.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub metadata: ProjectResolution,
    #[serde(default)]
    pub timeline: Option<ProjectTimeline>,
    #[serde(default)]
    pub dubbing: Option<DubbingSession>,
}

fn default_schema_version() -> u32 {
    1
}

fn default_source_language() -> String {
    "auto".to_string()
}

fn default_target_language() -> String {
    "en".to_string()
}

/// Canvas resolution + FPS.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResolution {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    /// Hex color, defaults to #000000.
    #[serde(default = "default_bg_color")]
    pub background_color: String,
}

fn default_bg_color() -> String {
    "#000000".to_string()
}

impl Default for ProjectResolution {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 30,
            background_color: default_bg_color(),
        }
    }
}

// ─── Timeline ──────────────────────────────────────────────────────────────────

/// Persisted timeline state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTimeline {
    pub tracks: Vec<Track>,
    pub items: Vec<TimelineItem>,
    #[serde(default)]
    pub current_frame: Option<u32>,
    #[serde(default)]
    pub zoom_level: Option<f64>,
    #[serde(default)]
    pub scroll_position: Option<f64>,
    #[serde(default)]
    pub in_point: Option<u32>,
    #[serde(default)]
    pub out_point: Option<u32>,
    #[serde(default)]
    pub markers: Vec<ProjectMarker>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
    #[serde(default)]
    pub compositions: Vec<SubComposition>,
    #[serde(default)]
    pub keyframes: Vec<ItemKeyframes>,
}

// ─── Track ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: Option<TrackKind>,
    #[serde(default = "default_track_height")]
    pub height: f64,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub volume: Option<f64>,
    #[serde(default)]
    pub color: Option<String>,
    pub order: i32,
    #[serde(default)]
    pub parent_track_id: Option<String>,
    #[serde(default)]
    pub is_group: bool,
    #[serde(default)]
    pub is_collapsed: bool,
}

fn default_track_height() -> f64 {
    48.0
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TrackKind {
    Video,
    Audio,
}

// ─── Timeline Item (discriminated union via `item_type`) ───────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineItem {
    pub id: String,
    pub track_id: String,
    /// Start frame (Remotion convention).
    pub from: u32,
    /// Duration in frames.
    pub duration_in_frames: u32,
    pub label: String,
    #[serde(rename = "type")]
    pub item_type: ItemType,

    // Common optional fields
    #[serde(default)]
    pub media_id: Option<String>,
    #[serde(default)]
    pub origin_id: Option<String>,
    #[serde(default)]
    pub linked_group_id: Option<String>,
    #[serde(default)]
    pub composition_id: Option<String>,

    // Trim properties
    #[serde(default)]
    pub trim_start: Option<u32>,
    #[serde(default)]
    pub trim_end: Option<u32>,
    #[serde(default)]
    pub source_start: Option<u32>,
    #[serde(default)]
    pub source_end: Option<u32>,
    #[serde(default)]
    pub source_duration: Option<u32>,
    #[serde(default)]
    pub source_fps: Option<f64>,
    #[serde(default)]
    pub speed: Option<f64>,

    // Transform
    #[serde(default)]
    pub transform: Option<TransformProperties>,

    // Audio
    #[serde(default)]
    pub volume: Option<f64>,
    #[serde(default)]
    pub audio_fade_in: Option<f64>,
    #[serde(default)]
    pub audio_fade_out: Option<f64>,

    // Video
    #[serde(default)]
    pub fade_in: Option<f64>,
    #[serde(default)]
    pub fade_out: Option<f64>,

    // Effects
    #[serde(default)]
    pub effects: Vec<ItemEffect>,
    #[serde(default)]
    pub blend_mode: Option<String>,
    #[serde(default)]
    pub keyframes: Vec<TimelineKeyframe>,

    // ── Type-specific fields (flattened for JSON compat with FreeCut) ──
    #[serde(default)]
    pub src: Option<String>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub source_width: Option<u32>,
    #[serde(default)]
    pub source_height: Option<u32>,
    #[serde(default)]
    pub waveform_data: Option<Vec<f32>>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub font_size: Option<f64>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub text_align: Option<String>,
    #[serde(default)]
    pub shape_type: Option<ShapeType>,
    #[serde(default)]
    pub fill_color: Option<String>,
    #[serde(default)]
    pub stroke_color: Option<String>,
    #[serde(default)]
    pub stroke_width: Option<f64>,
    #[serde(default)]
    pub composition_width: Option<u32>,
    #[serde(default)]
    pub composition_height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    Video,
    Audio,
    Text,
    Image,
    Shape,
    Adjustment,
    Composition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ShapeType {
    Rectangle,
    Circle,
    Triangle,
    Ellipse,
    Star,
    Polygon,
    Heart,
    Path,
}

// ─── Supporting types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformProperties {
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub scale: Option<f64>,
    #[serde(default)]
    pub rotation: Option<f64>,
    #[serde(default)]
    pub opacity: Option<f64>,
    #[serde(default)]
    pub corner_radius: Option<f64>,
    #[serde(default)]
    pub aspect_ratio_locked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemEffect {
    pub id: String,
    pub effect_type: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimelineKeyframe {
    pub frame_offset: u32,
    pub property: String, // "x", "y", "scale", "rotation", "opacity"
    pub value: f64,
    #[serde(default)]
    pub easing: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMarker {
    pub id: String,
    pub frame: u32,
    #[serde(default)]
    pub label: Option<String>,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub id: String,
    pub from_item_id: String,
    pub to_item_id: String,
    pub transition_type: String,
    pub duration_in_frames: u32,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubComposition {
    pub id: String,
    pub name: String,
    pub items: Vec<TimelineItem>,
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
    #[serde(default)]
    pub keyframes: Vec<ItemKeyframes>,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub duration_in_frames: u32,
    #[serde(default)]
    pub background_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemKeyframes {
    pub item_id: String,
    pub properties: Vec<KeyframeProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeProperty {
    pub property: String,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    pub id: String,
    pub frame: u32,
    pub value: f64,
    pub easing: String,
    #[serde(default)]
    pub easing_config: Option<serde_json::Value>,
}

// ─── Media metadata ────────────────────────────────────────────────────────────

/// FFprobe-extracted media info.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
    pub media_type: MediaType,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub duration_secs: Option<f64>,
    #[serde(default)]
    pub fps: Option<f64>,
    #[serde(default)]
    pub codec: Option<String>,
    #[serde(default)]
    pub audio_codec: Option<String>,
    #[serde(default)]
    pub sample_rate: Option<u32>,
    #[serde(default)]
    pub channels: Option<u32>,
    #[serde(default)]
    pub thumbnail_path: Option<String>,
    pub imported_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Video,
    Audio,
    Image,
}

// ─── Dubbing ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DubbingSession {
    #[serde(default)]
    pub source_media_id: Option<String>,
    #[serde(default)]
    pub source_media_path: Option<String>,
    #[serde(default = "default_source_language")]
    pub source_language: String,
    #[serde(default = "default_target_language")]
    pub target_language: String,
    #[serde(default)]
    pub ingest_mode: DubbingIngestMode,
    #[serde(default)]
    pub imported_srt_path: Option<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub segments: Vec<DubbingSegment>,
    #[serde(default)]
    pub speakers: Vec<DubbingSpeaker>,
    #[serde(default)]
    pub llm: Option<DubbingLlmConfig>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_generated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DubbingIngestMode {
    #[default]
    Srt,
    Whisper,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DubbingSegment {
    pub id: String,
    pub start_secs: f64,
    pub end_secs: f64,
    pub text: String,
    #[serde(default)]
    pub output_text: Option<String>,
    #[serde(default)]
    pub speaker_id: Option<String>,
    #[serde(default)]
    pub audio_path: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DubbingSpeaker {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub voice: String,
    #[serde(default)]
    pub rate: Option<String>,
    #[serde(default)]
    pub pitch: Option<String>,
    #[serde(default)]
    pub volume: Option<String>,
    #[serde(default)]
    pub rvc: Option<DubbingRvcConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DubbingRvcConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub python_path: Option<String>,
    #[serde(default)]
    pub cli_path: Option<String>,
    #[serde(default)]
    pub model_path: Option<String>,
    #[serde(default)]
    pub index_path: Option<String>,
    #[serde(default)]
    pub pitch_shift: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DubbingLlmConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
}

// ─── Export config ──────────────────────────────────────────────────────────────

/// Export job configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportConfig {
    pub output_path: String,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    #[serde(default = "default_quality")]
    pub quality: String,
    #[serde(default)]
    pub hw_accel: Option<String>,
}

fn default_quality() -> String {
    "high".to_string()
}

// ─── Event payloads (Tauri emit) ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameRenderedEvent {
    pub frame: u32,
    /// Path to the rendered frame image (PNG/BMP in temp dir).
    pub bitmap_path: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgressEvent {
    pub percent: f64,
    pub current_frame: u32,
    pub total_frames: u32,
    pub eta_secs: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaImportedEvent {
    pub media: MediaMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailsReadyEvent {
    pub media_id: String,
    pub thumbnail_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformReadyEvent {
    pub media_id: String,
    pub peaks: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_roundtrip() {
        let project = Project {
            id: "test-1".to_string(),
            name: "Test Project".to_string(),
            description: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            duration: 300,
            schema_version: 1,
            metadata: ProjectResolution::default(),
            timeline: None,
            dubbing: None,
        };

        let json = serde_json::to_string(&project).expect("serialize");
        let back: Project = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, "test-1");
        assert_eq!(back.metadata.width, 1920);
        assert_eq!(back.metadata.fps, 30);
        assert!(back.dubbing.is_none());
    }

    #[test]
    fn timeline_item_video_roundtrip() {
        let item = TimelineItem {
            id: "item-1".to_string(),
            track_id: "track-1".to_string(),
            from: 0,
            duration_in_frames: 60,
            label: "clip.mp4".to_string(),
            item_type: ItemType::Video,
            media_id: Some("media-1".to_string()),
            origin_id: None,
            linked_group_id: None,
            composition_id: None,
            trim_start: None,
            trim_end: None,
            source_start: Some(0),
            source_end: Some(300),
            source_duration: Some(300),
            source_fps: Some(30.0),
            speed: Some(1.0),
            transform: None,
            volume: Some(0.0),
            audio_fade_in: None,
            audio_fade_out: None,
            fade_in: None,
            fade_out: None,
            effects: vec![],
            blend_mode: None,
            src: Some("/media/clip.mp4".to_string()),
            thumbnail_url: None,
            source_width: Some(1920),
            source_height: Some(1080),
            waveform_data: None,
            text: None,
            font_size: None,
            font_family: None,
            color: None,
            text_align: None,
            shape_type: None,
            fill_color: None,
            stroke_color: None,
            stroke_width: None,
            composition_width: None,
            composition_height: None,
        };

        let json = serde_json::to_string(&item).expect("serialize");
        let back: TimelineItem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.item_type, ItemType::Video);
        assert_eq!(back.source_width, Some(1920));
    }

    #[test]
    fn media_metadata_roundtrip() {
        let meta = MediaMetadata {
            id: "m-1".to_string(),
            file_name: "test.mp4".to_string(),
            file_path: "/tmp/test.mp4".to_string(),
            file_size: 1024 * 1024,
            media_type: MediaType::Video,
            width: Some(1920),
            height: Some(1080),
            duration_secs: Some(10.5),
            fps: Some(30.0),
            codec: Some("h264".to_string()),
            audio_codec: Some("aac".to_string()),
            sample_rate: Some(44100),
            channels: Some(2),
            thumbnail_path: None,
            imported_at: Utc::now(),
        };

        let json = serde_json::to_string(&meta).expect("serialize");
        let back: MediaMetadata = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.media_type, MediaType::Video);
        assert_eq!(back.width, Some(1920));
    }

    #[test]
    fn dubbing_roundtrip() {
        let project = Project {
            id: "dub-1".to_string(),
            name: "Dub Project".to_string(),
            description: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            duration: 120,
            schema_version: 1,
            metadata: ProjectResolution::default(),
            timeline: None,
            dubbing: Some(DubbingSession {
                source_media_id: Some("media-1".to_string()),
                source_media_path: Some("C:/movie.mp4".to_string()),
                source_language: "ja".to_string(),
                target_language: "en".to_string(),
                ingest_mode: DubbingIngestMode::Srt,
                imported_srt_path: Some("C:/movie.srt".to_string()),
                output_dir: Some("C:/render/dubbing".to_string()),
                notes: None,
                segments: vec![DubbingSegment {
                    id: "seg-1".to_string(),
                    start_secs: 0.0,
                    end_secs: 1.5,
                    text: "hello".to_string(),
                    output_text: Some("hi".to_string()),
                    speaker_id: Some("speaker-1".to_string()),
                    audio_path: Some("C:/render/dubbing/seg-1.mp3".to_string()),
                    status: Some("ready".to_string()),
                }],
                speakers: vec![DubbingSpeaker {
                    id: "speaker-1".to_string(),
                    label: "Speaker 1".to_string(),
                    voice: "en-US-AriaNeural".to_string(),
                    rate: Some("+0%".to_string()),
                    pitch: None,
                    volume: None,
                    rvc: None,
                }],
                llm: Some(DubbingLlmConfig {
                    enabled: true,
                    model: Some("llama3".to_string()),
                    mode: Some("translate".to_string()),
                }),
                updated_at: Some(Utc::now()),
                last_generated_at: None,
            }),
        };

        let json = serde_json::to_string(&project).expect("serialize");
        let back: Project = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.dubbing.unwrap().segments.len(), 1);
    }
}
