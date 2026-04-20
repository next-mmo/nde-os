//! KFA — Khmer Forced Aligner core.
//!
//! Native Rust port of the Python KFA project by Seanghay Yath (@seanghay).
//! Provides word-level timestamp alignment for Khmer audio using a
//! wav2vec2-CTC ONNX model + Phonetisaurus G2P FST.
//!
//! ## Usage
//! ```rust,no_run
//! use ai_launcher_core::kfa::{AlignmentSession, Alignment};
//! use ai_launcher_core::kfa::audio::load_wav_mono_16k_bytes;
//!
//! let wav_bytes = std::fs::read("audio.wav").unwrap();
//! let (samples, sr) = load_wav_mono_16k_bytes(&wav_bytes).unwrap();
//!
//! let mut session = AlignmentSession::new(false).unwrap();
//! let text = "ការប្រើប្រាស់បច្ចេកវិទ្យា";
//! let alignments = session.align(&samples, sr, text, None).unwrap();
//! for seg in &alignments {
//!     println!("{:.2}–{:.2}  {}", seg.start, seg.end, seg.text);
//! }
//! ```

pub mod alignment_utils;
pub mod audio;
pub mod g2p;
pub mod lexicon;
pub mod normalizer;
pub mod number_verbalize;
pub mod session;
pub mod text_normalize;
pub mod vocab;

pub use session::{Alignment, AlignmentSession, SAMPLE_RATE};
