//! Language definitions for movie dubbing.
//!
//! Supported source languages: English, Chinese.
//! Target language: Khmer (ខ្មែរ).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Languages supported by the movie dubbing pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    En,
    Zh,
    Km,
}

impl Lang {
    /// ISO 639-1 language code.
    pub fn code(&self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Zh => "zh",
            Lang::Km => "km",
        }
    }

    /// Human-readable language name.
    pub fn name(&self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Zh => "Chinese",
            Lang::Km => "Khmer",
        }
    }

    /// Average syllables per second for natural speech in this language.
    pub fn syllable_rate(&self) -> f32 {
        match self {
            Lang::En => 5.0,
            Lang::Zh => 4.5,
            Lang::Km => 6.5,
        }
    }

    /// Parse a language code string into a `Lang`.
    pub fn from_code(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "en" | "english" => Ok(Lang::En),
            "zh" | "chinese" | "cn" => Ok(Lang::Zh),
            "km" | "khmer" => Ok(Lang::Km),
            _ => anyhow::bail!("Unknown language: {s} (supported: en, zh, km)"),
        }
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
