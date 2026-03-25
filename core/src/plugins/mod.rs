/// Plugin engine — manifest-based discovery, lifecycle management, and hook system.
/// Inspired by PocketPaw's extension platform but implemented in Rust.

pub mod engine;
pub mod hooks;
pub mod manifest;

pub use engine::{PluginEngine, PluginLogEntry};
pub use hooks::{HookContext, HookResult, HookType};
pub use manifest::{PluginManifest, PluginType};
