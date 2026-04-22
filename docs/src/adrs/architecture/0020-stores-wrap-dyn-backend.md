---
number: 20
title: Stores wrap dyn Backend internally (type erasure at store boundary)
date: 2026-04-22
status: accepted
---

# 20. Stores wrap dyn Backend internally (type erasure at store boundary)

Date: 2026-04-22

## Status

Accepted

## Context

Stores need backend polymorphism — the same TaskStore API must work with SQLite, SurrealDB, or an in-memory backend. The question: where does the Backend type parameter live?

Alternatives considered:

1. **Stores are traits, backends implement them** — `trait TaskStore`, `impl TaskStore for SqliteBackend`. This is the original design in gyres-core. Backend polymorphism via `Arc<dyn TaskStore>`. The store IS a trait, not a struct.

2. **Stores are generic structs** — `TaskStore<B: Backend>`. Static dispatch, zero vtable overhead. But `B` propagates upward: `GyreContext<B>`, `Gyre<B>`, `Agent<B>` — every function signature touching context carries `B`. This is the "generic soup" problem.

3. **Stores are concrete structs wrapping `Arc<dyn Backend>`** — `struct TaskStore { backend: Arc<dyn Backend> }`. The Backend type is erased at the store boundary. Nothing above the store layer sees `B`.

## Decision

Option 3: **Stores are concrete structs wrapping `Arc<dyn Backend>`.**

```rust
pub struct TaskStore {
    backend: Arc<dyn Backend>,
}

impl TaskStore {
    pub fn ready_tasks(&self) -> Result<Vec<Task>, GyreError> {
        self.backend.query_tasks_where_unblocked() // dynamic dispatch
    }
}
```

Domain-specific operations are inherent methods on the concrete struct, not on a shared generic trait. GyreContext holds concrete store types, not trait objects:

```rust
pub struct GyreContext {
    pub tasks: TaskStore,       // not Arc<dyn TaskStore>, not TaskStore<B>
    pub memory: MemoryStore,
    pub documents: DocumentStore,
    pub artifacts: ArtifactStore,
    // ...
}
```

## Consequences

- **Clean API surface.** Agent and Gyre implementations never see the Backend type. `fn step(&self, ctx: &GyreContext)` — no generics.
- **Dynamic dispatch on store calls.** Each store method call goes through a vtable. Cost: ~1 nanosecond per call. Store operations are IO-bound (database queries, ~1 millisecond), so vtable overhead is negligible.
- **No generic propagation.** Adding a new Backend doesn't change any function signature above the store layer.
- **Swappable at runtime.** Backend selection is a configuration choice at startup. Tests can use InMemoryBackend while production uses SqliteBackend, with no code changes.
- **Can migrate to generics later** if profiling reveals dynamic dispatch as a bottleneck (unlikely for IO-bound store operations). The migration path: make stores generic, monomorphize at the GyreContext boundary.
- **This differs from the original design** where stores were traits (`Arc<dyn TaskStore>`). The new design moves polymorphism inward (Backend) instead of outward (Store trait). The Store struct IS the API; the Backend is the swappable implementation detail.
