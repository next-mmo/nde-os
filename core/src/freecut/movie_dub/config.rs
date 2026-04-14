//! Configuration for the movie dubbing pipeline.
//!
//! All settings are per-project and stored alongside FreeCut project data.
//! LLM API keys should be resolved from the NDE-OS secrets store, not stored
//! in plaintext config files.

use serde::{Deserialize, Serialize};

/// Top-level movie dubbing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieDubConfig {
    pub translation: TranslationConfig,
    pub sync: SyncConfig,
    pub tts: TtsConfig,
    pub stt: SttConfig,
    pub output: OutputConfig,
}

impl Default for MovieDubConfig {
    fn default() -> Self {
        Self {
            translation: TranslationConfig::default(),
            sync: SyncConfig::default(),
            tts: TtsConfig::default(),
            stt: SttConfig::default(),
            output: OutputConfig::default(),
        }
    }
}

/// Translation engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    /// "auto" | "free" | "llm"
    pub provider: String,
    /// Free engine: "lingva"
    pub free_engine: String,
    /// Lingva instance URL.
    pub lingva_url: String,
    /// LLM provider: "claude" | "ollama"
    pub llm_provider: String,
    /// API key — should be resolved from NDE-OS secrets at runtime.
    #[serde(default)]
    pub llm_api_key: String,
    /// Model name (e.g. "claude-sonnet-4-20250514").
    pub llm_model: String,
    /// Ollama base URL.
    pub ollama_url: String,
    /// Use LLM for length-controlled dub translation.
    pub enable_length_control: bool,
    /// Target Khmer syllable rate (syllables/sec).
    pub target_syllable_rate: f32,
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self {
            provider: "auto".into(),
            free_engine: "lingva".into(),
            lingva_url: "https://lingva.ml".into(),
            llm_provider: "claude".into(),
            llm_api_key: String::new(),
            llm_model: "claude-sonnet-4-20250514".into(),
            ollama_url: "http://localhost:11434".into(),
            enable_length_control: true,
            target_syllable_rate: 6.5,
        }
    }
}

/// WSOLA time-stretch configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Max allowed stretch ratio before re-translating.
    pub max_stretch_ratio: f32,
    /// Min allowed stretch ratio.
    pub min_stretch_ratio: f32,
    /// Padding silence at segment boundaries (ms).
    pub boundary_padding_ms: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_stretch_ratio: 1.3,
            min_stretch_ratio: 0.75,
            boundary_padding_ms: 50,
        }
    }
}

/// TTS engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// "edge" (via NDE-OS voice runtime)
    pub engine: String,
    /// Khmer voice name for edge-tts.
    pub edge_voice: String,
    /// Speech speed multiplier (1.0 = normal).
    pub speed: f32,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            engine: "edge".into(),
            edge_voice: "km-KH-PisethNeural".into(),
            speed: 1.0,
        }
    }
}

/// STT engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// Whisper model: "tiny" | "base" | "small" | "medium" | "large"
    pub model_size: String,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            model_size: "medium".into(),
        }
    }
}

/// Output configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            channels: 1,
        }
    }
}
