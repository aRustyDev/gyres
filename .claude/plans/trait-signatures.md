### StepResult

```rust
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

    pub fn is_done(&self) -> bool {
        matches!(self, StepResult::Done(_))
    }

    pub fn into_action(self) -> A {
        match self {
            StepResult::Continue(a) | StepResult::Done(a) => a,
        }
    }
}
```

### Agent

```rust
pub trait Agent: Send + Sync {
    type Observation: Send + Sync;
    type Action: Send + Clone;
    type Feedback: Send + Sync + Clone;
    type Error: std::error::Error + Send + Sync + 'static;

    /// Produce an action given an observation.
    fn step(
        &self,
        obs: &Self::Observation,
    ) -> impl Future<Output = Result<StepResult<Self::Action>, Self::Error>> + Send;

    /// Batch-step: produce N actions from N observations.
    /// Default calls step() sequentially. Override for GPU batching.
    fn step_batch(
        &self,
        observations: &[Self::Observation],
    ) -> impl Future<Output = Result<Vec<StepResult<Self::Action>>, Self::Error>> + Send {
        async {
            let mut results = Vec::with_capacity(observations.len());
            for obs in observations {
                results.push(self.step(obs).await?);
            }
            Ok(results)
        }
    }

    /// Side-channel feedback signal (scores, rewards, reflections).
    /// NOT for observations — tool results are the next observation, not feedback.
    /// Called by the Gyre when external evaluation is available.
    fn feedback(&self, fb: &Self::Feedback);

    /// Reset agent state for a new episode/session.
    /// &mut self enforces no concurrent steps during reset.
    fn reset(&mut self) -> Result<(), Self::Error>;
}
```

### Gyre

```rust
pub trait Gyre<A: Agent>: Send + Sync {
    /// Result of a completed run.
    type Outcome: Send + Clone;
    /// Per-run configuration. Use () if no strategy is needed.
    type Strategy: Send + Sync;

    fn run(
        &self,
        agent: &A,
        ctx: &GyreContext,
        strategy: &Self::Strategy,
    ) -> impl Future<Output = Result<Self::Outcome, GyreError>> + Send;
}
```

### GyreContext

```rust
#[derive(Clone)]
pub struct GyreContext {
    /// Identity of the agent this context is for.
    pub agent_id: AgentId,
    /// Session to resume, or None for new session.
    pub session_id: Option<SessionId>,
    /// Parent telemetry span to nest under, or None for root.
    pub parent_span: Option<SpanId>,
    /// Permission evaluation chain.
    pub permissions: Arc<dyn PermissionGate>,
    /// Session and state persistence.
    pub state: Arc<dyn StateStore>,
    /// Application configuration.
    pub config: Arc<Config>,
    /// Telemetry sink.
    pub telemetry: Arc<dyn TelemetrySink>,
}
```

### PermissionGate + types

```rust
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ActionKind {
    Read { tool: String },
    Write { tool: String },
    Execute { tool: String, input: String },
    Network { tool: String, url: String },
    Spawn { agent: String, prompt: String, cache: String, tools: String },
    Other { kind: String, tool: String, args: Vec<String> },
}

#[derive(Debug, Clone)]
pub enum Resource {
    None,
    File { path: std::path::PathBuf },
    Url { url: String },
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum PermissionContext {
    Worktree(WorktreePath),
    Branch(Branch),
    Commit(CommitHash),
    IsMainWorktree(bool),
    AgentRole(String),
    Custom { key: String, value: serde_json::Value },
}

#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub agent_id: AgentId,
    pub action: ActionKind,
    pub resource: Resource,
    pub context: Vec<PermissionContext>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Deny(String),
    Defer,
}

pub trait PermissionGate: Send + Sync {
    fn evaluate(
        &self,
        request: &PermissionRequest,
    ) -> Pin<Box<dyn Future<Output = Verdict> + Send + '_>>;
}
```

### StateStore + Turn

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedTurn {
    pub timestamp: SystemTime,
    pub domain: String,
    pub observation: serde_json::Value,
    pub action: serde_json::Value,
    pub feedback: Option<serde_json::Value>,
}

pub trait Turn: Send + Clone {
    const DOMAIN: &'static str;
    fn serialize(&self) -> SerializedTurn;
    fn deserialize(turn: &SerializedTurn) -> Result<Self, GyreError> where Self: Sized;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub id: SessionId,
    pub created_at: SystemTime,
    pub last_active: SystemTime,
    pub turns: Vec<SerializedTurn>,
    pub worktree: Option<String>,
    pub metadata: serde_json::Value,
}

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

    fn append_turn(
        &self,
        id: &SessionId,
        turn: &SerializedTurn,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>> {
        // Default: load, append, save
        let id = id.clone();
        let turn = turn.clone();
        Box::pin(async move {
            let mut state = self.load_session(&id).await?
                .unwrap_or_else(|| SessionState::new(id.clone()));
            state.turns.push(turn);
            state.last_active = SystemTime::now();
            self.save_session(&id, &state).await
        })
    }

    fn list_sessions(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<SessionMeta>, GyreError>> + Send + '_>>;
}
```

### TelemetrySink

```rust
pub type SpanId = u64;

pub trait TelemetrySink: Send + Sync {
    fn start_span(&self, name: &str, parent: Option<SpanId>) -> SpanId;
    fn end_span(&self, id: SpanId);
    fn set_attribute(&self, span: SpanId, key: &str, value: &str);
    fn record_event(&self, span: SpanId, event: &str);
    fn flush(&self);
}

pub struct NoopTelemetry;

impl TelemetrySink for NoopTelemetry {
    fn start_span(&self, _name: &str, _parent: Option<SpanId>) -> SpanId { 0 }
    fn end_span(&self, _id: SpanId) {}
    fn set_attribute(&self, _span: SpanId, _key: &str, _value: &str) {}
    fn record_event(&self, _span: SpanId, _event: &str) {}
    fn flush(&self) {}
}
```

### GyreError

```rust
#[derive(Error, Debug)]
pub enum GyreError {
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("agent error: {0}")]
    Agent(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("state store error: {0}")]
    State(String),

    #[error("telemetry error: {0}")]
    Telemetry(String),

    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
```
