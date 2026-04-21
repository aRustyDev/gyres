use std::future::Future;

/// Result of a single agent step.
///
/// The agent signals whether the loop should continue or terminate
/// by returning `Continue` or `Done`. Both variants carry an action,
/// so the final step always produces output.
#[derive(Debug, Clone)]
pub enum StepResult<A> {
    /// Action produced, loop continues.
    Continue(A),
    /// Final action, agent signals completion.
    Done(A),
}

impl<A> StepResult<A> {
    pub fn action(&self) -> &A {
        match self {
            StepResult::Continue(a) | StepResult::Done(a) => a,
        }
    }

    pub fn into_action(self) -> A {
        match self {
            StepResult::Continue(a) | StepResult::Done(a) => a,
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self, StepResult::Done(_))
    }
}

/// The core agent abstraction. Domain-specific implementations
/// (LLM, RL, etc.) define what observations, actions, and feedback mean.
///
/// # Concurrency
///
/// [`step`](Agent::step) and [`feedback`](Agent::feedback) take `&self`,
/// enabling concurrent and batched execution. Implementations must use
/// interior mutability (e.g., `RwLock`) for mutable state.
/// [`reset`](Agent::reset) takes `&mut self`, guaranteeing exclusive
/// access — the compiler enforces that no steps are in flight during reset.
///
/// # Observations vs Feedback
///
/// **Observations** (via [`step`](Agent::step)): inputs that drive the next
/// action — user messages, tool results, environment states.
///
/// **Feedback** (via [`feedback`](Agent::feedback)): side-channel signals
/// that inform but don't drive the next step — scores, rewards, reflection
/// critiques, human ratings. Called by the Gyre when external evaluation
/// is available (e.g., from a reflection agent in a Reflexion strategy).
pub trait Agent: Send + Sync {
    /// What the agent observes. Passed by `&ref` to concurrent `step` calls.
    type Observation: Send + Sync;

    /// What the agent produces. Returned by value from `step`.
    type Action: Send + Clone;

    /// Side-channel signal type. Passed by `&ref` to concurrent `feedback` calls.
    type Feedback: Send + Sync + Clone;

    /// Agent-specific error type. Boxed into `GyreError::Agent` by the Gyre.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Produce an action given an observation.
    ///
    /// Returns [`StepResult::Continue`] to keep the loop running,
    /// or [`StepResult::Done`] to signal agent-initiated completion.
    /// The Gyre may also terminate externally (timeout, max turns).
    fn step(
        &self,
        obs: &Self::Observation,
    ) -> impl Future<Output = Result<StepResult<Self::Action>, Self::Error>> + Send;

    /// Produce N actions from N observations.
    ///
    /// Default implementation calls [`step`](Agent::step) sequentially.
    /// Override for GPU-batched or vectorized execution in RL agents.
    fn step_batch(
        &self,
        observations: &[Self::Observation],
    ) -> impl Future<Output = Result<Vec<StepResult<Self::Action>>, Self::Error>> + Send {
        async move {
            let mut results = Vec::with_capacity(observations.len());
            for obs in observations {
                results.push(self.step(obs).await?);
            }
            Ok(results)
        }
    }

    /// Receive a side-channel feedback signal.
    ///
    /// Synchronous — buffer expensive processing and defer to the next
    /// [`step`](Agent::step) call. Called by the Gyre when external
    /// evaluation is available (reflection, scoring, reward signals).
    fn feedback(&self, fb: &Self::Feedback);

    /// Reset agent state for a new episode or session.
    ///
    /// `&mut self` guarantees exclusive access — the compiler enforces
    /// that no concurrent steps are in flight during reset.
    fn reset(&mut self) -> Result<(), Self::Error>;
}
