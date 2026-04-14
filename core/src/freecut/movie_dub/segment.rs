//! Segment types for the movie dubbing pipeline.
//!
//! A `Segment` is a speech region extracted from source media.
//! A `TimedText` is a segment after translation with timing metadata.

use serde::{Deserialize, Serialize};

use super::lang::Lang;

/// A speech segment extracted from source video/audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_text: String,
    pub source_lang: Lang,
    pub speaker_id: Option<u32>,
}

impl Segment {
    /// Duration of this segment in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }

    /// Duration of this segment in seconds.
    pub fn duration_secs(&self) -> f32 {
        self.duration_ms() as f32 / 1000.0
    }
}

/// A segment after translation with timing metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimedText {
    pub segment: Segment,
    pub translated_text: String,
    pub target_lang: Lang,
    pub estimated_duration_ms: u64,
    pub syllable_count: Option<u32>,
    pub provider_used: String,
    pub stretch_ratio: f32,
}

impl TimedText {
    /// Whether this segment needs time-stretching to fit.
    pub fn needs_stretch(&self) -> bool {
        self.stretch_ratio < 0.8 || self.stretch_ratio > 1.2
    }

    /// Whether the stretch ratio is within the safe audible range.
    pub fn stretch_is_safe(&self) -> bool {
        self.stretch_ratio >= 0.7 && self.stretch_ratio <= 1.35
    }
}

/// Final dubbed segment with audio data (not serializable — runtime only).
#[derive(Debug, Clone)]
pub struct DubbedSegment {
    pub timed_text: TimedText,
    pub audio_samples: Vec<f32>,
    pub sample_rate: u32,
}
