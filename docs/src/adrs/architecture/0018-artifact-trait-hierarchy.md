---
number: 18
title: Artifact as trait hierarchy, not struct
date: 2026-04-22
status: accepted
---

# 18. Artifact as trait hierarchy, not struct

Date: 2026-04-22

## Status

Accepted

## Context

"Artifact" in gyres is overloaded. Implicit side-effects (decisions, documentation), PM documents (PRDs, ADRs, ROADMAPs), tasks, and memories are all "things an agent produces." We need a type system that covers all of these without collapsing into "everything is a generic struct with a kind string."

The core tension: agent work products have fundamentally different access patterns (Tasks have DAG semantics, Memories are mutable with semantic search, Documents have lifecycles and versioning) but share common identity and provenance needs (id, producer, timestamps, metadata).

Alternatives considered:

1. **Flat struct with kind: String** — `Artifact { kind: "adr", content: "..." }`. Simple but loses type safety. Can't enforce ADR-specific lifecycle states at the type level. The current design before this decision.

2. **Artifact enum with variants** — `enum Artifact { Task(Task), Memory(Memory), Document(Document) }`. Closed set — gyres defines all types. Users can't add custom artifact types without forking.

3. **Artifact as trait with Document subtrait** — `trait Artifact`, `trait Document: Artifact`, concrete types implement the appropriate trait. Open for extension, type-safe, different stores handle different trait impls.

4. **Separate unrelated types** — Task, Memory, and Document share no base type. Each store is fully independent. Loses the ability to search across all artifact types.

## Decision

Option 3: **Artifact as a trait hierarchy.**

```
Artifact (trait)
├── Task              — implements Artifact directly
├── MemoryEntry       — implements Artifact directly
└── Document (trait: Artifact)
    ├── Adr           — implements Document (gets Artifact via blanket impl)
    ├── Prd
    ├── Roadmap
    ├── Plan
    └── FeatureSpec
```

The Artifact trait provides self-describing getters (id, producer, title, content, kind, timestamps, metadata). It does not provide mutation methods — those live on the stores.

The Document trait extends Artifact with an associated `Status: Lifecycle` type, enabling per-document-type lifecycle state machines enforced at compile time.

A blanket impl provides Artifact for all Document types:
```rust
impl<T: Document> Artifact for T { ... }
```

Task and MemoryEntry implement Artifact directly (they are not Documents).

## Consequences

- New artifact types can be added by implementing Artifact or Document — the system is open for extension.
- ArtifactStore can provide cross-type search over anything implementing Artifact via the shared trait interface.
- Each Document type defines its own lifecycle states via the associated type, with compile-time enforcement of valid transitions.
- Task and Memory are clearly distinct from Documents — they have different access patterns and different stores.
- The trait hierarchy adds a level of abstraction compared to a flat struct, but this is justified by the genuinely different behavior of each artifact category.
- Downstream: ArtifactKind enum provides runtime type discrimination for filtering and serialization.
