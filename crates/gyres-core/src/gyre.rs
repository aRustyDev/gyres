use std::future::Future;

use crate::agent::Agent;
use crate::context::GyreContext;
use crate::error::GyreError;

/// The feedback loop driver. Owns the execution strategy for a
/// specific agent domain (LLM conversation loop, RL episode, etc.)
pub trait Gyre<A: Agent>: Send {
    /// The result of a completed run (transcript, episode stats, etc.)
    type Outcome;

    /// Drive the agent through one full episode/session.
    fn run(
        &mut self,
        agent: &mut A,
        ctx: &mut GyreContext,
    ) -> impl Future<Output = Result<Self::Outcome, GyreError>> + Send;
}
