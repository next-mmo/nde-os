//! Pipeline phases and progress reporting for movie dubbing.

use serde::{Deserialize, Serialize};

/// Pipeline processing phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Extract,
    Transcribe,
    Translate,
    Separate,
    Synthesize,
    Sync,
    Mix,
    Export,
}

impl Phase {
    /// All phases in order.
    pub fn all() -> &'static [Phase] {
        &[
            Phase::Extract,
            Phase::Transcribe,
            Phase::Translate,
            Phase::Separate,
            Phase::Synthesize,
            Phase::Sync,
            Phase::Mix,
            Phase::Export,
        ]
    }
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Extract => write!(f, "Extract"),
            Phase::Transcribe => write!(f, "Transcribe"),
            Phase::Translate => write!(f, "Translate"),
            Phase::Separate => write!(f, "Separate"),
            Phase::Synthesize => write!(f, "Synthesize"),
            Phase::Sync => write!(f, "Sync"),
            Phase::Mix => write!(f, "Mix"),
            Phase::Export => write!(f, "Export"),
        }
    }
}

/// Progress callback type for the pipeline.
pub type ProgressFn = Box<dyn Fn(Phase, f32, &str) + Send + Sync>;

/// Options for a full video dubbing run.
#[derive(Debug, Clone)]
pub struct DubVideoOptions {
    pub input_path: std::path::PathBuf,
    pub output_path: std::path::PathBuf,
    pub source_lang: super::lang::Lang,
    pub dual_audio: bool,
    pub generate_subtitles: bool,
    pub burn_subtitles: bool,
}
