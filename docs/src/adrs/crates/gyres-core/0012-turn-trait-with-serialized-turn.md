---
status: accepted
date: 2026-04-20
tags: [persistence, turns, gyres-core]
---

# Turn Trait with SerializedTurn for Type-Erased Persistence

## Context

Turn data (observation + action + feedback) is domain-specific (LLM uses Messages, RL uses StateVectors). The StateStore must be dyn-compatible (`Arc<dyn StateStore>`), which means it can't be generic over turn types.

## Decision

Two-layer model:

- `SerializedTurn` (in gyres-core): type-erased, uses `serde_json::Value`. The StateStore operates on this.
- `Turn` trait (in gyres-core): domain crates implement this to convert between their typed turns and `SerializedTurn`.
- Domain crates define typed turns (e.g., `LlmTurn` with `Message` fields) and implement `Turn`.

## Consequences

- StateStore stays dyn-compatible — no generic type parameters.
- Full type safety in domain code (the Gyre works with typed turns).
- Serialization is explicit via `Turn::serialize()` / `Turn::deserialize()`.
- New domains (RL, robotics) implement `Turn` for their own types without modifying gyres-core.
- The `domain: String` field on `SerializedTurn` enables heterogeneous session loading.

## Alternatives Considered

- **Generic `Turn<O, A, F>`**: Makes `SessionState` and `StateStore` generic, breaking dyn-compatibility. `Arc<dyn StateStore<LlmTurn>>` infects GyreContext with domain types.
- **`#[non_exhaustive]` enums in core**: Rust doesn't allow adding variants to enums defined in other crates. Would require gyres-core to have opinions about every domain.
- **`serde_json::Value` everywhere (no Turn trait)**: Works but loses the compile-time guarantee that serialization is correct. Silent runtime failures.
