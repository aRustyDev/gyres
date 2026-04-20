use std::future::Future;
use std::pin::Pin;

use crate::error::GyreError;

/// Unique identifier for a session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

/// Metadata about a session (without full state).
#[derive(Debug, Clone)]
pub struct SessionMeta {
    pub id: SessionId,
    // TODO: worktree path, branch, created_at, last_active
}

/// Serializable session state.
#[derive(Debug, Clone)]
pub struct SessionState {
    // TODO: conversation history, agent state, metadata
}

/// Trait for session and state persistence.
/// Implementations are worktree-aware — the store knows which
/// worktree it's operating in.
/// Dyn-compatible for use behind `Arc<dyn StateStore>` in GyreContext.
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

    fn list_sessions(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<SessionMeta>, GyreError>> + Send + '_>>;
}
