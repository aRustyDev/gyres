---
status: accepted
date: 2026-04-20
tags: [context, architecture, core]
---

# GyreContext is Thin with Always-Present Telemetry

## Context

GyreContext carries shared infrastructure for Gyre implementations. The question was how much to put directly on the struct vs behind trait interfaces.

## Decision

Four trait-object fields plus identity/session metadata:

```
agent_id, session_id, parent_span,
permissions (Arc<dyn PermissionGate>),
state (Arc<dyn StateStore>),
config (Arc<Config>),
telemetry (Arc<dyn TelemetrySink>)
```

- Approval cache is accessed through `permissions` (the PermissionChain owns it internally).
- Git/worktree context is accessed through `state` (the StateStore is worktree-aware).
- Telemetry is top-level because it's the one cross-cutting concern every Gyre should use without thinking about it.
- `GyreContext` implements `Clone` — all fields are `Arc`.
- Takes `&GyreContext` (immutable) — `Arc` fields handle their own interior mutability.

## Consequences

- Adding telemetry to a Gyre is zero effort — it's always there, even as a no-op.
- Approval cache and git context aren't directly accessible — you go through the permission chain or state store. Slightly more indirection, but cleaner separation.
- `session_id: Option<SessionId>` enables session resumption without a separate mechanism.
- `parent_span: Option<SpanId>` enables the executor to nest multiple Gyre runs under a single trace.
