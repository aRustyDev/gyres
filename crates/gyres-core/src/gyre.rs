use std::future::Future;

use crate::agent::Agent;
use crate::context::GyreContext;
use crate::error::GyreError;

/// The feedback loop driver. Owns the execution strategy for a
/// specific agent domain (LLM conversation, RL episode, etc.).
///
/// # Statelessness
///
/// `run` takes `&self` — the Gyre is a stateless strategy definition,
/// not a mutable executor. A single Gyre instance can drive multiple
/// concurrent runs with different agents.
///
/// # Strategy
///
/// [`Strategy`](Gyre::Strategy) is per-run configuration passed by `&ref`.
/// The caller mutates it between runs for adaptive behavior (e.g., epsilon
/// decay in RL exploration). Use `()` when no strategy is needed.
///
/// # Lifecycle
///
/// The Gyre calls [`Agent::step`] and [`Agent::feedback`] (both `&self`).
/// It does **not** call [`Agent::reset`] — that is the executor's
/// responsibility between runs.
pub trait Gyre<A: Agent>: Send + Sync {
    /// Result of a completed run (transcript, episode stats, etc.).
    type Outcome: Send + Clone;

    /// Per-run configuration. Use `()` if no strategy is needed.
    ///
    /// Passed by `&ref` to [`run`](Gyre::run), allowing the caller to
    /// mutate it between runs for adaptive behavior.
    type Strategy: Send + Sync;

    /// Drive the agent through one full episode or session.
    ///
    /// Uses `ctx.parent_span` as the parent for its root telemetry span.
    /// Accesses `ctx.permissions` and `ctx.telemetry` for cross-cutting
    /// concerns inside the loop.
    fn run(
        &self,
        agent: &A,
        ctx: &GyreContext,
        strategy: &Self::Strategy,
    ) -> impl Future<Output = Result<Self::Outcome, GyreError>> + Send;
}
