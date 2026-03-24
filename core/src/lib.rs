pub mod app_manager;
pub mod manifest;
pub mod sandbox;
pub mod system_metrics;
pub mod uv_env;

// Agent runtime
pub mod agent;
pub mod llm;
pub mod tools;
pub mod channels;
pub mod skills;
pub mod knowledge;
pub mod memory;
pub mod security;

// Phase 2
pub mod plugins;
pub mod mcp;

// Shield Browser (anti-detect)
pub mod shield;

#[cfg(test)]
mod tests;
