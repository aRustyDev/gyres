use std::future::Future;

/// The core agent abstraction. Domain-specific (LLM, RL, etc.)
/// implementations define what observations, actions, and feedback mean.
pub trait Agent: Send {
    /// What the agent observes (user message, env state, tool result, etc.)
    type Observation;
    /// What the agent produces (assistant response, action, etc.)
    type Action;
    /// Feedback signal (tool result, reward, score, etc.)
    type Feedback;
    /// Agent-specific error type.
    type Error;

    /// Produce an action given an observation.
    fn step(
        &mut self,
        obs: &Self::Observation,
    ) -> impl Future<Output = Result<Self::Action, Self::Error>> + Send;

    /// Receive feedback from the last action.
    fn feedback(&mut self, fb: &Self::Feedback);

    /// Agent-driven termination signal.
    /// The Gyre may also terminate externally (timeout, error, env done).
    fn should_stop(&self) -> bool;
}
