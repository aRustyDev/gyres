use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::GyreError;

/// Unique identifier for a memory entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub String);

/// A memory entry — a piece of knowledge stored by or for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: MemoryId,
    pub content: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    /// Domain tag (e.g., "user", "project", "feedback").
    pub kind: String,
    /// Arbitrary structured metadata.
    pub metadata: serde_json::Value,
}

/// Filter for memory queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryFilter {
    /// Filter by kind.
    pub kind: Option<String>,
    /// Maximum number of results.
    pub limit: Option<usize>,
}

/// Shared memory store for agent knowledge.
///
/// Supports keyword recall and basic filtering. Backends with
/// vector search or graph capabilities extend this via
/// [`GraphMemoryStore`].
///
/// Dyn-compatible for use behind `Arc<dyn MemoryStore>` in
/// [`GyreContext`](crate::context::GyreContext).
pub trait MemoryStore: Send + Sync {
    /// Store a new memory entry. Returns its assigned ID.
    fn store(
        &self,
        entry: &MemoryEntry,
    ) -> Pin<Box<dyn Future<Output = Result<MemoryId, GyreError>> + Send + '_>>;

    /// Recall memories matching a query string.
    ///
    /// For backends with vector search: semantic similarity.
    /// For basic backends: keyword/substring matching.
    fn recall(
        &self,
        query: &str,
        filter: &MemoryFilter,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<MemoryEntry>, GyreError>> + Send + '_>>;

    /// Get a specific memory entry by ID.
    fn get(
        &self,
        id: &MemoryId,
    ) -> Pin<Box<dyn Future<Output = Result<Option<MemoryEntry>, GyreError>> + Send + '_>>;

    /// Remove a memory entry.
    fn forget(
        &self,
        id: &MemoryId,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;
}

/// Relationship between two memory entries in a knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRelation {
    pub from: MemoryId,
    pub to: MemoryId,
    /// Relationship label (e.g., "related_to", "derived_from").
    pub kind: String,
}

/// Extension of [`MemoryStore`] with graph traversal capabilities.
///
/// Backends that support native graph queries (SurrealDB, Cozo)
/// implement this. Backends without graph support should not implement
/// this trait — consumers check for it via downcast or feature flags.
pub trait GraphMemoryStore: MemoryStore {
    /// Create a relationship between two memory entries.
    fn relate(
        &self,
        relation: &MemoryRelation,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    /// Traverse relationships from a starting memory entry.
    ///
    /// Returns entries reachable within `depth` hops via the given
    /// relationship kind. `None` depth means unlimited traversal.
    fn traverse(
        &self,
        from: &MemoryId,
        relation_kind: &str,
        depth: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<MemoryEntry>, GyreError>> + Send + '_>>;
}
