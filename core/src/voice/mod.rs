//! NDE-OS Voice Services — global TTS (Edge TTS) + RVC voice conversion.
//!
//! This module provides framework-agnostic voice services that any NDE-OS app
//! can consume. The runtime is managed centrally at `~/.ai-launcher/voice-runtime/`.

pub mod runtime;
pub mod rvc;
pub mod tts;
pub mod types;
