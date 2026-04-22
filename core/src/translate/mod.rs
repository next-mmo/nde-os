//! Standalone SRT translation service with pluggable providers.
//!
//! Decoupled from FreeCut/movie-dub — this is a first-class NDE-OS core
//! subsystem for translating subtitle files between any languages.
//!
//! Provider hierarchy:
//! 1. **LLM** — highest accuracy for Khmer. Uses the existing `core::llm`
//!    infrastructure (Claude, Ollama, OpenAI-compat, Groq, etc.).
//! 2. **Google Translate** — free, fast, good for many languages.
//!    Proxied via Lingva (no API key) or direct Google Cloud API.
//! 3. **NDE Agent** — uses the local NDE-OS agent chat endpoint as
//!    a translation engine (whatever model is currently active).
//!
//! Usage:
//! ```rust,ignore
//! use ai_launcher_core::translate::{TranslateService, TranslateConfig, Provider};
//!
//! let config = TranslateConfig {
//!     provider: Provider::Llm { .. },
//!     source_lang: "en".into(),
//!     target_lang: "km".into(),
//!     ..Default::default()
//! };
//! let service = TranslateService::new(config);
//! let translated_srt = service.translate_srt(&srt_content).await?;
//! ```

pub mod google;
pub mod llm;
pub mod nde_agent;
pub mod srt;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ── Provider trait ───────────────────────────────────────────────────────────

/// Trait all translation providers must implement.
#[async_trait]
pub trait TranslationProvider: Send + Sync {
    /// Human-readable provider name.
    fn name(&self) -> &str;

    /// Translate a single text string.
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String>;

    /// Translate a batch of texts (default: sequential).
    /// Override for providers that support batch APIs.
    async fn translate_batch(
        &self,
        texts: &[String],
        source_lang: &str,
        target_lang: &str,
    ) -> Result<Vec<String>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.translate(text, source_lang, target_lang).await?);
        }
        Ok(results)
    }
}

// ── Provider config ──────────────────────────────────────────────────────────

/// Which provider to use for translation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Provider {
    /// Google Translate via Lingva proxy (free, no API key).
    Google {
        /// Lingva instance URL (default: https://lingva.ml).
        #[serde(default = "default_lingva_url")]
        lingva_url: String,
    },
    /// LLM-based translation (Claude, Ollama, OpenAI, etc.).
    Llm {
        /// Provider type: "claude", "ollama", "openai", "groq", etc.
        provider: String,
        /// Model name.
        model: String,
        /// API key (optional — env vars are checked as fallback).
        #[serde(default)]
        api_key: Option<String>,
        /// Base URL (optional).
        #[serde(default)]
        base_url: Option<String>,
    },
    /// Use the local NDE-OS agent chat endpoint.
    NdeAgent {
        /// NDE server URL (default: http://127.0.0.1:8080).
        #[serde(default = "default_nde_url")]
        url: String,
    },
}

fn default_lingva_url() -> String {
    "https://lingva.ml".into()
}

fn default_nde_url() -> String {
    "http://127.0.0.1:8080".into()
}

impl Default for Provider {
    fn default() -> Self {
        Provider::Google {
            lingva_url: default_lingva_url(),
        }
    }
}

// ── Service config ───────────────────────────────────────────────────────────

/// Top-level translation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateConfig {
    /// Provider to use.
    pub provider: Provider,
    /// Source language code (e.g. "en", "zh", "auto").
    pub source_lang: String,
    /// Target language code (e.g. "km", "en", "zh").
    pub target_lang: String,
    /// Max concurrent translations (for batch operations).
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

fn default_concurrency() -> usize {
    4
}

impl Default for TranslateConfig {
    fn default() -> Self {
        Self {
            provider: Provider::default(),
            source_lang: "en".into(),
            target_lang: "km".into(),
            concurrency: default_concurrency(),
        }
    }
}

// ── Translation result ───────────────────────────────────────────────────────

/// Result of translating a single SRT cue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatedCue {
    /// 1-indexed sequence number.
    pub index: u32,
    /// Start timestamp in milliseconds.
    pub start_ms: u64,
    /// End timestamp in milliseconds.
    pub end_ms: u64,
    /// Original text.
    pub original_text: String,
    /// Translated text.
    pub translated_text: String,
    /// Provider that produced this translation.
    pub provider: String,
}

/// Result of translating an entire SRT file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateSrtResult {
    /// Translated cues.
    pub cues: Vec<TranslatedCue>,
    /// The full translated SRT as a string.
    pub srt_content: String,
    /// Provider used.
    pub provider: String,
    /// Source language.
    pub source_lang: String,
    /// Target language.
    pub target_lang: String,
    /// Number of cues translated.
    pub cue_count: usize,
}

// ── Main service ─────────────────────────────────────────────────────────────

/// The main translation service.
pub struct TranslateService {
    config: TranslateConfig,
    provider: Box<dyn TranslationProvider>,
}

impl TranslateService {
    /// Create a new translation service from config.
    pub fn new(config: TranslateConfig) -> Result<Self> {
        let provider: Box<dyn TranslationProvider> = match &config.provider {
            Provider::Google { lingva_url } => {
                Box::new(google::GoogleTranslateProvider::new(lingva_url))
            }
            Provider::Llm {
                provider,
                model,
                api_key,
                base_url,
            } => Box::new(llm::LlmTranslateProvider::new(
                provider,
                model,
                api_key.as_deref(),
                base_url.as_deref(),
            )?),
            Provider::NdeAgent { url } => {
                Box::new(nde_agent::NdeAgentProvider::new(url))
            }
        };

        Ok(Self { config, provider })
    }

    /// Translate an SRT string → translated SRT string + metadata.
    pub async fn translate_srt(&self, srt_content: &str) -> Result<TranslateSrtResult> {
        // Parse SRT.
        let cues = srt::parse_srt(srt_content)?;
        if cues.is_empty() {
            anyhow::bail!("No subtitle cues found in SRT content");
        }

        info!(
            "Translating {} cues ({} → {}) via {}",
            cues.len(),
            self.config.source_lang,
            self.config.target_lang,
            self.provider.name(),
        );

        // Extract texts for batch translation.
        let texts: Vec<String> = cues.iter().map(|c| c.text.clone()).collect();

        // Translate batch.
        let translated = self
            .provider
            .translate_batch(
                &texts,
                &self.config.source_lang,
                &self.config.target_lang,
            )
            .await?;

        // Build result cues.
        let provider_name = self.provider.name().to_string();
        let mut result_cues = Vec::with_capacity(cues.len());
        for (cue, trans_text) in cues.iter().zip(translated.iter()) {
            result_cues.push(TranslatedCue {
                index: cue.index,
                start_ms: cue.start_ms,
                end_ms: cue.end_ms,
                original_text: cue.text.clone(),
                translated_text: trans_text.clone(),
                provider: provider_name.clone(),
            });
        }

        // Build translated SRT string.
        let srt_output = srt::build_srt(&result_cues);
        let cue_count = result_cues.len();

        info!(
            "Translation complete: {} cues via {}",
            cue_count, provider_name
        );

        Ok(TranslateSrtResult {
            cues: result_cues,
            srt_content: srt_output,
            provider: provider_name,
            source_lang: self.config.source_lang.clone(),
            target_lang: self.config.target_lang.clone(),
            cue_count,
        })
    }

    /// Translate a single text string.
    pub async fn translate_text(&self, text: &str) -> Result<String> {
        self.provider
            .translate(text, &self.config.source_lang, &self.config.target_lang)
            .await
    }

    /// List available providers.
    pub fn available_providers() -> Vec<ProviderInfo> {
        vec![
            ProviderInfo {
                id: "google".into(),
                name: "Google Translate".into(),
                description: "Free translation via Lingva proxy (Google Translate). No API key required.".into(),
                requires_api_key: false,
                accuracy_tier: "good".into(),
            },
            ProviderInfo {
                id: "llm".into(),
                name: "LLM (Claude / Ollama / OpenAI / Groq)".into(),
                description: "High-accuracy translation using large language models. Best for Khmer and context-aware translations.".into(),
                requires_api_key: true,
                accuracy_tier: "best".into(),
            },
            ProviderInfo {
                id: "nde_agent".into(),
                name: "NDE Agent (Local)".into(),
                description: "Uses the currently active NDE-OS LLM model for translation. No extra configuration needed.".into(),
                requires_api_key: false,
                accuracy_tier: "best".into(),
            },
        ]
    }
}

/// Info about an available translation provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub requires_api_key: bool,
    pub accuracy_tier: String,
}
