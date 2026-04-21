# gyres-core Specification

> Version: 0.1.0-draft
> Status: Draft
> Last Updated: 2026-04-20

## Overview

`gyres-core` defines the domain-agnostic abstractions for the Gyres agent harness. It contains no LLM, RL, or domain-specific logic. All domain concerns are expressed through generic associated types on the core traits.

## Non-Goals

- Core has no opinion on LLM vs RL vs any other domain
- Core does not depend on the `tracing` crate (bridging lives in `gyres-tracing`)
- Core does not define specific strategies, tools, or providers
- Core does not define the executor (that's `gyres-runtime`)

---

## Types

### Identity Types (`types.rs`)

| Type | Wraps | Semantics |
|---|---|---|
| `AgentId` | `String` | Unique identifier for an agent instance |
| `WorktreePath` | `PathBuf` | Absolute path to a git worktree |
| `Branch` | `String` | Git branch name |
| `CommitHash` | `String` | Full 40-char hex git commit hash |

All implement `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`.

### StepResult<A>

```rust
pub enum StepResult<A> {
    Continue(A),  // action produced, loop continues
    Done(A),      // final action, agent signals completion
}
```

Convenience methods: `action() -> &A`, `into_action() -> A`, `is_done() -> bool`.

### Config (`config.rs`)

```rust
pub struct Config {
    pub flush_interval: Duration,   // default: 5s
    pub max_queue_size: usize,      // default: 100,000
    pub agents_dir: PathBuf,        // default: ~/.agents/
}
```

Implements `Default`, `Serialize`, `Deserialize`.

---

## Traits

### Agent

The core agent abstraction. Domain-specific implementations define what observations, actions, and feedback mean.

```rust
pub trait Agent: Send + Sync {
    type Observation: Send + Sync;
    type Action: Send + Clone;
    type Feedback: Send + Sync + Clone;
    type Error: std::error::Error + Send + Sync + 'static;

    fn step(&self, obs: &Self::Observation)
        -> impl Future<Output = Result<StepResult<Self::Action>, Self::Error>> + Send;

    fn step_batch(&self, observations: &[Self::Observation])
        -> impl Future<Output = Result<Vec<StepResult<Self::Action>>, Self::Error>> + Send;

    fn feedback(&self, fb: &Self::Feedback);

    fn reset(&mut self) -> Result<(), Self::Error>;
}
```

**Contracts:**

- `step` and `feedback` take `&self`. Implementations use interior mutability (`RwLock`, `Mutex`) for mutable state. This enables concurrent and batched execution.
- `reset` takes `&mut self`. The compiler guarantees no steps are in flight during reset.
- `step_batch` has a default implementation that calls `step` sequentially. Override for GPU-batched or vectorized execution.
- `feedback` is synchronous. Buffer expensive processing and defer to the next `step` call.
- **Observations** are inputs that drive the next action (user messages, tool results, environment states).
- **Feedback** is side-channel signals that inform but don't drive (scores, rewards, reflection critiques).

**Bound rationale:**

| Type | Bound | Why |
|---|---|---|
| Observation | `Send + Sync` | Passed as `&ref` to `&self` method; `&T: Send` requires `T: Sync` |
| Action | `Send + Clone` | Returned by value; cloned for telemetry/history |
| Feedback | `Send + Sync + Clone` | Passed as `&ref` to `&self` method; cloned for recording |
| Error | `Error + Send + Sync + 'static` | Boxed into `GyreError::Agent(Box<dyn Error + Send + Sync>)` |

### Gyre

The feedback loop driver. Owns the execution strategy for a specific agent domain.

```rust
pub trait Gyre<A: Agent>: Send + Sync {
    type Outcome: Send + Clone;
    type Strategy: Send + Sync;

    fn run(&self, agent: &A, ctx: &GyreContext, strategy: &Self::Strategy)
        -> impl Future<Output = Result<Self::Outcome, GyreError>> + Send;
}
```

**Contracts:**

- `run` takes `&self`. The Gyre is stateless — it's a strategy definition, not a stateful executor. Use interior mutability for rare cases needing mutable Gyre state.
- `Strategy` is per-run configuration passed by `&ref`. The caller mutates it between runs for adaptive behavior (e.g., epsilon decay in RL exploration). Use `()` when no strategy is needed.
- The Gyre calls `agent.step()` and `agent.feedback()` (both `&self`). It does NOT call `agent.reset()` — that's the executor's responsibility between runs.
- The Gyre uses `ctx.permissions`, `ctx.telemetry`, and `ctx.state` for cross-cutting concerns inside the loop.
- The Gyre creates telemetry spans using `ctx.parent_span` as the parent for its root span.

### PermissionGate

A single gate in the permission filter chain.

```rust
pub trait PermissionGate: Send + Sync {
    fn evaluate(&self, request: &PermissionRequest)
        -> Pin<Box<dyn Future<Output = Verdict> + Send + '_>>;
}
```

Dyn-compatible for use as `Arc<dyn PermissionGate>` in `GyreContext`.

### StateStore

Session and state persistence. Implementations are worktree-aware.

```rust
pub trait StateStore: Send + Sync {
    fn save_session(&self, id: &SessionId, state: &SessionState)
        -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    fn load_session(&self, id: &SessionId)
        -> Pin<Box<dyn Future<Output = Result<Option<SessionState>, GyreError>> + Send + '_>>;

    fn append_turn(&self, id: &SessionId, turn: &SerializedTurn)
        -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    fn list_sessions(&self)
        -> Pin<Box<dyn Future<Output = Result<Vec<SessionMeta>, GyreError>> + Send + '_>>;
}
```

`append_turn` has a default implementation that loads, appends, and saves. Override for efficient append-only backends (SQLite).

Dyn-compatible for use as `Arc<dyn StateStore>` in `GyreContext`.

### TelemetrySink

Span-based telemetry. Always present in `GyreContext`, even if no-op.

```rust
pub type SpanId = u64;

pub trait TelemetrySink: Send + Sync {
    fn start_span(&self, name: &str, parent: Option<SpanId>) -> SpanId;
    fn end_span(&self, id: SpanId);
    fn set_attribute(&self, span: SpanId, key: &str, value: &str);
    fn record_event(&self, span: SpanId, event: &str);
    fn flush(&self);
}
```

**Contracts:**

- `start_span` returns a `SpanId` used to reference the span in subsequent calls.
- Parent-child relationships are established at span creation via `parent: Option<SpanId>`.
- Implementations buffer internally and drop on overflow (match Langfuse SDK behavior).
- `NoopTelemetry` returns `0` for all spans and ignores all calls.

Dyn-compatible for use as `Arc<dyn TelemetrySink>` in `GyreContext`.

### Turn

Bridge between typed domain turns and the type-erased persistence layer.

```rust
pub trait Turn: Send + Clone {
    const DOMAIN: &'static str;
    fn serialize(&self) -> SerializedTurn;
    fn deserialize(turn: &SerializedTurn) -> Result<Self, GyreError> where Self: Sized;
}
```

Domain crates implement this for their typed turns (e.g., `LlmTurn`, `RlTurn`). `SerializedTurn` uses `serde_json::Value` for the observation/action/feedback fields.

---

## Structs

### GyreContext

Shared infrastructure available to every Gyre implementation.

```rust
#[derive(Clone)]
pub struct GyreContext {
    pub agent_id: AgentId,
    pub session_id: Option<SessionId>,
    pub parent_span: Option<SpanId>,
    pub permissions: Arc<dyn PermissionGate>,
    pub state: Arc<dyn StateStore>,
    pub config: Arc<Config>,
    pub telemetry: Arc<dyn TelemetrySink>,
}
```

All fields behind `Arc` — cloning is cheap. `Clone` enables forking contexts for sub-agents.

Constructed via builder: `GyreContext::builder().agent_id(...).permissions(...).build()`.

### PermissionRequest

```rust
pub struct PermissionRequest {
    pub agent_id: AgentId,
    pub action: ActionKind,
    pub resource: Resource,
    pub context: Vec<PermissionContext>,
}
```

### ActionKind

```rust
#[non_exhaustive]
pub enum ActionKind {
    Read { tool: String },
    Write { tool: String },
    Execute { tool: String, input: String },
    Network { tool: String, url: String },
    Spawn { agent: String, prompt: String, cache: String, tools: String },
    Other { kind: String, tool: String, args: Vec<String> },
}
```

`is_write() -> bool` returns true for `Write`, `Execute`, `Spawn`.

### Resource

```rust
pub enum Resource {
    None,
    File { path: PathBuf },
    Url { url: String },
}
```

### PermissionContext

```rust
#[non_exhaustive]
pub enum PermissionContext {
    Worktree(WorktreePath),
    Branch(Branch),
    Commit(CommitHash),
    IsMainWorktree(bool),
    AgentRole(String),
    Custom { key: String, value: serde_json::Value },
}
```

### Verdict

```rust
pub enum Verdict {
    Allow,
    Deny(String),
    Defer,
}
```

### SerializedTurn

```rust
pub struct SerializedTurn {
    pub timestamp: SystemTime,
    pub domain: String,
    pub observation: serde_json::Value,
    pub action: serde_json::Value,
    pub feedback: Option<serde_json::Value>,
}
```

### SessionState

```rust
pub struct SessionState {
    pub id: SessionId,
    pub created_at: SystemTime,
    pub last_active: SystemTime,
    pub turns: Vec<SerializedTurn>,
    pub worktree: Option<String>,
    pub metadata: serde_json::Value,
}
```

### GyreError

```rust
pub enum GyreError {
    PermissionDenied(String),
    Agent(Box<dyn std::error::Error + Send + Sync>),
    State(String),
    Telemetry(String),
    Timeout(Duration),
    Other(Box<dyn std::error::Error + Send + Sync>),
}
```

`Agent` variant preserves the original error for downcasting.
