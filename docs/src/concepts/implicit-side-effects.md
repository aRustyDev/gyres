# Implicit Side-Effects

> **Note:** The artifact type system and store architecture have been formalized in [artifact-taxonomy.md](artifact-taxonomy.md). This document describes the original vision. Where they conflict, the taxonomy document is authoritative.

## Concept

As agents work, they produce knowledge, decisions, and documentation as natural byproducts. Rather than requiring agents to explicitly "write an ADR" or "update documentation," the Gyre captures these as implicit side-effects and routes them to the appropriate domain store.

## How the Gyre Routes Work Products

The Gyre is responsible for recognizing artifact-worthy outputs and routing them:

```
Agent step → response contains architectural decision
  → Gyre detects decision pattern
  → Gyre creates Document (ADR) in DocumentStore
  → Gyre stores rationale as MemoryEntry in MemoryStore
  → Both are searchable via ArtifactStore for future RAG retrieval
```

This creates a virtuous cycle: agent produces knowledge → stored in domain store → retrievable via ArtifactStore as context for future work → agent builds on prior knowledge.

Detection strategies (from most to least autonomous):
- **Pattern-based:** regex/keyword matching on agent output
- **Agent-based:** a dedicated observer agent that evaluates outputs
- **Explicit:** the agent signals via structured output metadata

## Artifact Types

Resolved in [artifact-taxonomy.md](artifact-taxonomy.md). The type hierarchy:

- **Task** — work items at all PM levels (theme, epic, story, task, subtask)
- **MemoryEntry** — mutable agent knowledge and observations
- **Document** — structured documents with typed lifecycle (ADR, PRD, Roadmap, Plan, FeatureSpec)

## Relationship to Memory

Both Memories and Documents are Artifacts. The distinction is access pattern, not audience:

- **MemoryStore** — mutable, semantic recall, agent knowledge. The agent's working memory.
- **DocumentStore** — lifecycle-managed, versioned, structured. Formal records of decisions and plans.

Both are backed by the same database and searchable via ArtifactStore.

## Graph integration

When artifacts and memories are stored in a graph-capable backend, relationships emerge:

```
Agent A ──decided──→ "Use PostgreSQL"
  ├──reason──→ "Need JSONB support"
  ├──affects──→ file:src/db.rs
  └──during──→ task:implement-persistence

file:src/db.rs ──imports──→ crate:sqlx
crate:sqlx ──version──→ "0.7"
```

This knowledge graph grows organically. No explicit "build a knowledge graph" step — it's a side-effect of the artifact and memory emission that happens during normal agent work.
