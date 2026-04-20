use std::sync::Arc;

use crate::permissions::PermissionGate;
use crate::state::StateStore;
use crate::telemetry::TelemetrySink;

/// Shared infrastructure available to every Gyre implementation.
/// Kept thin — domain-specific concerns live behind trait interfaces.
pub struct GyreContext {
    /// Permission evaluation chain (approval cache accessed through this).
    pub permissions: Arc<dyn PermissionGate>,
    /// Session and state persistence (git/worktree context accessed through this).
    pub state: Arc<dyn StateStore>,
    /// Application configuration.
    pub config: Arc<Config>,
    /// Telemetry sink — always present, even if no-op.
    pub telemetry: Arc<dyn TelemetrySink>,
}

/// Top-level harness configuration.
#[derive(Debug, Clone)]
pub struct Config {
    // TODO: define config fields
}
