//! Shared types for the NDE-OS voice services (TTS + RVC).

use serde::{Deserialize, Serialize};

// ─── TTS ───────────────────────────────────────────────────────────────────────

/// Request to synthesize speech from text via Edge TTS.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsSynthesizeRequest {
    pub text: String,
    #[serde(default = "default_tts_voice")]
    pub voice: String,
    #[serde(default)]
    pub rate: Option<String>,
    #[serde(default)]
    pub pitch: Option<String>,
    #[serde(default)]
    pub volume: Option<String>,
    /// Where to write the output MP3. If `None`, auto-generates a temp path.
    #[serde(default)]
    pub output_path: Option<String>,
}

fn default_tts_voice() -> String {
    "en-US-AriaNeural".to_string()
}

/// Result of a TTS synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsSynthesizeResult {
    pub audio_path: String,
    pub voice: String,
}

// ─── RVC ───────────────────────────────────────────────────────────────────────

/// Request to run RVC voice conversion on an audio file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RvcConvertRequest {
    /// Path to the input audio file (e.g. TTS output).
    pub input_audio: String,
    /// Path to the `.pth` model file.
    pub model_path: String,
    /// Path to the `.index` file.
    pub index_path: String,
    /// Pitch shift in semitones.
    #[serde(default)]
    pub pitch_shift: Option<i32>,
    /// Optional explicit Python path. Falls back to runtime detection.
    #[serde(default)]
    pub python_path: Option<String>,
    /// Optional RVC CLI script path (e.g. `rvc.py`).
    #[serde(default)]
    pub cli_path: Option<String>,
    /// Where to write the converted output. If `None`, auto-generates.
    #[serde(default)]
    pub output_path: Option<String>,
    /// Output format. Defaults to MP3.
    #[serde(default = "default_rvc_format")]
    pub output_format: String,
}

fn default_rvc_format() -> String {
    "MP3".to_string()
}

/// Result of RVC conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RvcConvertResult {
    pub output_path: String,
    pub model_path: String,
}

/// A discovered RVC model on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceModel {
    pub name: String,
    pub model_path: String,
    #[serde(default)]
    pub index_path: Option<String>,
}

// ─── Runtime ───────────────────────────────────────────────────────────────────

/// Status of the global voice runtime.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VoiceRuntimeStatus {
    pub runtime_installed: bool,
    pub runtime_path: Option<String>,
    pub edge_tts_available: bool,
    pub whisper_available: bool,
    pub python_available: bool,
    pub rvc_available: bool,
    #[serde(default)]
    pub voices: Vec<String>,
    #[serde(default)]
    pub details: Vec<String>,
}

/// Request to install voice runtime components.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceInstallRequest {
    pub components: Vec<String>,
}

/// Result of a voice runtime install.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceInstallResult {
    pub installed_packages: Vec<String>,
    pub runtime_path: String,
    pub bin_path: String,
    pub message: String,
}
