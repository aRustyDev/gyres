# gyres-core Spec Discussion

## 1B: Sync vs Async feedback — the full matrix

Here's the concrete impact for each combination:

### LLM + Sync feedback
```rust
fn feedback(&mut self, fb: &ToolResult) {
    self.history.push(fb.clone()); // Vec::push — O(1), no I/O
}
```
Cost: ~nanoseconds. Perfect fit.

### LLM + Async feedback
```rust
async fn feedback(&mut self, fb: &ToolResult) {
    self.history.push(fb.clone()); // same operation, wrapped in async
}
```
Cost: Future allocation + executor scheduling for a nanosecond operation. Pure overhead. Every tool result in every agent loop pays this tax.

### RL + Sync feedback
```rust
fn feedback(&mut self, fb: &Reward) {
    self.reward_buffer.push(fb.value); // buffer it
    // Gradient update happens in next step(), not here
}
```
Cost: nanoseconds. The expensive work (policy update) is deferred to `step()`, which is already async. This is the standard RL pattern — collect rewards, batch-update policy on the next forward pass.

### RL + Async feedback
```rust
async fn feedback(&mut self, fb: &Reward) {
    self.policy.update(fb).await; // immediate gradient update
}
```
Cost: enables immediate policy updates without blocking. But this pattern is unusual — most RL algorithms (PPO, SAC, DQN) batch updates, not per-step updates. The only case where immediate async feedback matters is online learning with a remote policy server.

### Performance / reliability / complexity summary

| | Sync | Async |
|---|---|---|
| **LLM perf** | Optimal (nanoseconds) | Overhead (Future alloc + schedule) |
| **RL perf** | Good (buffer + batch in step) | Marginal benefit (immediate updates) |
| **Reliability** | Can't fail (no I/O) | Can fail (timeout, network) |
| **Complexity** | Simple (no .await in Gyre loop) | Every Gyre must .await feedback |
| **Trait ergonomics** | `fn feedback(&mut self, fb: &F)` | `fn feedback(&mut self, fb: &F) -> Pin<Box<dyn Future<...>>>` (dyn-compat) or RPIT |

### Defense of sync

Sync wins because:
1. The 99% case (LLM tool results, RL reward buffering) is trivially sync
2. The 1% case (RL immediate policy update) can be handled by buffering in `feedback` and processing in `step` — which is already async
3. Making feedback async infects the entire Gyre loop with unnecessary `.await` calls
4. An RL agent that *genuinely* needs async feedback (remote policy server) can spawn a background task in `feedback` and await its completion in `step`

**Recommendation: sync.** The pattern of "buffer in feedback, process in step" handles everything without complicating the common case.

---

## 1D: Reset return type

`reset` can fail in realistic scenarios:
- RL agent resets a remote environment simulator (network call)
- Agent checkpoints state before clearing (I/O)
- Agent releases resources that might be held by another process

```rust
fn reset(&mut self) -> Result<(), Self::Error>;
```

Using `Self::Error` keeps it consistent with `step`. An LLM agent that just clears a vec returns `Ok(())`.

---

## 1E: `&mut self` on step — the limitation in depth

The constraint: Rust's borrow checker ensures that while a `&mut self` borrow exists, no other reference to self can exist. When `step` returns a future that borrows `&mut self`, you can't:

```rust
// This won't compile:
let fut1 = agent.step(&obs1);  // &mut self borrowed here
let fut2 = agent.step(&obs2);  // ERROR: cannot borrow `agent` as mutable more than once
join!(fut1, fut2);
```

**When this matters:**

1. **Vectorized RL environments.** You want to step 64 environments simultaneously. With `&mut self`, you need 64 agent instances. With `&self`, you could share one agent (if it uses interior mutability).

2. **Speculative execution.** Try multiple approaches in parallel, pick the best. Requires concurrent steps on the same agent state.

3. **Tree search / MCTS.** Branch from a state, explore multiple paths concurrently.

**When it doesn't matter:**

1. **Standard LLM agent loop.** Strictly sequential: step → tool calls → feedback → step.
2. **Single RL episode.** One step at a time per episode.
3. **Multi-agent.** Multiple *different* agents run in parallel, each with `&mut self`. That works fine.

**The design space:**

```
&mut self (current)
├── Sequential only
├── Compile-time safety (no data races possible)
├── Simple agent implementations (no Mutex/RwLock)
└── For concurrency: use multiple agent instances

&self
├── Concurrent steps possible
├── Requires interior mutability (Mutex<InternalState>)
├── Every agent impl needs to think about thread safety
└── Runtime cost: lock contention, potential deadlocks
```

**Recommendation:** Keep `&mut self` for the `Agent` trait. If batched/concurrent execution is needed later, add a separate `BatchAgent` trait:

```rust
pub trait BatchAgent: Send {
    type Observation: Send;
    type Action: Send + Clone;
    type Error: std::error::Error + Send + 'static;

    fn step_batch(
        &mut self,
        observations: &[Self::Observation],
    ) -> impl Future<Output = Result<Vec<StepResult<Self::Action>>, Self::Error>> + Send;
}
```

This keeps the base Agent simple while giving RL a natural extension point. A `BatchAgent` takes N observations and returns N actions — the parallelism happens *inside* the implementation (GPU batch, threadpool, etc.), not at the trait boundary.

---

## 2A: Hooks vs Middleware — deeper comparison

### Hooks on the Gyre trait

```rust
pub trait Gyre<A: Agent>: Send {
    type Outcome: Send + Clone;

    fn on_start(&self, _ctx: &GyreContext) {}
    fn on_step(&self, _action: &A::Action, _ctx: &GyreContext) {}
    fn on_end(&self, _outcome: &Self::Outcome, _ctx: &GyreContext) {}

    fn run(&self, agent: &mut A, ctx: &GyreContext)
        -> impl Future<Output = Result<Self::Outcome, GyreError>> + Send;
}
```

**Adding telemetry:**
```rust
impl Gyre<LlmAgent> for ConversationGyre {
    fn on_start(&self, ctx: &GyreContext) {
        ctx.telemetry.record(SpanStart { name: "conversation" });
    }
    // ...etc
}
```

**Problem:** hooks are *suggestions*, not guarantees. The `run` implementation has to remember to call them. If `ConversationGyre::run` forgets to call `on_step`, telemetry silently breaks. There's no compile-time enforcement.

### Middleware wrapper

```rust
pub struct WithTelemetry<G> { inner: G }

impl<A: Agent, G: Gyre<A>> Gyre<A> for WithTelemetry<G> {
    type Outcome = G::Outcome;
    fn run(&self, agent: &mut A, ctx: &GyreContext) -> ... {
        async move {
            ctx.telemetry.record(SpanStart { name: "gyre" });
            let result = self.inner.run(agent, ctx).await;
            ctx.telemetry.record(SpanEnd { name: "gyre" });
            result
        }
    }
}
```

**Problem:** middleware can only wrap the *outer* `run()`. It can't intercept individual steps or tool calls inside the loop. Permission checking happens per-tool-call, *inside* the Gyre's loop — middleware can't reach there.

### The real insight

Neither hooks nor middleware solve the *inner loop* cross-cutting concerns (permission checking, per-tool telemetry). Those are handled by `GyreContext` — the Gyre calls `ctx.permissions.evaluate(...)` and `ctx.telemetry.record(...)` inside its loop. That's not a hooks-vs-middleware question; it's just "use the context."

The question is only about *outer run() concerns*: timing the whole run, retry on failure, timeout, logging start/end.

| Concern | Hooks | Middleware | GyreContext |
|---|---|---|---|
| Timeout around run() | | Wrapper | |
| Retry on failure | | Wrapper | |
| Log start/end of run | Default methods | | |
| Per-step telemetry | | | `ctx.telemetry` |
| Per-tool permission | | | `ctx.permissions` |
| Tool dispatch | | | Part of Gyre impl |

**Revised recommendation:** Don't add hooks OR middleware to the trait. Keep `Gyre` minimal (just `run`). Inner-loop infrastructure goes through `GyreContext`. Outer-loop wrappers (timeout, retry) are standalone:

```rust
let result = tokio::time::timeout(
    Duration::from_secs(300),
    gyre.run(&mut agent, &ctx),
).await;
```

This avoids both the "hooks aren't enforced" problem and the "middleware can't reach inside the loop" problem.

---

## 3B: Does `Gyre::run` need `&mut self`?

With `ConversationGyre { max_turns: usize }`, nothing mutates during `run()`. The turn counter is a local variable inside the async block. `max_turns` is read-only config.

When *would* a Gyre genuinely need `&mut self`?
- Accumulating statistics across multiple calls to `run()` (rare, can use interior mutability)
- Adaptive behavior (changing strategy based on past runs)

**Since we decided `&GyreContext` in 2C:** the Gyre reads from context immutably. If the Gyre's own state is also immutable during `run()`, then `&self` makes sense:

```rust
pub trait Gyre<A: Agent>: Send + Sync {
    type Outcome: Send + Clone;

    fn run(
        &self,
        agent: &mut A,
        ctx: &GyreContext,
    ) -> impl Future<Output = Result<Self::Outcome, GyreError>> + Send;
}
```

Adding `Sync` since `&self` across async boundaries requires it. This also means you can share one `Gyre` instance across multiple concurrent runs (with different agents), which is natural — the Gyre is a strategy, not stateful.

---

## 5B: Turn — typed vs Value

The problem: `Turn` lives in `gyres-core`, but the actual observation/action/feedback types are defined in domain crates (`gyres-llm`, `gyres-rl`).

If `Turn` is generic `Turn<O, A, F>`, then `SessionState<O, A, F>` and `StateStore<O, A, F>` become generic — breaking dyn-compatibility.

**Proposed solution: two-layer model.**

```rust
// gyres-core: persistence boundary — untyped
#[not_exhaustive]
pub enum ObservationType {
    ...
}
#[not_exhaustive]
pub enum ActionType {
    ...
}
#[not_exhaustive]
pub enum FeedbackType {
    ...
}
pub struct SerializedTurn {
    pub timestamp: SystemTime,
    pub observation: ObservationType,
    pub action: ActionType,
    pub feedback: Option<FeedbackType>,
}

// SessionState and StateStore use SerializedTurn
pub struct SessionState {
    pub id: SessionId,
    pub turns: Vec<SerializedTurn>,
    // ...
}
```

```rust
// gyres-llm: domain layer — fully typed
pub enum ObservationType {
    LlmObservation (Message)
}
pub enum ActionType {
    LlmAction (Message)
}
pub enum FeedbackType {
    LlmFeedback (Score)
}
pub struct LlmTurn {
    pub timestamp: SystemTime,
    pub observation: LlmObservation,
    pub action: LlmAction,
    pub feedback: Option<LlmFeedback>,
}

impl From<LlmTurn> for SerializedTurn { ... }
impl TryFrom<SerializedTurn> for LlmTurn { ... }
```

The Gyre works with `LlmTurn` internally. When it calls `ctx.state.save_session(...)`, it serializes to `SerializedTurn`. When loading, it deserializes back to `LlmTurn`.

This gives you type safety where you write code (the Gyre) and flexibility where you store data (the StateStore). The conversion is explicit, not hidden.

---

## 6B: Flat events vs spans — what this means concretely

When an agent runs, the execution looks like:

```
conversation                          [=================]
├── agent_step (turn 1)               [======]
│   └── provider_complete             [====]
├── tool_execution: bash "git status" [==]
├── agent_step (turn 2)               [========]
│   └── provider_complete             [======]
└── (done)
```

**With flat events**, you emit disconnected points:

```
record(SpanStart { name: "conversation" })
record(SpanStart { name: "agent_step" })
record(SpanStart { name: "provider_complete" })
record(SpanEnd { name: "provider_complete" })
record(SpanEnd { name: "agent_step" })
record(SpanStart { name: "tool_execution" })
record(SpanEnd { name: "tool_execution" })
...
```

The sink has to reconstruct the tree by matching Start/End pairs and tracking nesting. If events arrive out of order (async), reconstruction gets fragile.

**With span-based API**, the tree is explicit:

```rust
let conversation = ctx.telemetry.start_span("conversation", None);
let step1 = ctx.telemetry.start_span("agent_step", Some(conversation));
let llm = ctx.telemetry.start_span("provider_complete", Some(step1));
ctx.telemetry.end_span(llm);
ctx.telemetry.end_span(step1);
let tool = ctx.telemetry.start_span("tool_execution", Some(conversation));
ctx.telemetry.end_span(tool);
ctx.telemetry.end_span(conversation);
```

Parent-child relationships are established at creation time. The sink receives a pre-built tree. This maps directly to OTEL spans and Langfuse observations (which are parent-child spans).

**For MVP**, flat events are simpler and sufficient — the `StdoutTelemetry` sink doesn't need tree structure. But when `gyres-tracing` integrates with Langfuse/OTEL, we'll want spans. The question is whether to design the trait for spans now or migrate later.

**Revised recommendation:** define the trait with spans now, since it's not much more complex and avoids a breaking change later:

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

`NoopTelemetry` returns 0 for every span and ignores everything. Simple implementations are still trivial. But the Langfuse sink gets the tree structure it needs.

---

## 6C: `tracing` crate relationship — clarified

There are **two separate concerns** that happen to both be called "tracing":

**Concern 1: Internal logging/instrumentation** (the `tracing` crate)

This is how gyres code itself emits diagnostic information:

```rust
// Inside ConversationGyre::run()
tracing::info!(turn = turn_count, "agent step completed");
tracing::warn!("context window 80% full, compaction recommended");
```

This is developer-facing. It goes to stdout, a log file, Datadog, whatever subscriber the user configures. The `tracing` crate handles this, and gyres should use it — it's Rust's standard.

**Concern 2: Agent observability export** (the `TelemetrySink` trait)

This is how agent *behavior* is recorded for analysis in Langfuse/OTEL:

```rust
// Inside ConversationGyre::run()
let span = ctx.telemetry.start_span("agent_step", parent);
ctx.telemetry.set_attribute(span, "model", "claude-opus-4-6");
ctx.telemetry.set_attribute(span, "input_tokens", "1523");
ctx.telemetry.end_span(span);
```

This is user/analyst-facing. It goes to Langfuse, an OTEL collector, etc.

**They're complementary:**
- `tracing` = "what is the code doing?" (developer diagnostics)
- `TelemetrySink` = "what is the agent doing?" (agent observability)

**The bridge in `gyres-tracing`:**

```rust
// gyres-tracing provides a tracing::Layer that ALSO forwards to TelemetrySink
pub struct GyresTracingLayer {
    sink: Arc<dyn TelemetrySink>,
}

impl<S: Subscriber> Layer<S> for GyresTracingLayer {
    fn on_new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<S>) {
        // Forward to TelemetrySink
        self.sink.start_span(attrs.metadata().name(), parent);
    }
}
```

This means if you want, you can use `tracing::span!()` macros and have them automatically flow to Langfuse. But you can also use `TelemetrySink` directly without `tracing` at all.

**So the split is:**
- `gyres-core` defines `TelemetrySink` (no `tracing` dependency)
- `gyres-tracing` bridges `tracing` → `TelemetrySink` (depends on both)
- Users choose: use `TelemetrySink` directly, or use `tracing` macros + the bridge layer

---

## 7A: Error conversion — option 1 vs 2

**Option 1: `Agent::Error: Into<GyreError>`**

```rust
// Agent implementor must write:
impl From<MyAgentError> for GyreError {
    fn from(e: MyAgentError) -> Self {
        GyreError::Agent(e.to_string())  // loses the original error type
    }
}

// Gyre uses:
let action = agent.step(obs).await?;  // ? auto-converts via Into
```

Limitations:
- Every Agent error type is coupled to `GyreError`
- The `From` impl typically stringifies, losing the original error (can't downcast)
- If someone has `Agent` in a separate crate that doesn't depend on `gyres-core`, they can't impl `From<TheirError> for GyreError` (orphan rule). Actually — they DO depend on `gyres-core` since they implement the `Agent` trait. So the orphan rule isn't an issue. But they still have to write the boilerplate.

**Option 2: `Agent::Error: std::error::Error + Send + 'static`**

```rust
// Agent implementor writes nothing extra — just derive Error:
#[derive(Error, Debug)]
enum MyAgentError {
    #[error("model unavailable")]
    ModelDown,
}

// Gyre wraps manually:
let action = agent.step(obs).await
    .map_err(|e| GyreError::Agent(Box::new(e)))?;
```

Advantages:
- Zero boilerplate for Agent implementors
- Original error preserved (can downcast: `error.downcast_ref::<MyAgentError>()`)
- Standard Rust pattern (anyhow, eyre all use `Box<dyn Error>`)

The downcast preservation is the real win. With option 1, the error is stringified and the original type is lost. With option 2:

```rust
match &gyre_error {
    GyreError::Agent(boxed) => {
        if let Some(my_err) = boxed.downcast_ref::<MyAgentError>() {
            // Can pattern-match on the original error
        }
    }
}
```

**Recommendation: option 2.** The `map_err` in the Gyre is a one-liner, and Agent implementors get zero-cost error types with full preservability.

To make GyreError work with this:

```rust
#[derive(Error, Debug)]
pub enum GyreError {
    #[error("agent error: {0}")]
    Agent(#[source] Box<dyn std::error::Error + Send + Sync>),
    // ...rest unchanged...
}
```

---

## Updated decisions summary

| # | Decision | Status |
|---|---|---|
| 1A | `Observation: Send`, `Action: Send + Clone`, `Feedback: Send + Clone` | **Confirmed** |
| 1B | `feedback` is sync | **Confirmed** (buffer in feedback, process in step) |
| 1C | Replace `should_stop` with `StepResult<A>` | **Confirmed** |
| 1D | `fn reset(&mut self) -> Result<(), Self::Error>` | **New** |
| 1E | `&mut self` on step, BatchAgent trait for concurrency | **Needs your input** |
| 2A | No hooks, no middleware. Keep Gyre minimal. GyreContext handles inner-loop. tokio handles outer-loop (timeout, etc.) | **New — revised from middleware** |
| 2B | `Outcome: Send + Clone` | **Confirmed** |
| 2C | `&GyreContext` (immutable) | **Confirmed** |
| 3A | GyreContext carries AgentId | **Confirmed** |
| 3B | `Gyre::run` takes `&self` (not `&mut self`). Gyre is `Send + Sync`. | **New** |
| 3C | Builder pattern for GyreContext | **Confirmed** |
| 4A | `#[non_exhaustive]` with Network, Spawn, Other | **Confirmed** |
| 4B | `PathBuf` for paths, convert to String at Polar boundary | **Confirmed** |
| 4C | Separate `PermissionContext` type on request | **Confirmed** |
| 5A | `append_turn` with default read-modify-write impl | **Confirmed** |
| 5B | Two-layer: `SerializedTurn` in core (Value), typed `LlmTurn` in domain crate | **New** |
| 5C | No session versioning for MVP | **Confirmed** |
| 6A | Buffer internally, drop on overflow | **Confirmed** |
| 6B | Span-based API now (start_span/end_span with parent) | **New — revised from flat events** |
| 6C | `gyres-core`: defines `TelemetrySink` (no tracing dep). `gyres-tracing`: bridges `tracing` crate → `TelemetrySink`. | **Clarified** |
| 7A | `Agent::Error: std::error::Error + Send + 'static`, boxed in `GyreError::Agent` | **Confirmed: option 2** |

New decisions that emerged: 1D (reset return type), 2A (no hooks no middleware), 3B (Gyre is &self + Sync), 5B (two-layer turns), 6B (span-based telemetry).
