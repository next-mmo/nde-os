//! Types for the NDE-OS service registry.

use serde::{Deserialize, Serialize};

/// A service category grouping.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceGroup {
    Voice,
    Media,
    Ai,
    Tooling,
}

/// Definition of a registerable NDE-OS service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub group: ServiceGroup,
    /// Which apps use this service (display only).
    #[serde(default)]
    pub used_by: Vec<String>,
    /// Whether users can skip installing this.
    #[serde(default = "default_true")]
    pub optional: bool,
}

fn default_true() -> bool {
    true
}

/// Live status of a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub id: String,
    pub name: String,
    pub description: String,
    pub group: ServiceGroup,
    pub installed: bool,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub used_by: Vec<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub details: Option<String>,
}
