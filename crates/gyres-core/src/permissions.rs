use std::future::Future;
use std::pin::Pin;

use crate::types::{AgentId, Branch, CommitHash, WorktreePath};

/// What kind of action is being requested.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ActionKind {
    /// Read-only tool invocation (glob, grep, read).
    Read { tool: String },
    /// Write tool invocation (write, edit).
    Write { tool: String },
    /// Execution tool invocation (bash).
    Execute { tool: String, input: String },
    /// Network access (fetching URLs, calling APIs).
    Network { tool: String, url: String },
    /// Spawning a sub-agent.
    Spawn {
        agent: String,
        prompt: String,
        cache: String,
        tools: String,
    },
    /// Any other action not covered above.
    Other {
        kind: String,
        tool: String,
        args: Vec<String>,
    },
}

impl ActionKind {
    /// Returns true if this action modifies state.
    pub fn is_write(&self) -> bool {
        matches!(
            self,
            ActionKind::Write { .. } | ActionKind::Execute { .. } | ActionKind::Spawn { .. }
        )
    }

    /// The tool name for this action.
    pub fn tool(&self) -> &str {
        match self {
            ActionKind::Read { tool }
            | ActionKind::Write { tool }
            | ActionKind::Execute { tool, .. }
            | ActionKind::Network { tool, .. }
            | ActionKind::Other { tool, .. } => tool,
            ActionKind::Spawn { agent, .. } => agent,
        }
    }
}

/// The resource being acted upon.
#[derive(Debug, Clone)]
pub enum Resource {
    /// No specific resource.
    None,
    /// A file path.
    File { path: std::path::PathBuf },
    /// A URL.
    Url { url: String },
}

/// Environmental context for permission evaluation.
///
/// Composed as a `Vec<PermissionContext>` on [`PermissionRequest`].
/// The Gyre builds context incrementally — only adding what's available.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum PermissionContext {
    /// Current git worktree path.
    Worktree(WorktreePath),
    /// Current git branch.
    Branch(Branch),
    /// Current commit hash.
    Commit(CommitHash),
    /// Whether this is the main/primary worktree.
    IsMainWorktree(bool),
    /// Agent's declared role.
    AgentRole(String),
    /// Arbitrary key-value metadata for custom policies.
    Custom {
        key: String,
        value: serde_json::Value,
    },
}

/// A request to perform an action, evaluated by the permission chain.
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    /// Which agent is making the request.
    pub agent_id: AgentId,
    /// What action is being requested.
    pub action: ActionKind,
    /// What resource is being acted upon.
    pub resource: Resource,
    /// Environmental context for policy evaluation.
    pub context: Vec<PermissionContext>,
}

/// The result of evaluating a permission request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Action is allowed.
    Allow,
    /// Action is denied with a reason.
    Deny(String),
    /// This gate has no opinion — pass to the next gate in the chain.
    Defer,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verdict::Allow => write!(f, "allow"),
            Verdict::Deny(reason) => write!(f, "deny: {reason}"),
            Verdict::Defer => write!(f, "defer"),
        }
    }
}

/// A single gate in the permission filter chain.
///
/// Dyn-compatible for use behind `Arc<dyn PermissionGate>` in
/// [`GyreContext`](crate::context::GyreContext).
pub trait PermissionGate: Send + Sync {
    /// Evaluate a permission request.
    fn evaluate(
        &self,
        request: &PermissionRequest,
    ) -> Pin<Box<dyn Future<Output = Verdict> + Send + '_>>;
}
