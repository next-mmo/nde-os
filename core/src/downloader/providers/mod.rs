//! Provider implementations.
//!
//! Add a new provider by creating a `foo.rs` module, implementing
//! [`super::MediaProvider`], and pushing an instance into
//! [`default_providers`].

use std::sync::Arc;

use super::provider::MediaProvider;

pub mod short_drama;

/// All built-in providers. Router resolves the first match.
pub fn default_providers() -> Vec<Arc<dyn MediaProvider>> {
    vec![Arc::new(short_drama::ShortDramaProvider::new())]
}

/// Find the provider whose `matches(url)` returns true, or `None`.
pub fn resolve_for_url(url: &str) -> Option<Arc<dyn MediaProvider>> {
    default_providers().into_iter().find(|p| p.matches(url))
}
