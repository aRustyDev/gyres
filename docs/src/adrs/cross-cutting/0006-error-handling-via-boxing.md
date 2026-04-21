---
status: accepted
date: 2026-04-20
tags: [errors, cross-cutting, core]
---

# Agent Errors are Boxed, Not Converted

## Context

Agent implementations define their own error types. The Gyre returns `Result<Outcome, GyreError>`. A conversion path is needed from `Agent::Error` to `GyreError`.

## Decision

`Agent::Error: std::error::Error + Send + Sync + 'static`. The Gyre wraps errors via boxing:

```rust
agent.step(obs).await.map_err(|e| GyreError::Agent(Box::new(e)))?;
```

`GyreError::Agent` contains `Box<dyn std::error::Error + Send + Sync>`, preserving the original error for downcasting.

## Consequences

- Zero boilerplate for Agent implementors — just `#[derive(Error)]`.
- Original error type preserved — callers can `downcast_ref::<MyAgentError>()`.
- One `map_err` per step/reset call in the Gyre — minimal overhead.
- `Sync` bound on Error is required for the `Box<dyn Error + Send + Sync>` to work.

## Alternatives Considered

- **`Agent::Error: Into<GyreError>`**: Requires every agent to `impl From<MyError> for GyreError>`. Stringifies the error, losing the original type. More boilerplate for agent implementors.
