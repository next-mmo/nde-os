//! Figma JSON Render Engine — Core Module
//!
//! Provides Figma file → FDocument conversion, style resolution,
//! and LLM prompt generation. All heavy logic runs in Rust for
//! performance; the frontend only renders the final output.
//!
//! # Architecture
//!
//! ```text
//! figma_json/
//! ├── types.rs          — Serde structs (FDocument, FNode, fills, etc.)
//! ├── converter.rs      — Figma REST API JSON → FDocument
//! ├── style_resolver.rs — FNode → CSS inline styles
//! └── llm_prompt.rs     — Prompt templates for LLM auto-generation
//! ```
//!
//! # Future MCP Integration
//!
//! This module is designed to be exposed as MCP tools via `core/src/mcp/`.
//! Tool definitions: `convert_figma`, `resolve_styles`, `fetch_figma_file`.

pub mod converter;
pub mod llm_prompt;
pub mod style_resolver;
pub mod types;

// Re-exports for convenience
pub use converter::{convert_figma_file, convert_figma_node};
pub use llm_prompt::build_llm_prompt;
pub use style_resolver::{resolve_node_styles, styles_to_string};
pub use types::FDocument;
