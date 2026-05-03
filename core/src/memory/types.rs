//! Local type definitions for the memory substrate.
//!
//! These replace the `openfang_types::*` imports from the upstream crate,
//! keeping NDE-OS self-contained with zero external type dependencies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ID newtypes
// ---------------------------------------------------------------------------

/// Unique identifier for an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub uuid::Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub uuid::Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a memory fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub uuid::Uuid);

impl MemoryId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

// ---------------------------------------------------------------------------
// Memory types
// ---------------------------------------------------------------------------

/// Source of a memory fragment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    Conversation,
    System,
    Tool,
    User,
}

/// Filter for memory recall queries.
#[derive(Debug, Clone, Default)]
pub struct MemoryFilter {
    pub agent_id: Option<AgentId>,
    pub scope: Option<String>,
    pub min_confidence: Option<f32>,
    pub source: Option<MemorySource>,
}

impl MemoryFilter {
    /// Create a filter scoped to a specific agent.
    pub fn agent(agent_id: AgentId) -> Self {
        Self {
            agent_id: Some(agent_id),
            ..Default::default()
        }
    }
}

/// A recalled memory fragment with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFragment {
    pub id: MemoryId,
    pub agent_id: AgentId,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub source: MemorySource,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub access_count: u64,
    pub scope: String,
}

/// Report from a consolidation cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationReport {
    pub memories_merged: u64,
    pub memories_decayed: u64,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Knowledge graph types
// ---------------------------------------------------------------------------

/// Type of a knowledge graph entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Concept,
    Event,
    Custom(String),
}

/// Type of a knowledge graph relation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    WorksAt,
    Knows,
    IsA,
    HasA,
    PartOf,
    LocatedIn,
    RelatedTo,
    Custom(String),
}

/// A knowledge graph entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_type: EntityType,
    pub name: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A knowledge graph relation between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub source: String,
    pub relation: RelationType,
    pub target: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
}

/// A pattern for querying the knowledge graph.
#[derive(Debug, Clone)]
pub struct GraphPattern {
    pub source: Option<String>,
    pub relation: Option<RelationType>,
    pub target: Option<String>,
    pub max_depth: u32,
}

/// A single match from a graph query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMatch {
    pub source: Entity,
    pub relation: Relation,
    pub target: Entity,
}

// ---------------------------------------------------------------------------
// Session types
// ---------------------------------------------------------------------------

/// A single message in a conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
}

/// Role of a message sender.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
}

// ---------------------------------------------------------------------------
// Usage types
// ---------------------------------------------------------------------------

/// A single LLM usage event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub agent_id: AgentId,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub tool_calls: u32,
}

/// Summary of usage over a period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub call_count: u64,
    pub total_tool_calls: u64,
}

/// Usage grouped by model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub total_cost_usd: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub call_count: u64,
}

/// Daily usage breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyBreakdown {
    pub date: String,
    pub cost_usd: f64,
    pub tokens: u64,
    pub calls: u64,
}

/// Truncate a string to `max` chars (UTF-8 safe).
pub fn truncate_str(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
