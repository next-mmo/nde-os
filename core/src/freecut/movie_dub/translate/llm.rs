//! Claude/Ollama LLM translation with duration-aware prompting.
//!
//! Used only when free translation would require excessive time-stretching.
//! The LLM is prompted with target syllable counts so translations fit the
//! original audio timing slots.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::engine::{TranslateEngine, TranslateRequest, TranslateResult};
use super::khmer;
use super::super::lang::Lang;

/// Claude/OpenAI/Ollama LLM translation with duration-aware prompting.
pub struct LlmEngine {
    client: reqwest::Client,
    provider: LlmProvider,
    api_key: String,
    model: String,
    ollama_url: String,
}

#[derive(Debug, Clone, Copy)]
pub enum LlmProvider {
    Claude,
    Ollama,
}

// ── API response types ──

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: Option<OllamaMessage>,
    response: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

// ── Request types ──

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

impl LlmEngine {
    pub fn new_claude(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client"),
            provider: LlmProvider::Claude,
            api_key: api_key.into(),
            model: model.into(),
            ollama_url: String::new(),
        }
    }

    pub fn new_ollama(url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("failed to build HTTP client"),
            provider: LlmProvider::Ollama,
            api_key: String::new(),
            model: model.into(),
            ollama_url: url.into().trim_end_matches('/').to_string(),
        }
    }

    /// Build the dubbing-aware system prompt.
    fn build_system_prompt() -> String {
        r#"You are an expert Khmer movie dubbing translator.

Your job: translate dialogue for voice dubbing. The translated text will be spoken
by a Khmer voice actor/TTS and MUST fit within the original audio timing.

Critical rules:
1. Use natural spoken Khmer (ភាសាខ្មែរនិយាយ), NOT literary/formal style
2. Use Khmer script only — no romanization, no transliteration
3. Match the emotional tone and register of the source
4. Keep translations concise — dubbing needs brevity
5. If a syllable count target is given, stay within ±2 syllables
6. Preserve character names in their original form unless they have
   well-known Khmer equivalents
7. Return ONLY the Khmer translation text, nothing else — no quotes,
   no explanation, no romanization"#.to_string()
    }

    /// Build the user prompt for a dubbing translation request.
    fn build_user_prompt(req: &TranslateRequest) -> String {
        let max_syls = req.max_duration_ms
            .map(|ms| khmer::ms_to_max_syllables(ms));

        let mut prompt = format!(
            "Source language: {}\nDialogue: \"{}\"\n",
            req.source_lang.name(),
            req.text,
        );

        if let Some(ms) = req.max_duration_ms {
            prompt.push_str(&format!("Time slot: {}ms\n", ms));
        }
        if let Some(max) = max_syls {
            prompt.push_str(&format!("Target: maximum {} Khmer syllables\n", max));
        }
        if let Some(ctx) = &req.context {
            prompt.push_str(&format!("Scene context: {}\n", ctx));
        }

        prompt.push_str("\nTranslate to spoken Khmer:");
        prompt
    }

    /// Call Claude API.
    async fn call_claude(&self, system: &str, user_msg: &str) -> Result<String> {
        let body = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 512,
            system: system.to_string(),
            messages: vec![Message {
                role: "user".into(),
                content: user_msg.into(),
            }],
        };

        debug!("Claude request model={}", self.model);

        let resp = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Claude API request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Claude API error {status}: {body}");
        }

        let data: ClaudeResponse = resp.json()
            .await
            .context("Failed to parse Claude response")?;

        let text = data.content
            .into_iter()
            .filter_map(|c| c.text)
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string();

        if text.is_empty() {
            anyhow::bail!("Claude returned empty response");
        }

        Ok(text)
    }

    /// Call Ollama API.
    async fn call_ollama(&self, system: &str, user_msg: &str) -> Result<String> {
        let body = OllamaChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message { role: "system".into(), content: system.into() },
                Message { role: "user".into(), content: user_msg.into() },
            ],
            stream: false,
        };

        let url = format!("{}/api/chat", self.ollama_url);
        debug!("Ollama request model={} url={url}", self.model);

        let resp = self.client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Ollama request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama error {status}: {body}");
        }

        let data: OllamaResponse = resp.json()
            .await
            .context("Failed to parse Ollama response")?;

        let text = data.message
            .map(|m| m.content)
            .or(data.response)
            .unwrap_or_default()
            .trim()
            .to_string();

        if text.is_empty() {
            anyhow::bail!("Ollama returned empty response");
        }

        Ok(text)
    }

    /// Call whichever LLM provider is configured.
    async fn call_llm(&self, system: &str, user_msg: &str) -> Result<String> {
        match self.provider {
            LlmProvider::Claude => self.call_claude(system, user_msg).await,
            LlmProvider::Ollama => self.call_ollama(system, user_msg).await,
        }
    }
}

#[async_trait::async_trait]
impl TranslateEngine for LlmEngine {
    fn name(&self) -> &str {
        match self.provider {
            LlmProvider::Claude => "claude",
            LlmProvider::Ollama => "ollama",
        }
    }

    async fn translate(
        &self,
        text: &str,
        source: Lang,
        target: Lang,
    ) -> Result<String> {
        let system = Self::build_system_prompt();
        let user_msg = format!(
            "Source ({}): \"{}\"\nTranslate to {}:",
            source.name(), text, target.name()
        );
        self.call_llm(&system, &user_msg).await
    }

    /// Duration-aware translation with retry loop.
    async fn translate_for_dub(
        &self,
        req: &TranslateRequest,
    ) -> Result<TranslateResult> {
        let system = Self::build_system_prompt();
        let user_msg = Self::build_user_prompt(req);

        let mut best_result: Option<TranslateResult> = None;
        let max_attempts = req.max_retries + 1;

        for attempt in 0..max_attempts {
            let text = self.call_llm(&system, &user_msg).await?;
            let syllables = khmer::estimate_syllables(&text);
            let est_ms = khmer::syllables_to_ms(syllables);
            let stretch = req.max_duration_ms
                .map(|d| d as f32 / est_ms as f32)
                .unwrap_or(1.0);

            let result = TranslateResult {
                text,
                estimated_duration_ms: est_ms,
                syllable_count: syllables,
                provider: self.name().to_string(),
                stretch_ratio: stretch,
            };

            info!(
                "LLM translate attempt {}/{}: {} syls, {}ms est, {:.2}x stretch",
                attempt + 1, max_attempts, syllables, est_ms, stretch
            );

            // Perfect or good enough → return.
            if stretch >= 0.85 && stretch <= 1.15 {
                debug!("Translation fits well, no stretch needed");
                return Ok(result);
            }

            // Acceptable stretch range → keep as best but try again.
            if stretch >= 0.7 && stretch <= 1.35 {
                if best_result.as_ref()
                    .map(|b| (stretch - 1.0).abs() < (b.stretch_ratio - 1.0).abs())
                    .unwrap_or(true)
                {
                    best_result = Some(result);
                }
            } else if best_result.is_none() {
                best_result = Some(result);
            }
        }

        best_result.ok_or_else(|| anyhow::anyhow!("All translation attempts failed"))
    }
}
