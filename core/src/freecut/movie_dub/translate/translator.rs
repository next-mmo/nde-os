//! Smart translation router for movie dubbing.
//!
//! Routes between free (Lingva) and paid (LLM) translation engines.
//! Strategy: try free first, only use LLM when the free translation
//! would require excessive time-stretching.

use anyhow::Result;
use tracing::{debug, info, warn};

use super::engine::{TranslateEngine, TranslateRequest, TranslateResult};
use super::khmer;
use super::lingva::LingvaEngine;
use super::llm::LlmEngine;
use super::super::config::TranslationConfig;
use super::super::lang::Lang;
use super::super::segment::{Segment, TimedText};

/// Main translator that routes between free and LLM engines.
pub struct Translator {
    free: LingvaEngine,
    llm: Option<LlmEngine>,
    config: TranslationConfig,
}

impl Translator {
    /// Build from config.
    pub fn from_config(config: &TranslationConfig) -> Self {
        let free = LingvaEngine::new(&config.lingva_url);

        let llm = if !config.llm_api_key.is_empty() {
            let engine = match config.llm_provider.as_str() {
                "ollama" => LlmEngine::new_ollama(
                    &config.ollama_url,
                    &config.llm_model,
                ),
                _ => LlmEngine::new_claude(
                    &config.llm_api_key,
                    &config.llm_model,
                ),
            };
            info!("LLM engine enabled: {} ({})", config.llm_provider, config.llm_model);
            Some(engine)
        } else {
            info!("No LLM key — using free translation only");
            None
        };

        Self { free, llm, config: config.clone() }
    }

    /// Translate a single segment for dubbing.
    pub async fn translate_segment(
        &self,
        segment: &Segment,
        target_lang: Lang,
    ) -> Result<TimedText> {
        let req = TranslateRequest::new(
            &segment.source_text,
            segment.source_lang,
            target_lang,
        ).with_duration(segment.duration_ms());

        let result = self.translate_auto(&req).await?;

        Ok(TimedText {
            segment: segment.clone(),
            translated_text: result.text,
            target_lang,
            estimated_duration_ms: result.estimated_duration_ms,
            syllable_count: Some(result.syllable_count),
            provider_used: result.provider,
            stretch_ratio: result.stretch_ratio,
        })
    }

    /// Translate a batch of segments.
    pub async fn translate_segments(
        &self,
        segments: &[Segment],
        target_lang: Lang,
    ) -> Result<Vec<TimedText>> {
        let mut results = Vec::with_capacity(segments.len());

        for (i, seg) in segments.iter().enumerate() {
            info!(
                "[{}/{}] Translating: \"{}\" ({} → {}, {}ms slot)",
                i + 1, segments.len(),
                truncate(&seg.source_text, 40),
                seg.source_lang.code(),
                target_lang.code(),
                seg.duration_ms(),
            );

            let timed = self.translate_segment(seg, target_lang).await?;

            debug!(
                "  → \"{}\" | {} syls | {}ms est | {:.2}x stretch | via {}",
                truncate(&timed.translated_text, 30),
                timed.syllable_count.unwrap_or(0),
                timed.estimated_duration_ms,
                timed.stretch_ratio,
                timed.provider_used,
            );

            results.push(timed);
        }

        // Summary.
        let stretchy: Vec<_> = results.iter().filter(|t| t.needs_stretch()).collect();
        let unsafe_stretch: Vec<_> = results.iter().filter(|t| !t.stretch_is_safe()).collect();

        info!(
            "Translation done: {} segments, {} need stretching, {} risky",
            results.len(), stretchy.len(), unsafe_stretch.len()
        );

        Ok(results)
    }

    /// Smart routing: use LLM only when free translation would need excessive stretching.
    async fn translate_auto(&self, req: &TranslateRequest) -> Result<TranslateResult> {
        match (&self.llm, self.config.provider.as_str()) {
            // Force free.
            (_, "free") => {
                debug!("Forced free engine");
                self.translate_free(req).await
            }

            // Force LLM.
            (Some(llm), "llm") => {
                debug!("Forced LLM engine");
                llm.translate_for_dub(req).await
            }

            // Auto: has LLM key.
            (Some(llm), _) => {
                // Step 1: try free translation first (save tokens).
                let free_result = self.translate_free(req).await?;

                // Step 2: if free result fits well enough, use it.
                if free_result.stretch_ratio >= 0.8 && free_result.stretch_ratio <= 1.25 {
                    debug!(
                        "Free translation fits ({:.2}x stretch), skipping LLM",
                        free_result.stretch_ratio
                    );
                    return Ok(free_result);
                }

                // Step 3: free doesn't fit → use LLM for length-aware translation.
                info!(
                    "Free stretch {:.2}x out of range, using LLM for better fit",
                    free_result.stretch_ratio
                );

                match llm.translate_for_dub(req).await {
                    Ok(llm_result) => {
                        let free_dist = (free_result.stretch_ratio - 1.0).abs();
                        let llm_dist = (llm_result.stretch_ratio - 1.0).abs();

                        if llm_dist < free_dist {
                            debug!("LLM result fits better ({:.2}x vs {:.2}x)",
                                llm_result.stretch_ratio, free_result.stretch_ratio);
                            Ok(llm_result)
                        } else {
                            debug!("Free result actually fits better, keeping it");
                            Ok(free_result)
                        }
                    }
                    Err(e) => {
                        warn!("LLM failed, falling back to free: {e}");
                        Ok(free_result)
                    }
                }
            }

            // No LLM key.
            (None, _) => {
                self.translate_free(req).await
            }
        }
    }

    /// Translate using free engine with duration estimation.
    async fn translate_free(&self, req: &TranslateRequest) -> Result<TranslateResult> {
        let text = self.free
            .translate(&req.text, req.source_lang, req.target_lang)
            .await?;

        let syllables = khmer::estimate_syllables(&text);
        let est_ms = khmer::syllables_to_ms(syllables);
        let stretch = req.max_duration_ms
            .map(|d| d as f32 / est_ms as f32)
            .unwrap_or(1.0);

        Ok(TranslateResult {
            text,
            estimated_duration_ms: est_ms,
            syllable_count: syllables,
            provider: "lingva".to_string(),
            stretch_ratio: stretch,
        })
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max - 3).collect();
        format!("{t}...")
    }
}
