---
status: accepted
date: 2026-04-20
tags: [agent, concurrency, core]
---

# Agent Trait Uses &self for step/feedback, &mut self for reset

## Context

The Agent trait's `step` method originally took `&mut self`, which provides compile-time data race safety but prevents concurrent, batched, and speculative execution — all targeted scenarios for gyres (vectorized RL environments, MCTS tree search, speculative execution).

## Decision

- `step(&self)` and `feedback(&self)` — enables concurrent calls. Implementations use interior mutability (`RwLock`, `Mutex`).
- `reset(&mut self)` — exclusive access. Compiler enforces no steps in flight during reset.
- `Agent: Send + Sync` — required because `&self` references cross async task boundaries.

## Consequences

- LLM agents pay a minor cost: `RwLock<Vec<Message>>` instead of `Vec<Message>`. Invisible vs LLM API latency.
- RL agents can override `step_batch` for GPU-batched forward passes without needing a separate trait.
- One unified `Agent` trait serves both sequential and concurrent execution patterns.
- `step_batch` has a default sequential implementation, so simple agents don't need to implement it.

## Alternatives Considered

- **`&mut self` on step + separate `BatchAgent` trait**: Rejected because it creates combinatorial complexity — every Gyre needs to be generic over both traits or have two implementations.
- **`&self` on everything including reset**: Rejected because reset genuinely needs exclusive access to avoid resetting while steps are in flight.
