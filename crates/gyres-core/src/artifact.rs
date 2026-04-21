use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::GyreError;
use crate::types::AgentId;

/// Unique identifier for an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(pub String);

/// An artifact produced by an agent during work.
///
/// Artifacts are implicit side-effects — decisions, documentation,
/// memory entries, code changes generated as the agent works. They
/// are write-heavy and searchable for RAG retrieval, enabling
/// agents to consume context they (or other agents) previously produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: ArtifactId,
    /// The agent that produced this artifact.
    pub producer: AgentId,
    /// Artifact type (e.g., "decision", "documentation", "code_change").
    pub kind: String,
    /// Human-readable title.
    pub title: String,
    /// Full content of the artifact.
    pub content: String,
    pub created_at: SystemTime,
    pub metadata: serde_json::Value,
}

/// Lightweight artifact metadata (without full content).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMeta {
    pub id: ArtifactId,
    pub producer: AgentId,
    pub kind: String,
    pub title: String,
    pub created_at: SystemTime,
}

/// Filter for artifact queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtifactFilter {
    /// Filter by artifact kind.
    pub kind: Option<String>,
    /// Filter by producing agent.
    pub producer: Option<AgentId>,
    /// Maximum number of results.
    pub limit: Option<usize>,
}

/// Artifact storage with search/RAG retrieval.
///
/// Append-heavy: agents emit artifacts as they work. Searchable:
/// artifacts become RAG context for future agent work.
///
/// Dyn-compatible for use behind `Arc<dyn ArtifactStore>` in
/// [`GyreContext`](crate::context::GyreContext).
pub trait ArtifactStore: Send + Sync {
    /// Emit a new artifact. Returns its assigned ID.
    fn emit(
        &self,
        artifact: &Artifact,
    ) -> Pin<Box<dyn Future<Output = Result<ArtifactId, GyreError>> + Send + '_>>;

    /// Search artifacts for RAG context retrieval.
    ///
    /// For backends with full-text search: ranked results.
    /// For basic backends: substring matching.
    fn search(
        &self,
        query: &str,
        filter: &ArtifactFilter,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Artifact>, GyreError>> + Send + '_>>;

    /// Get a specific artifact by ID.
    fn get(
        &self,
        id: &ArtifactId,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Artifact>, GyreError>> + Send + '_>>;

    /// List artifact metadata matching a filter.
    fn list(
        &self,
        filter: &ArtifactFilter,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ArtifactMeta>, GyreError>> + Send + '_>>;
}
