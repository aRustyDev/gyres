use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::GyreError;

/// Unique identifier for a session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

/// Type-erased turn for persistence. Domain crates convert to/from
/// their typed turns via the [`Turn`] trait.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedTurn {
    pub timestamp: SystemTime,
    /// Domain identifier (e.g., "llm", "rl").
    pub domain: String,
    pub observation: serde_json::Value,
    pub action: serde_json::Value,
    pub feedback: Option<serde_json::Value>,
}

/// Bridge between domain-specific typed turns and type-erased
/// [`SerializedTurn`] for persistence.
///
/// Domain crates implement this for their typed turns (e.g., `LlmTurn`).
/// The [`StateStore`] operates on `SerializedTurn`; the Gyre works with
/// typed turns internally and serializes at the persistence boundary.
pub trait Turn: Send + Clone {
    /// Domain identifier, used to dispatch deserialization.
    const DOMAIN: &'static str;

    /// Convert this typed turn to a type-erased [`SerializedTurn`].
    fn serialize(&self) -> SerializedTurn;

    /// Reconstruct a typed turn from a [`SerializedTurn`].
    fn deserialize(turn: &SerializedTurn) -> Result<Self, GyreError>
    where
        Self: Sized;
}

/// Metadata about a session (without full state).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: SessionId,
    pub created_at: SystemTime,
    pub last_active: SystemTime,
    pub turn_count: usize,
    pub worktree: Option<String>,
}

/// Serializable session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub id: SessionId,
    pub created_at: SystemTime,
    pub last_active: SystemTime,
    pub turns: Vec<SerializedTurn>,
    pub worktree: Option<String>,
    /// Arbitrary domain-specific metadata.
    pub metadata: serde_json::Value,
}

impl SessionState {
    pub fn new(id: SessionId) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            created_at: now,
            last_active: now,
            turns: Vec::new(),
            worktree: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn meta(&self) -> SessionMeta {
        SessionMeta {
            id: self.id.clone(),
            created_at: self.created_at,
            last_active: self.last_active,
            turn_count: self.turns.len(),
            worktree: self.worktree.clone(),
        }
    }
}

/// Session and state persistence. Implementations are worktree-aware.
///
/// Dyn-compatible for use behind `Arc<dyn StateStore>` in
/// [`GyreContext`](crate::context::GyreContext).
pub trait StateStore: Send + Sync {
    fn save_session(
        &self,
        id: &SessionId,
        state: &SessionState,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    fn load_session(
        &self,
        id: &SessionId,
    ) -> Pin<Box<dyn Future<Output = Result<Option<SessionState>, GyreError>> + Send + '_>>;

    /// Append a single turn to a session.
    ///
    /// Default implementation loads the session, appends, and saves.
    /// Override for efficient append-only backends (e.g., SQLite).
    fn append_turn(
        &self,
        id: &SessionId,
        turn: &SerializedTurn,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    fn list_sessions(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<SessionMeta>, GyreError>> + Send + '_>>;
}
