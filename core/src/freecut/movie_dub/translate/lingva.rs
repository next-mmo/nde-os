//! Free translation via Lingva (Google Translate proxy).
//!
//! No API key required. Self-hostable.
//! Public instances: lingva.ml, lingva.thedaviddelta.com

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, warn};

use super::super::lang::Lang;

/// Free translation via Lingva (Google Translate proxy).
pub struct LingvaEngine {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct LingvaResponse {
    translation: String,
}

impl LingvaEngine {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("failed to build HTTP client"),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    /// List of known public Lingva instances for fallback.
    const FALLBACK_URLS: &'static [&'static str] = &[
        "https://lingva.ml",
        "https://lingva.thedaviddelta.com",
        "https://translate.plausibility.cloud",
    ];

    /// Try primary URL, then fallbacks.
    pub async fn translate_with_fallback(
        &self,
        text: &str,
        source: Lang,
        target: Lang,
    ) -> Result<String> {
        // Try primary.
        match self.do_translate(&self.base_url, text, source, target).await {
            Ok(t) => return Ok(t),
            Err(e) => warn!("Primary Lingva failed: {e}"),
        }

        // Try fallbacks.
        for url in Self::FALLBACK_URLS {
            if *url == self.base_url {
                continue;
            }
            match self.do_translate(url, text, source, target).await {
                Ok(t) => {
                    debug!("Lingva fallback succeeded: {url}");
                    return Ok(t);
                }
                Err(e) => warn!("Lingva fallback {url} failed: {e}"),
            }
        }

        anyhow::bail!("All Lingva instances failed for translation")
    }

    async fn do_translate(
        &self,
        base: &str,
        text: &str,
        source: Lang,
        target: Lang,
    ) -> Result<String> {
        let encoded = urlencoding::encode(text);
        let url = format!(
            "{}/api/v1/auto/{}/{}",
            base,
            target.code(),
            encoded
        );

        debug!("Lingva request: {url}");

        let resp = self.client.get(&url)
            .send()
            .await
            .context("Lingva request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Lingva HTTP {status}: {body}");
        }

        let data: LingvaResponse = resp.json()
            .await
            .context("Failed to parse Lingva response")?;

        if data.translation.is_empty() {
            anyhow::bail!("Lingva returned empty translation");
        }

        Ok(data.translation)
    }
}

#[async_trait::async_trait]
impl super::engine::TranslateEngine for LingvaEngine {
    fn name(&self) -> &str {
        "lingva"
    }

    async fn translate(
        &self,
        text: &str,
        source: Lang,
        target: Lang,
    ) -> Result<String> {
        self.translate_with_fallback(text, source, target).await
    }
}
