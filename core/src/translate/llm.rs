//! LLM-based translation provider.
//!
//! Leverages the existing `core::llm` provider infrastructure to use any
//! configured LLM (Claude, Ollama, OpenAI, Groq, Together, OpenRouter, etc.)
//! for high-accuracy subtitle translation.
//!
//! Particularly strong for Khmer (ភាសាខ្មែរ) where generic MT engines
//! often produce stilted or incorrect output.

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{debug, info};

use super::TranslationProvider;
use crate::llm::{self, LlmProvider, Message};

/// LLM-powered translation provider.
pub struct LlmTranslateProvider {
    provider: Box<dyn LlmProvider>,
    provider_name: String,
}

impl LlmTranslateProvider {
    /// Create from provider config. Reuses `crate::llm::create_provider`.
    pub fn new(
        provider_type: &str,
        model: &str,
        api_key: Option<&str>,
        base_url: Option<&str>,
    ) -> Result<Self> {
        let provider = llm::create_provider(provider_type, model, base_url, api_key)
            .context("Failed to create LLM provider for translation")?;
        let name = format!("llm:{}", provider.name());

        Ok(Self {
            provider,
            provider_name: name,
        })
    }

    /// Build the translation system prompt.
    fn system_prompt(target_lang: &str) -> String {
        let lang_specific = if target_lang == "km" || target_lang.eq_ignore_ascii_case("khmer") {
            r#"
KHMER-SPECIFIC RULES:
- Use natural spoken Khmer (ភាសាខ្មែរនិយាយ), NOT literary/formal style
- Use Khmer script ONLY — no romanization, no transliteration
- Preserve Khmer grammar and sentence structure
- Character names: keep original unless well-known Khmer equivalent exists"#
        } else {
            ""
        };

        format!(
            r#"You are an expert subtitle translator. /no_think

Your job: translate subtitle text accurately while preserving:
1. The original meaning and nuance
2. Natural spoken language (NOT literary/formal)
3. Concise phrasing appropriate for subtitles (viewers must read quickly)
4. Emotional tone and register of the source
5. Proper nouns and names in their original form
{lang_specific}

CRITICAL RULES:
- Return ONLY the translated text
- No quotes, no explanation, no notes, no romanization
- No "Translation:" prefix or any other prefix
- Do NOT think out loud or emit reasoning — output the translation directly
- Preserve line breaks if present in the source
- If the source text is very short (e.g. "OK", "Yes"), translate appropriately
- If the text contains HTML tags or formatting codes, preserve them"#
        )
    }

    /// Build the user prompt for a translation request.
    fn user_prompt(text: &str, source_lang: &str, target_lang: &str) -> String {
        let source_name = lang_name(source_lang);
        let target_name = lang_name(target_lang);

        // `/no_think` is a Qwen3-family directive that disables the model's
        // chain-of-thought reasoning block. Harmless for non-reasoning models
        // (Claude, OpenAI, etc.); required for Qwen3/DeepSeek-R1-style GGUF
        // models that otherwise burn the whole token budget on `<think>…</think>`
        // and return empty content.
        format!(
            "Translate from {source_name} to {target_name}. /no_think\n\n{text}"
        )
    }
}

#[async_trait]
impl TranslationProvider for LlmTranslateProvider {
    fn name(&self) -> &str {
        &self.provider_name
    }

    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String> {
        let system = Self::system_prompt(target_lang);
        let user = Self::user_prompt(text, source_lang, target_lang);

        let messages = vec![
            Message::system(&system),
            Message::user(&user),
        ];

        debug!("LLM translate: \"{}\" ({} → {})", truncate(text, 40), source_lang, target_lang);

        let response = self
            .provider
            .chat(&messages, &[])
            .await
            .context("LLM translation request failed")?;

        let translated = response
            .content
            .ok_or_else(|| anyhow::anyhow!("LLM returned empty response"))?
            .trim()
            .to_string();

        let cleaned = strip_translation_prefix(&translated);

        if cleaned.is_empty() {
            anyhow::bail!("LLM returned empty translation after cleanup. RAW WAS: {:?}", translated);
        }

        debug!("LLM result: \"{}\"", truncate(&cleaned, 40));
        Ok(cleaned)
    }

    /// Batch translate — sequential for LLMs to avoid rate limits.
    async fn translate_batch(
        &self,
        texts: &[String],
        source_lang: &str,
        target_lang: &str,
    ) -> Result<Vec<String>> {
        let total = texts.len();
        let mut results = Vec::with_capacity(total);

        for (i, text) in texts.iter().enumerate() {
            info!(
                "[{}/{}] Translating: \"{}\" ({} → {})",
                i + 1,
                total,
                truncate(text, 30),
                source_lang,
                target_lang,
            );
            let translated = self.translate(text, source_lang, target_lang).await?;
            results.push(translated);
        }

        Ok(results)
    }
}

/// Strip common LLM prefix artifacts from translation output.
fn strip_translation_prefix(text: &str) -> String {
    let prefixes = [
        "Translation:",
        "Translated:",
        "Here is the translation:",
        "Here's the translation:",
        "The translation is:",
    ];
    let trimmed = text.trim();
    for prefix in &prefixes {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.trim().to_string();
        }
    }
    // Strip surrounding quotes if the entire text is quoted.
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('「') && trimmed.ends_with('」'))
        || (trimmed.starts_with('«') && trimmed.ends_with('»'))
    {
        return trimmed[1..trimmed.len() - 1].trim().to_string();
    }
    trimmed.to_string()
}

/// Map language code to human name.
fn lang_name(code: &str) -> &str {
    match code.to_lowercase().as_str() {
        "en" | "english" => "English",
        "km" | "khmer" => "Khmer",
        "zh" | "chinese" | "cn" => "Chinese",
        "ja" | "japanese" => "Japanese",
        "ko" | "korean" => "Korean",
        "th" | "thai" => "Thai",
        "vi" | "vietnamese" => "Vietnamese",
        "fr" | "french" => "French",
        "de" | "german" => "German",
        "es" | "spanish" => "Spanish",
        "pt" | "portuguese" => "Portuguese",
        "ru" | "russian" => "Russian",
        "ar" | "arabic" => "Arabic",
        "hi" | "hindi" => "Hindi",
        "auto" => "auto-detected",
        _ => code,
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
