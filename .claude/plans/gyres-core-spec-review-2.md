# gyres-core Spec — Review Round 2

## Q: Should Action be returned by value or passed as &ref?

**By value is correct.** Here's why:

The step signature is `fn step(&self, obs: &Observation) -> Future<Result<StepResult<Action>>>`.

Action is *produced* by the agent — it didn't exist before step was called. There's no owner to borrow from. The future creates the Action and must move it out. If step returned `&Action`, it would need to store the action somewhere with a stable address, which means interior mutability + lifetime gymnastics.

Contrast with Observation, which *already exists* when step is called — the Gyre owns it and passes a reference. The agent reads it, doesn't own it.

```
Observation: exists before step() → pass by &ref (Gyre owns, Agent borrows)
Action: created during step() → return by value (Agent creates, Gyre receives ownership)
Feedback: exists before feedback() → pass by &ref (Gyre owns, Agent borrows)
```

This also explains the bounds:
- `Observation: Send + Sync` — shared reference crosses thread boundaries
- `Action: Send + Clone` — moved by value, cloned when the Gyre needs copies (telemetry, history)
- `Feedback: Send + Sync + Clone` — shared reference crosses boundaries, cloned for recording

**Does Action need Sync?** Only if someone holds `&Action` across an await point in a Send future. The Gyre receives Action by value and could pass `&action` to telemetry. If `set_attribute` is sync (it is), the reference doesn't cross an await, so Sync isn't needed. But if the Gyre wants to spawn a background task with `&action`... it would clone instead (Action: Clone). So `Action: Send + Clone` is sufficient.

**No change needed.** The current bound is correct.

---

## Re(feedback): Reflection/Reflexion strategies

You're right — feedback IS called during an LLM conversation, just not by the main agent. The architecture:

```
┌──────────────────────────────────┐
│ Gyre (ConversationGyre)          │
│                                  │
│  1. agent.step(user_msg)         │
│     → Continue(response)         │
│                                  │
│  2. Execute tool calls           │
│     → tool_results               │
│                                  │
│  3. reflector.step(response)     │  ← separate Agent
│     → score/critique             │
│                                  │
│  4. agent.feedback(score)        │  ← HERE: side-channel from reflector
│                                  │
│  5. agent.step(tool_results)     │
│     → Continue/Done              │
└──────────────────────────────────┘
```

In a Reflexion strategy:
1. The primary agent produces a response
2. A reflection agent evaluates the response quality
3. The reflection result is fed back as `feedback()` to the primary agent
4. The primary agent uses this to improve its next step

This means feedback is called *by the Gyre*, not by the agent itself. The Gyre orchestrates the reflection loop as part of its strategy. This is exactly right — the Gyre owns the loop, and different Gyre implementations can incorporate reflection, self-critique, debate, etc.

**Does this change anything?** No — the current design supports this perfectly:
- `feedback(&self, fb: &Feedback)` is sync (the reflection result is already computed)
- The Gyre orchestrates when to call feedback (after reflection)
- The primary agent's feedback handler stores the critique for use in the next step

It validates our decision to keep feedback as a first-class method rather than folding it into observations. Feedback from a reflector is semantically different from a tool result — it's meta-information about the quality of the agent's output, not input for the next action.

**One consideration:** Should `Strategy` carry the reflection configuration? e.g.:

```rust
pub struct ConversationStrategy {
    pub max_turns: usize,
    pub reflection: Option<ReflectionConfig>,  // enable/configure reflexion
}

pub struct ReflectionConfig {
    pub reflector: Arc<dyn Agent<...>>,  // the reflection agent
    pub frequency: ReflectionFrequency,   // every turn, every N turns, on tool errors
}
```

This is a gyres-llm design question, not gyres-core. But it confirms that Strategy as an associated type is the right place for this — different strategies enable different meta-cognitive patterns.

---

## Re(PermissionContext): Struct vs Enum

Currently proposed as a struct:
```rust
pub struct PermissionContext {
    pub worktree: Option<WorktreePath>,
    pub branch: Option<Branch>,
    pub extra: HashMap<String, serde_json::Value>,
}
```

Your proposal: enum with `Vec<PermissionContext>`:
```rust
pub enum PermissionContext {
    Worktree(WorktreePath),
    Branch(Branch),
    TimeOfDay(SystemTime),
    Custom { key: String, value: serde_json::Value },
    // ...
}

pub struct PermissionRequest {
    pub agent_id: AgentId,
    pub action: ActionKind,
    pub resource: Resource,
    pub context: Vec<PermissionContext>,
}
```

**The enum approach is better.** Here's why:

1. **Composable.** The Gyre builds context incrementally — add git context if available, add time context if relevant, add custom metadata. A Vec is natural for this:
```rust
let mut context = vec![];
if let Some(git) = git_context {
    context.push(PermissionContext::Worktree(git.worktree_root));
    context.push(PermissionContext::Branch(git.branch));
}
context.push(PermissionContext::AgentRole(agent.role.clone()));
```

2. **Extensible.** With `#[non_exhaustive]`, new context types can be added without breaking existing policies. Policies that don't understand a context variant simply ignore it.

3. **Polar-friendly.** Polar can iterate the list and match on variants:
```polar
allow(agent, action, resource) if
    context in request.contexts and
    context.type = "branch" and
    context.value != "main";
```

4. **No Option ceremony.** The struct has `Option<WorktreePath>`, `Option<Branch>`, etc. — most fields are optional because context varies by environment. The enum just omits what isn't present.

**Refined enum:**

```rust
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
    /// Arbitrary key-value metadata.
    Custom { key: String, value: serde_json::Value },
}
```

**Updated PermissionRequest:**
```rust
pub struct PermissionRequest {
    pub agent_id: AgentId,
    pub action: ActionKind,
    pub resource: Resource,
    pub context: Vec<PermissionContext>,
}
```

---

## Re(Root Telemetry Span): Option B confirmed

Adding `parent_span: Option<SpanId>` to GyreContext. The executor creates the root span and passes it down:

```rust
// Executor running multiple Gyres
let root = telemetry.start_span("session", None);

let span_a = telemetry.start_span("agent_a_run", Some(root));
let ctx_a = GyreContext::builder()
    .agent_id("agent-a")
    .parent_span(span_a)
    // ...
    .build();
gyre.run(&agent_a, &ctx_a, &strategy_a).await?;
telemetry.end_span(span_a);

let span_b = telemetry.start_span("agent_b_run", Some(root));
let ctx_b = GyreContext::builder()
    .agent_id("agent-b")
    .parent_span(span_b)
    // ...
    .build();
gyre.run(&agent_b, &ctx_b, &strategy_b).await?;
telemetry.end_span(span_b);

telemetry.end_span(root);
```

The Gyre uses `ctx.parent_span` as the parent for its top-level span:
```rust
async fn run(&self, agent: &A, ctx: &GyreContext, strategy: &S) -> ... {
    let run_span = ctx.telemetry.start_span("gyre_run", ctx.parent_span);
    // all child spans use Some(run_span)
    // ...
    ctx.telemetry.end_span(run_span);
}
```

---

## Re(Executor pattern): Where does it belong?

The executor is **gyres-runtime** territory, not gyres-core. But the core traits must support the executor's needs. The right sequencing:

1. **gyres-core spec** (current) — defines the traits the executor uses
2. **gyres-runtime spec** (next) — defines the executor, session lifecycle, git integration, config loading
3. **gyres-polar spec** — permission chain, approval cache, HITL
4. **gyres-llm spec** — LLM agent, conversation gyre, tool registry, providers
5. **gyres-tracing spec** — telemetry backends, tracing bridge
6. **gyres-mcp spec** — MCP protocol, tool bridge

The executor pattern should be spec'd in the gyres-runtime phase. What we need to verify now is that the gyres-core traits support it. Let me check:

**Executor needs from gyres-core:**
- `Agent::reset(&mut self)` — exclusive borrow between runs ✓
- `GyreContext` with session_id, parent_span — for session resumption and span nesting ✓
- `Gyre::run(&self, &A, &GyreContext, &Strategy)` — all shared refs during run ✓
- `StateStore::save_session` / `load_session` — session persistence ✓
- `TelemetrySink::start_span(None)` — create root spans ✓
- `Config` — load from `~/.agents/` ✓

**The borrow lifecycle the executor manages:**
```
agent: &mut Agent       ← executor owns mutably
    ↓ lend as &Agent
gyre.run(&agent, ...)   ← borrows agent immutably
    ↓ run completes, borrow released
agent.reset()            ← executor uses &mut again
```

This works because Rust's borrow checker ensures the immutable borrow from `run` is released before `reset` takes the mutable borrow. The executor never holds both simultaneously.

**Conclusion:** gyres-core supports the executor. Spec the executor in the gyres-runtime phase.

---

## Q: Should we rename DomainTurn → Turn?

**Yes.** `DomainTurn` is redundant — the trait is in `gyres-core`, and the "Domain" prefix adds noise without clarity. `Turn` is the natural name.

```rust
// gyres-core
pub trait Turn: Send + Clone {
    const DOMAIN: &'static str;
    fn serialize(&self) -> SerializedTurn;
    fn deserialize(turn: &SerializedTurn) -> Result<Self, GyreError> where Self: Sized;
}
```

Users write:
```rust
impl Turn for LlmTurn { ... }
impl Turn for RlTurn { ... }
```

`SerializedTurn` keeps its name because it describes a specific representation (the serialized form), not a trait.

One naming concern: `Turn` as a trait name could collide with `Turn` as a struct name. But `LlmTurn`, `RlTurn` etc. are the struct names — nobody would name their struct just `Turn`. And the trait is imported as `gyres_core::Turn` which is unambiguous.

**Rename confirmed: `DomainTurn` → `Turn`.**

---

## Cascading effect updates

### From Inconsistency 1 (Error bounds): Update 1A

**Before:** `Error: Send + std::fmt::Display`
**After:** `Error: std::error::Error + Send + Sync + 'static`

Ripple: None beyond what 7A already established. GyreError::Agent box works with the Sync bound.

### From Inconsistency 2 (Observation/Feedback Sync): Update 1A

**Before:** `Observation: Send`, `Feedback: Send + Clone`
**After:** `Observation: Send + Sync`, `Feedback: Send + Sync + Clone`

Ripple to plan Task 1 (gyres-core types):
- The test file `traits.rs` uses `String` as Observation. `String: Sync` ✓ — no test changes needed.
- `EchoAgent` uses `String` for Observation. ✓
- Any custom Observation type must be Sync. This is almost always true (most types are Sync unless they contain `Rc`, `Cell`, or raw pointers).

### From Gap 1 (SessionId on GyreContext): Update 3A, 3C

**GyreContext gains two new fields:**
```rust
pub session_id: Option<SessionId>,
pub parent_span: Option<SpanId>,
```

Ripple to plan Tasks 1 and 11:
- Test code that constructs GyreContext needs these fields. Both are Option, so `None` works for tests.
- Builder (3C) needs `.session_id()` and `.parent_span()` methods.

### From PermissionContext enum: Update 4C

**Before:** `HashMap<ContextType, String>`
**After:** `Vec<PermissionContext>` where PermissionContext is `#[non_exhaustive]` enum

Ripple to plan Task 2 (PermissionRequest fields):
- Tests that construct PermissionRequest need `context: vec![]` or `context: vec![PermissionContext::Branch(...)]`
- The PermissionChain in gyres-polar needs to iterate context entries

### From DomainTurn → Turn rename: Update 5B

**Before:** `pub trait DomainTurn: Send + Clone`
**After:** `pub trait Turn: Send + Clone`

Ripple: naming only. No structural change.

---

## Final corrected complete trait signatures

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

---

## Final decision table (all cascading effects applied)

| # | Decision | Status |
|---|---|---|
| 1A | `Observation: Send + Sync`, `Action: Send + Clone`, `Feedback: Send + Sync + Clone`, `Error: std::error::Error + Send + Sync + 'static` | **Updated** (Sync added, Error corrected) |
| 1B | `feedback` is sync | Confirmed |
| 1C | `StepResult<A>` with `Continue(A)` and `Done(A)` | Confirmed |
| 1D | `fn reset(&mut self) -> Result<(), Self::Error>` | Confirmed |
| 1E | `&self` on step/feedback, `&mut self` on reset. `step_batch` default impl. `Agent: Send + Sync`. | Confirmed |
| 2A | No hooks, no middleware. Gyre minimal. | Confirmed |
| 2B | `Outcome: Send + Clone` | Confirmed |
| 2C | `&GyreContext` (immutable) | Confirmed |
| 3A | GyreContext carries `agent_id`, `session_id`, `parent_span` | **Updated** (session_id + parent_span added) |
| 3B | `Gyre: Send + Sync`, `&self` on run. `type Strategy: Send + Sync`. | Confirmed |
| 3C | Builder pattern for GyreContext (now 7 fields) | **Updated** (field count) |
| 4A | `ActionKind #[non_exhaustive]`: Read, Write, Execute, Network, Spawn, Other | Confirmed |
| 4B | `PathBuf` for Resource::File paths | Confirmed |
| 4C | `PermissionContext` as `#[non_exhaustive]` enum. `PermissionRequest::context: Vec<PermissionContext>` | **Updated** (enum, not struct) |
| 5A | `append_turn` with default read-modify-write impl taking `&SerializedTurn` | Confirmed |
| 5B | `Turn` trait (renamed from DomainTurn) + `SerializedTurn` | **Updated** (renamed) |
| 5C | No session versioning for MVP | Confirmed |
| 6A | Buffer internally, drop on overflow | Confirmed |
| 6B | Span-based API: `start_span`, `end_span`, `set_attribute`, `record_event` | Confirmed |
| 6C | gyres-core defines TelemetrySink; gyres-tracing bridges to `tracing` crate | Confirmed |
| 7A | `Agent::Error: std::error::Error + Send + Sync + 'static`, boxed in `GyreError::Agent` | Confirmed |
| NEW | Feedback is for side-channel signals (reflection, scores, rewards), not tool results | **Documented** |
| NEW | Executor spec belongs in gyres-runtime phase (next), core traits verified to support it | **Documented** |
