# gyres-core Spec — Holistic Review

## Complete decision set (for reference)

| # | Decision |
|---|---|
| 1A | `Observation: Send + Sync`, `Action: Send + Clone`, `Feedback: Send + Clone` |
| 1B | `feedback` is sync |
| 1C | `StepResult<A>` replaces `should_stop` |
| 1D | `fn reset(&mut self) -> Result<(), Self::Error>` |
| 1E | `Agent: Send + Sync`. `&self` on step/feedback. `&mut self` on reset. `step_batch` with default sequential impl. |
| 2A | No hooks, no middleware. Gyre minimal. |
| 2B | `Outcome: Send + Clone` |
| 2C | `&GyreContext` (immutable) |
| 3A | GyreContext carries AgentId |
| 3B | `Gyre: Send + Sync`. `&self` on run. Strategy as associated type. |
| 3C | Builder pattern for GyreContext |
| 4A | `ActionKind`: `#[non_exhaustive]` with Read, Write, Execute, Network, Spawn, Other |
| 4B | `PathBuf` for Resource paths |
| 4C | `PermissionContext` as separate type on request |
| 5A | `append_turn` on StateStore with default read-modify-write impl |
| 5B | `DomainTurn` trait + `SerializedTurn` (Approach 3) |
| 5C | No session versioning for MVP |
| 6A | Telemetry: buffer internally, drop on overflow |
| 6B | Span-based telemetry API (start_span/end_span with parent SpanId) |
| 6C | gyres-core defines TelemetrySink; gyres-tracing bridges to `tracing` crate |
| 7A | `Agent::Error: std::error::Error + Send + Sync + 'static`, boxed in GyreError |

---

## Inconsistency 1: Agent::Error bound mismatch

**1A** says: `Error: Send + std::fmt::Display`
**7A** says: `Error: std::error::Error + Send + 'static`

These conflict. 7A supersedes 1A, but it's also incomplete. Since `GyreError::Agent` contains `Box<dyn std::error::Error + Send + Sync>`, the Agent's error must be `Sync` too for the boxing to work.

**Corrected bound:**
```rust
type Error: std::error::Error + Send + Sync + 'static;
```

`std::error::Error` implies `Display + Debug`, so `Display` from 1A is redundant. `Sync` is needed for `Box<dyn Error + Send + Sync>`. `'static` is needed for boxing (no borrowed data).

**Fix:** Update 1A's Error entry to match 7A: `Error: std::error::Error + Send + Sync + 'static`.

---

## Inconsistency 2: Observation bounds ripple from 1E

**1A** originally said: `Observation: Send`
**1E** changed step to `&self`, which means the observation is passed as `&Self::Observation` to a `Send` future.

For `&T` to be `Send`, `T` must be `Sync`. So `Observation: Send` alone is insufficient — it must be `Observation: Send + Sync`.

The 1E decision already notes `Observation: Send + Sync`. But 1A was never explicitly updated.

**Fix:** 1A should read `Observation: Send + Sync`, `Action: Send + Clone`, `Feedback: Send + Clone`.

**Follow-up question:** Does `Feedback` also need `Sync`? `feedback(&self, fb: &Self::Feedback)` takes `&Feedback` in a `&self` method. If feedback is called from multiple threads concurrently (which `&self` allows), then `&Feedback` must be `Send`, requiring `Feedback: Sync`. In practice, feedback is called by the Gyre, which holds `&A` — the Gyre is unlikely to call feedback from multiple threads simultaneously, but the trait allows it.

**Conservative fix:** `Feedback: Send + Sync + Clone`. Same for `Action` if it's ever passed by reference across thread boundaries.

Actually, `Action` is returned by value from `step`, not passed by reference. So `Action: Send + Clone` is sufficient — no Sync needed. But `Feedback` is passed as `&Self::Feedback`, and if multiple concurrent callers invoke `feedback(&self, &fb)`, the reference needs to be Send, requiring Sync.

**Final corrected bounds:**
```rust
type Observation: Send + Sync;      // passed as &ref to &self method
type Action: Send + Clone;           // returned by value
type Feedback: Send + Sync + Clone;  // passed as &ref to &self method
type Error: std::error::Error + Send + Sync + 'static;
```

---

## Inconsistency 3: StepResult and the feedback flow

With `StepResult<A>`, the Gyre loop looks like:

```rust
loop {
    let result = agent.step(&obs).await?;
    match result {
        StepResult::Continue(action) => {
            // execute tool calls from action
            // tool results become next obs
        }
        StepResult::Done(action) => {
            break;
        }
    }
}
```

**Where does feedback() get called?** In the LLM case, tool results are the next *observation*, not feedback. The Gyre feeds them back as `agent.step(&tool_results)`. So `feedback()` is never called during a normal LLM conversation.

`feedback()` is for **side-channel signals**: human ratings, evaluation scores, RL reward signals. Things that inform the agent but aren't the next observation in the loop.

This is semantically correct but potentially confusing. An LLM user might expect tool results to go through `feedback()`. Need to be very clear in docs that:
- **Observation** = the next input to the agent (user message, tool result, env state)
- **Feedback** = side-channel signal that doesn't drive the next step (score, reward, rating)

**Not an inconsistency, but a documentation gap.** No design change needed.

---

## Gap 1: SessionId is missing from GyreContext

The Gyre needs to persist session state via `ctx.state.save_session(id, state)`. But where does `id` come from? Currently GyreContext doesn't carry a SessionId.

Options:
- The Gyre generates a new SessionId per run (no resumption)
- The executor passes SessionId to the Gyre somehow
- GyreContext carries an optional SessionId

**Recommendation:** Add `session_id: Option<SessionId>` to GyreContext. `None` means "new session" (the Gyre creates one). `Some(id)` means "resume this session."

```rust
pub struct GyreContext {
    pub agent_id: AgentId,
    pub session_id: Option<SessionId>,
    pub permissions: Arc<dyn PermissionGate>,
    pub state: Arc<dyn StateStore>,
    pub config: Arc<Config>,
    pub telemetry: Arc<dyn TelemetrySink>,
}
```

This also affects the builder (3C) — add `session_id` to the builder.

---

## Gap 2: PermissionContext concrete type undefined

Decision 4C says "separate PermissionContext type" but we never defined it. Polar policies need access to:
- Git branch (for "only push on feature branches")
- Worktree path (for "only write within your worktree")
- Arbitrary metadata (for extensibility)

**Recommendation:**

```rust
pub struct PermissionContext {
    /// Current worktree path, if in a git repo.
    pub worktree: Option<WorktreePath>,
    /// Current branch name.
    pub branch: Option<Branch>,
    /// Extensible metadata for custom policies.
    pub extra: HashMap<String, serde_json::Value>,
}
```

Well-known fields are typed (direct access in Polar without map lookups). `extra` handles everything else. The Gyre constructs this from git context + any domain-specific data.

**Updated PermissionRequest:**
```rust
pub struct PermissionRequest {
    pub agent_id: AgentId,
    pub action: ActionKind,
    pub resource: Resource,
    pub context: PermissionContext,
}
```

---

## Gap 3: Root telemetry span ownership

With span-based telemetry (6B), someone needs to create the root span. The Gyre creates spans for steps and tool calls, but who creates the outermost span that encompasses the entire run?

**Option A:** The Gyre creates the root span in `run()`:
```rust
async fn run(&self, agent: &A, ctx: &GyreContext, strategy: &S) -> ... {
    let root = ctx.telemetry.start_span("gyre_run", None);
    // ... all child spans use Some(root)
    ctx.telemetry.end_span(root);
}
```

**Option B:** The executor creates the root span and passes it via GyreContext:
```rust
pub struct GyreContext {
    // ...existing...
    pub parent_span: Option<SpanId>,  // parent to nest under
}
```

Option B is more composable — an executor running multiple Gyres can nest them under a top-level span. Option A is simpler.

**Recommendation:** Add `parent_span: Option<SpanId>` to GyreContext. The Gyre uses it as parent when creating its root span. If None, the Gyre creates a true root. This is zero cost (one optional u64) and enables span composition.

---

## Gap 4: Executor pattern not spec'd

The executor (in `gyres-runtime`) wires everything together:

```rust
// This is what the executor does:
let config = Config::load()?;                    // from ~/.agents/
let git = GitContext::detect(&cwd).await?;       // worktree detection
let state = JsonFileStore::new(sessions_dir);    // or SqliteStore
let permissions = PermissionChain::builder()
    .load_policies(&config.agents_dir)?
    .build();
let telemetry = StdoutTelemetry::new();          // or Langfuse

let ctx = GyreContext::builder()
    .agent_id("agent-1")
    .session_id(resume_session)
    .permissions(permissions)
    .state(state)
    .telemetry(telemetry)
    .build();

let outcome = gyre.run(&agent, &ctx, &strategy).await?;
agent.reset()?;
```

This pattern is consistent with all our decisions. No inconsistency, but it should be documented as part of the gyres-runtime spec to ensure the core traits support it.

Key note: `agent.reset()` requires `&mut agent`, which means the executor must own the agent (not share it). After `gyre.run()` completes and returns the `&A` borrow, the executor can call `reset()`. This works because Rust's borrow checker ensures the run's borrow is released before reset is called.

---

## Gap 5: Strategy associated type defaults unstable

Decision 3B uses `type Strategy: Send + Sync` as an associated type on Gyre. For simple Gyres that don't need a strategy, the implementor writes `type Strategy = ();`.

`associated_type_defaults` (which would allow `type Strategy: Send + Sync = ();` in the trait definition) is **unstable** in Rust as of 1.85.

This means every Gyre impl must specify `type Strategy`, even for `()`. This is one line of boilerplate, which is acceptable. But worth noting.

**No design change needed.** Just a documentation note.

---

## Cascading effect check: 1E (&self on Agent) ripples

Decision 1E (Agent uses `&self`) affects:

| Decision | Effect | Status |
|---|---|---|
| 1A (bounds) | Observation needs Sync, Feedback needs Sync | **Needs update** (see Inconsistency 2) |
| 1B (feedback sync) | Still correct — &self feedback is sync | No change |
| 1C (StepResult) | step() returns StepResult — unaffected by &self | No change |
| 1D (reset) | reset remains &mut self — correct, enforces exclusivity | No change |
| 2A (no middleware) | Gyre is simpler with &A — no &mut forwarding to worry about | Simplifies |
| 3B (Gyre::run) | Takes `&A` instead of `&mut A` | **Already updated** |
| 5B (DomainTurn) | Gyre serializes turns via DomainTurn — unaffected by Agent self type | No change |
| 7A (error boxing) | Agent::Error needs Sync (see Inconsistency 1) | **Needs update** |

---

## Cascading effect check: 3B (Strategy) ripples

| Decision | Effect | Status |
|---|---|---|
| 2A (no middleware) | Strategy replaces what middleware would configure | Consistent |
| 3C (builder) | Builder doesn't need Strategy — it's on Gyre, not GyreContext | No change |
| Gyre::run signature | `fn run(&self, agent: &A, ctx: &GyreContext, strategy: &Self::Strategy)` | **Already updated** |

---

## Cascading effect check: 5B (DomainTurn) ripples

| Decision | Effect | Status |
|---|---|---|
| 5A (append_turn) | append_turn takes `&SerializedTurn` | Consistent |
| StateStore trait | Works with SerializedTurn — dyn-compatible | Consistent |
| GyreContext | StateStore behind Arc<dyn> — no generic parameters needed | Consistent |

---

## Summary: All issues found

### Inconsistencies (must fix)

1. **Agent::Error bound** — 1A says `Send + Display`, 7A says `Error + Send + 'static`. Correct bound is `std::error::Error + Send + Sync + 'static`.

2. **Observation/Feedback Sync** — 1E changed step/feedback to `&self`, which requires `Observation: Sync` and `Feedback: Sync` for `&ref` to be Send across threads.

### Gaps (must fill)

3. **SessionId on GyreContext** — Gyre needs to save/load sessions but has no session identity. Add `session_id: Option<SessionId>`.

4. **PermissionContext type** — Decision 4C says "separate type" but it was never defined. Proposed: typed fields (worktree, branch) + extensible HashMap.

5. **Root telemetry span** — Add `parent_span: Option<SpanId>` to GyreContext so the executor can nest Gyre runs under broader spans.

### Documentation gaps (not design issues)

6. **feedback vs observation semantics** — Must document clearly that tool results are observations, not feedback. Feedback is for side-channel signals (scores, rewards).

7. **Strategy `()` boilerplate** — Associated type defaults are unstable. Every Gyre must write `type Strategy = ();`. Not a problem, just document it.

8. **Executor pattern** — gyres-runtime should document the wiring pattern and the `run()` → `reset()` lifecycle.

---

## Corrected final trait signatures

### Agent (gyres-core)

```rust
pub enum StepResult<A> {
    Continue(A),
    Done(A),
}

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

    /// Side-channel feedback (scores, rewards). Not the next observation.
    fn feedback(&self, fb: &Self::Feedback);

    /// Reset agent state for a new episode/session.
    /// &mut self enforces no concurrent steps during reset.
    fn reset(&mut self) -> Result<(), Self::Error>;
}
```

### Gyre (gyres-core)

```rust
pub trait Gyre<A: Agent>: Send + Sync {
    type Outcome: Send + Clone;
    type Strategy: Send + Sync;

    fn run(
        &self,
        agent: &A,
        ctx: &GyreContext,
        strategy: &Self::Strategy,
    ) -> impl Future<Output = Result<Self::Outcome, GyreError>> + Send;
}
```

### GyreContext (gyres-core)

```rust
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

### PermissionRequest (gyres-core)

```rust
pub struct PermissionContext {
    pub worktree: Option<WorktreePath>,
    pub branch: Option<Branch>,
    pub extra: HashMap<String, serde_json::Value>,
}

pub struct PermissionRequest {
    pub agent_id: AgentId,
    pub action: ActionKind,
    pub resource: Resource,
    pub context: PermissionContext,
}
```

### TelemetrySink (gyres-core)

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

### DomainTurn + SerializedTurn (gyres-core)

```rust
pub struct SerializedTurn {
    pub timestamp: SystemTime,
    pub domain: String,
    pub observation: serde_json::Value,
    pub action: serde_json::Value,
    pub feedback: Option<serde_json::Value>,
}

pub trait DomainTurn: Send + Clone {
    const DOMAIN: &'static str;
    fn serialize(&self) -> SerializedTurn;
    fn deserialize(turn: &SerializedTurn) -> Result<Self, GyreError> where Self: Sized;
}
```

### GyreError (gyres-core)

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
