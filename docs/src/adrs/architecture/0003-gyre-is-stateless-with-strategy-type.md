---
status: accepted
date: 2026-04-20
tags: [gyre, strategy, core]
---

# Gyre is Stateless with Strategy as Associated Type

## Context

Gyres drive agent execution loops. We needed to support adaptive behavior (e.g., epsilon decay in RL, reflection frequency adjustment) without making the Gyre itself mutable.

## Decision

- `Gyre::run` takes `&self` — the Gyre is a stateless strategy definition.
- `type Strategy: Send + Sync` is an associated type on the Gyre trait, passed by `&ref` to `run()`.
- The caller mutates Strategy between runs. The Gyre itself is `Send + Sync` and shareable.
- Gyres that need no strategy use `type Strategy = ()`.

## Consequences

- A single Gyre instance can be shared across concurrent runs with different agents.
- Adaptive behavior is explicit — the caller controls strategy mutation, not hidden Gyre state.
- Every Gyre implementation must specify `type Strategy`, even for `()` (associated type defaults are unstable in Rust as of 1.85).
- The `run` signature is `fn run(&self, agent: &A, ctx: &GyreContext, strategy: &Self::Strategy)` — four parameters, all shared references.

## Alternatives Considered

- **Strategy baked into Gyre at construction**: Simpler signature but requires new Gyre instances for strategy changes. Less natural for adaptive loops.
- **Strategy in GyreContext/Config**: Stringly-typed, loses compile-time safety.
- **`&mut self` on run**: Would prevent sharing Gyre instances and add unnecessary state management.
