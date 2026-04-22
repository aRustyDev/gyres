---
number: 19
title: CQRS with shared storage for artifact retrieval
date: 2026-04-22
status: accepted
---

# 19. CQRS with shared storage for artifact retrieval

Date: 2026-04-22

## Status

Accepted

## Context

The store architecture has domain-specific write stores (TaskStore, MemoryStore, DocumentStore) and a cross-type read store (ArtifactStore) for RAG retrieval. The question: how does ArtifactStore see data that domain stores write?

This is fundamentally a CQRS (Command Query Responsibility Segregation) problem — the write model (domain stores with rich semantics) is separate from the read model (ArtifactStore with cross-type search).

Alternatives considered:

1. **Synchronous dual-write** — Each domain store also writes to ArtifactStore within the same call. Strong consistency, but every store is coupled to ArtifactStore. Adding a new domain store requires wiring it to ArtifactStore.

2. **Event-driven projection** — Domain stores emit events. ArtifactStore subscribes and projects. Decoupled, but introduces eventual consistency, requires an event bus, and adds significant complexity for a system that starts single-process.

3. **Shared storage with query views** — All stores share one Backend instance (already the design from the store abstraction). ArtifactStore queries the same database tables that domain stores write to. No projection, no event bus, no consistency gap.

## Decision

Option 3: **Shared storage with query views.**

ArtifactStore is a read-only view over the same Backend that domain stores write to. It has no `emit()` method and no data of its own. Its `search()` implementation is a cross-table query (e.g., SQLite UNION + FTS5, SurrealDB cross-collection) over rows written by TaskStore, MemoryStore, and DocumentStore.

```
TaskStore ──writes──→ ┐
MemoryStore ──writes──→ ├──→ SharedBackend ←──reads── ArtifactStore
DocumentStore ──writes──→ ┘
```

The store roles formalize as:
- **MutStore** — read+write, domain-specific operations (TaskStore, MemoryStore, DocumentStore)
- **RefStore** — read-only cross-type search (ArtifactStore)
- **Sink** — write-only fire-and-forget (TelemetrySink)

## Consequences

- **Zero consistency gap.** ArtifactStore reads committed data immediately. No eventual consistency, no stale reads, no projection lag.
- **No event bus needed.** Simplifies the initial architecture. An event system can be added later for other purposes (notifications, webhooks) without being load-bearing for data consistency.
- **ArtifactStore implementation is coupled to domain store schemas.** Its cross-table query must know the table structures of all domain stores. This is the trade-off — decoupling at the data level instead of the code level.
- **Adding a new domain store requires updating ArtifactStore's query.** When a new MutStore is added, ArtifactStore's search must be extended to include its tables. This is manageable since new domain stores are infrequent.
- **The `emit()` method from the original ArtifactStore design is removed.** Domain stores write directly; ArtifactStore only reads.
- **Backend implementations must support cross-store queries** — SQLite via UNION/FTS5, SurrealDB via cross-collection queries, InMemory via iterating all stores.
