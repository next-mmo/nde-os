//! Text normalization + phonemization pipeline for Khmer.
//!
//! Combines: khmernormalizer → khmercut CRF tokenizer → lexicon/G2P phonemizer.

use anyhow::Result;
use crfs::Model;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::kfa::g2p::G2pModel;
use crate::kfa::lexicon::LEXICON;
use crate::kfa::normalizer::normalize;
use crate::kfa::number_verbalize::{number_replacer, number_translate2ascii};
use crate::kfa::vocab::lookup;

static CRF_MODEL_FILE: &[u8] = include_bytes!("data/crf_ner_10000.crfsuite");

static CRF_MODEL: Lazy<Model> =
    Lazy::new(|| Model::new(CRF_MODEL_FILE).expect("Failed to load embedded CRF model"));

/// A token that was successfully phonemized.
#[derive(Debug, Clone)]
pub enum PhonemizedToken {
    Known {
        token: String,
        lattice: String,
        token_ids: Vec<usize>,
    },
    Unknown {
        token: String,
    },
}

/// Stateless text pipeline that holds G2P model reference.
pub struct TextPipeline {
    pub g2p: G2pModel,
}

impl TextPipeline {
    pub fn new() -> Result<Self> {
        Ok(Self { g2p: G2pModel::new()? })
    }

    /// Normalize, tokenize, and phonemize a line of Khmer text.
    pub fn tokenize_phonemize(&self, text: &str) -> Result<Vec<PhonemizedToken>> {
        let normalized = normalize(text, true);
        let mut results = Vec::new();

        let tokens = khmercut::tokenize(&CRF_MODEL, &normalized);
        for token in tokens {
            if let Some(phonemic) = phonemize(&token, &self.g2p) {
                let lattices_str = phonemic.join("");
                let re_dots = Regex::new(r"\.+").unwrap();
                let cleaned = re_dots.replace_all(&lattices_str, ".").into_owned();

                let token_ids: Vec<usize> = cleaned.chars()
                    .map(|c: char| lookup(&c.to_string()))
                    .collect::<Vec<usize>>();

                results.push(PhonemizedToken::Known {
                    token: token.clone(),
                    lattice: cleaned,
                    token_ids,
                });
            } else {
                results.push(PhonemizedToken::Unknown { token: token.clone() });
            }
        }
        Ok(results)
    }
}

fn phonemize(text: &str, g2p: &G2pModel) -> Option<Vec<String>> {
    let lower_text = text.to_lowercase();
    let ascii_num_text = number_translate2ascii(&lower_text);
    let verbalized_text = number_replacer(&ascii_num_text);

    if verbalized_text.contains('▁') {
        let mut result = Vec::new();
        for subtoken in verbalized_text.split('▁') {
            if let Some(mut phonemes) = phonemize(subtoken, g2p) {
                result.append(&mut phonemes);
                result.push(".".to_string());
            }
        }
        return Some(result);
    }

    let re_clean = Regex::new(r"[^\u{1780}-\u{17d2}a-z]+").unwrap();
    let cleaned = re_clean.replace_all(&verbalized_text, "").into_owned();

    if cleaned.trim().is_empty() {
        return None;
    }

    if let Some(lex) = LEXICON.get(&cleaned) {
        return Some(lex.clone());
    }

    g2p.phoneticize(&cleaned).ok()
}
