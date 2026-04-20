pub mod app_manager;
pub mod events;
pub mod manifest;
pub mod media;
pub mod node_env;
pub mod sandbox;
pub mod system_metrics;
pub mod uv_env;

// Agent runtime
pub mod agent;
pub mod channels;
pub mod knowledge;
pub mod llm;
pub mod memory;
pub mod secrets;
pub mod security;
pub mod skills;
pub mod tools;

// Phase 2
pub mod mcp;
pub mod plugins;

// OpenViking context database integration
pub mod openviking;

// Figma JSON Render Engine
pub mod figma_json;

// Shield Browser (anti-detect)
pub mod shield;

// Shield Actor System (Apify-compatible browser automation)
pub mod actor;

// FreeCut video editor engine
pub mod freecut;

// Khmer Forced Aligner (KFA) — native Rust wav2vec2 CTC alignment engine
pub mod kfa;

// Download Center — provider-pluggable media downloader
pub mod downloader;

// Global voice services (Edge TTS + RVC)
pub mod voice;

// Global service registry (onboarding)
pub mod services;

#[cfg(feature = "screenshot")]
pub mod screenshot;

#[cfg(test)]
mod tests;
