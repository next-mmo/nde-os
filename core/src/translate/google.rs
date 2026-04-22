//! Google Translate provider via Lingva proxy.
//!
//! Lingva is a free, self-hostable Google Translate frontend.
//! No API key required. Public instances available as fallback.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, warn};

use super::TranslationProvider;

/// Google Translate via Lingva proxy.
pub struct GoogleTranslateProvider {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct LingvaResponse {
    translation: String,
}

impl GoogleTranslateProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("failed to build HTTP client"),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    /// Known public Lingva instances for fallback.
    const FALLBACK_URLS: &'static [&'static str] = &[
        "https://lingva.ml",
        "https://lingva.thedaviddelta.com",
        "https://translate.plausibility.cloud",
    ];

    async fn do_translate(
        &self,
        base: &str,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String> {
        let source = if source_lang == "auto" || source_lang.is_empty() {
            "auto"
        } else {
            source_lang
        };
        let encoded = urlencoding::encode(text);
        let url = format!("{}/api/v1/{}/{}/{}", base, source, target_lang, encoded);

        debug!("Lingva request: {url}");

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Lingva request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Lingva HTTP {status}: {body}");
        }

        let data: LingvaResponse = resp
            .json()
            .await
            .context("Failed to parse Lingva response")?;

        if data.translation.is_empty() {
            anyhow::bail!("Lingva returned empty translation");
        }

        Ok(data.translation)
    }
}

#[async_trait]
impl TranslationProvider for GoogleTranslateProvider {
    fn name(&self) -> &str {
        "google"
    }

    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String> {
        // Try primary.
        match self
            .do_translate(&self.base_url, text, source_lang, target_lang)
            .await
        {
            Ok(t) => return Ok(t),
            Err(e) => warn!("Primary Lingva instance failed: {e}"),
        }

        // Try fallbacks.
        for url in Self::FALLBACK_URLS {
            if *url == self.base_url {
                continue;
            }
            match self.do_translate(url, text, source_lang, target_lang).await {
                Ok(t) => {
                    debug!("Lingva fallback succeeded: {url}");
                    return Ok(t);
                }
                Err(e) => warn!("Lingva fallback {url} failed: {e}"),
            }
        }

        anyhow::bail!("All Lingva instances failed for translation")
    }
}
