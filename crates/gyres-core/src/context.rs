use std::sync::Arc;

use crate::config::Config;
use crate::permissions::PermissionGate;
use crate::state::{SessionId, StateStore};
use crate::telemetry::{SpanId, TelemetrySink};
use crate::types::AgentId;

/// Shared infrastructure available to every [`Gyre`](crate::gyre::Gyre).
///
/// All fields behind `Arc` — cloning is cheap and enables forking
/// contexts for sub-agents. Takes `&GyreContext` (immutable) in
/// [`Gyre::run`](crate::gyre::Gyre::run); `Arc` fields handle
/// their own interior mutability.
///
/// Constructed via [`GyreContext::builder()`].
#[derive(Clone)]
pub struct GyreContext {
    /// Identity of the agent this context is for.
    pub agent_id: AgentId,
    /// Session to resume, or `None` for a new session.
    pub session_id: Option<SessionId>,
    /// Parent telemetry span to nest under, or `None` for a root span.
    ///
    /// When the executor runs multiple Gyres, it creates a root span
    /// and passes child spans here so all runs appear in a single trace.
    pub parent_span: Option<SpanId>,
    /// Permission evaluation chain (approval cache accessed through this).
    pub permissions: Arc<dyn PermissionGate>,
    /// Session and state persistence (worktree-aware).
    pub state: Arc<dyn StateStore>,
    /// Application configuration.
    pub config: Arc<Config>,
    /// Telemetry sink — always present, even if no-op.
    pub telemetry: Arc<dyn TelemetrySink>,
}
