//! Translation subsystem for movie dubbing.
//!
//! Provides free (Lingva) and paid (Claude/Ollama) translation with
//! Khmer-specific syllable analysis for timing-aware dubbing.

pub mod engine;
pub mod khmer;
pub mod lingva;
pub mod llm;
pub mod translator;

pub use translator::Translator;
