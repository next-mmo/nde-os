//! Translation engine trait and shared types for movie dubbing.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::super::lang::Lang;

/// Request to translate a segment for dubbing.
#[derive(Debug, Clone)]
pub struct TranslateRequest {
    pub text: String,
    pub source_lang: Lang,
    pub target_lang: Lang,
    /// Available duration in the original audio (ms).
    pub max_duration_ms: Option<u64>,
    /// Scene/emotion context for better translation.
    pub context: Option<String>,
    /// Max retries for length-aware translation.
    pub max_retries: u32,
}

impl TranslateRequest {
    pub fn new(text: impl Into<String>, source: Lang, target: Lang) -> Self {
        Self {
            text: text.into(),
            source_lang: source,
            target_lang: target,
            max_duration_ms: None,
            context: None,
            max_retries: 2,
        }
    }

    pub fn with_duration(mut self, ms: u64) -> Self {
        self.max_duration_ms = Some(ms);
        self
    }

    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context = Some(ctx.into());
        self
    }
}

/// Translation result with timing metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateResult {
    pub text: String,
    pub estimated_duration_ms: u64,
    pub syllable_count: u32,
    pub provider: String,
    pub stretch_ratio: f32,
}

impl TranslateResult {
    pub fn needs_stretch(&self) -> bool {
        self.stretch_ratio < 0.85 || self.stretch_ratio > 1.15
    }
}

/// Trait all translation engines must implement.
#[async_trait::async_trait]
pub trait TranslateEngine: Send + Sync {
    fn name(&self) -> &str;

    async fn translate(
        &self,
        text: &str,
        source: Lang,
        target: Lang,
    ) -> Result<String>;

    /// Duration-aware translate (default: ignore duration, just translate).
    async fn translate_for_dub(
        &self,
        req: &TranslateRequest,
    ) -> Result<TranslateResult> {
        let text = self.translate(&req.text, req.source_lang, req.target_lang).await?;
        let syllables = super::khmer::estimate_syllables(&text);
        let est_ms = super::khmer::syllables_to_ms(syllables);
        let stretch = req.max_duration_ms
            .map(|d| d as f32 / est_ms as f32)
            .unwrap_or(1.0);

        Ok(TranslateResult {
            text,
            estimated_duration_ms: est_ms,
            syllable_count: syllables,
            provider: self.name().to_string(),
            stretch_ratio: stretch,
        })
    }
}
