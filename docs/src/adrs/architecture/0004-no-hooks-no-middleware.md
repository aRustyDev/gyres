---
status: accepted
date: 2026-04-20
tags: [gyre, architecture, core]
---

# No Hooks or Middleware on the Gyre Trait

## Context

Cross-cutting concerns (telemetry, permissions, timeout) need to apply to every Gyre run. Two patterns were considered: lifecycle hooks on the trait, and middleware wrappers.

## Decision

Neither hooks nor middleware. The Gyre trait has only `run()`. Cross-cutting concerns are handled by:

- **Inner-loop concerns** (per-step telemetry, per-tool permissions): accessed through `GyreContext` fields (`ctx.telemetry`, `ctx.permissions`). The Gyre calls these directly inside its loop.
- **Outer-loop concerns** (timeout, retry): handled by standard async combinators (`tokio::time::timeout`, retry crates).

## Consequences

- The Gyre trait stays minimal — one method.
- No "forgot to call on_step()" bugs (hooks aren't enforced by the compiler).
- No nested type signatures from middleware composition.
- Gyre implementations are responsible for calling `ctx.telemetry` and `ctx.permissions` at the right points — this is an explicit design choice, not a gap.

## Alternatives Considered

- **Hooks on the trait** (`on_start`, `on_step`, `on_end`): Rejected because hooks are suggestions, not guarantees. The `run()` implementation must remember to call them. Silent failure if forgotten.
- **Middleware wrappers** (`WithTelemetry<G>`): Rejected because middleware can only intercept the outer `run()`, not inner-loop operations (tool dispatch, per-step telemetry). The cross-cutting concerns that matter happen inside the loop.
