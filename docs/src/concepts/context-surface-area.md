# GyreContext Surface Area

## Current (post-store-abstraction)

```rust
pub struct GyreContext {
    // Identity
    pub agent_id: AgentId,
    pub session_id: Option<SessionId>,
    pub parent_span: Option<SpanId>,

    // Infrastructure
    pub permissions: Arc<dyn PermissionGate>,
    pub config: Arc<Config>,
    pub telemetry: Arc<dyn TelemetrySink>,

    // Stores
    pub state: Arc<dyn StateStore>,
    pub memory: Arc<dyn MemoryStore>,
    pub tasks: Arc<dyn TaskStore>,
    pub artifacts: Arc<dyn ArtifactStore>,
}
```

10 fields. All behind Arc (cheap clone). Builder with defaults.

## Potential additions from multi-agent

```rust
    // Multi-agent (optional for single-agent Gyres)
    pub agents: Option<Arc<dyn AgentRegistry>>,
    pub messages: Option<Arc<dyn MessageBus>>,
```

12 fields total. The optional fields avoid burdening single-agent use.

## Design principles

1. **Always-present fields** (no Option): things every Gyre should use without thinking.
   - permissions, config, telemetry, state, memory, tasks, artifacts
   - Even if unused, the InMemory backend is zero-cost.

2. **Optional fields**: things only multi-agent Gyres need.
   - agents, messages

3. **Identity fields**: immutable per-run context.
   - agent_id, session_id, parent_span

4. **No domain-specific fields**: GyreContext doesn't know about LLM messages, RL episodes, etc. Domain state lives in the Agent or Strategy.

## Why not group into sub-structs?

Could group stores:
```rust
pub stores: Stores,  // contains state, memory, tasks, artifacts
```

Pro: fewer top-level fields. Con: `ctx.stores.memory.recall(...)` instead of `ctx.memory.recall(...)`. The extra nesting adds noise to every call site in every Gyre implementation.

Decision: flat fields. The builder handles construction complexity. Call sites stay clean.

## Minimal constructor for tests

```rust
impl GyreContext {
    pub fn minimal(agent_id: impl Into<AgentId>) -> Self {
        let backend = Arc::new(InMemoryBackend::new());
        Self {
            agent_id: AgentId::new(agent_id),
            session_id: None,
            parent_span: None,
            permissions: Arc::new(AllowAll),
            config: Arc::new(Config::default()),
            telemetry: Arc::new(NoopTelemetry),
            state: backend.clone(),
            memory: backend.clone(),
            tasks: backend.clone(),
            artifacts: backend,
            agents: None,
            messages: None,
        }
    }
}
```

One line to get a working GyreContext for tests.
