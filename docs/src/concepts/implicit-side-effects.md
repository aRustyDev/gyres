# Implicit Side-Effects

## Concept

As agents work, they produce knowledge, decisions, and documentation as natural byproducts. Rather than requiring agents to explicitly "write an ADR" or "update documentation," the harness captures these as implicit side-effects.

## The ArtifactStore

ArtifactStore is a Store trait (readable + writable), not just a sink:
- **Write path:** Gyre emits artifacts as agents work.
- **Read path:** Artifacts are retrievable via search for RAG context — the agent's own outputs become future inputs.

This creates a virtuous cycle: agent produces knowledge → stored as artifact → retrieved as context for future work → agent builds on prior knowledge.

## Artifact Types (not yet formalized — see gyres-24p)

Candidates for first-class artifact types:
- **Decision** — architectural or design choice with context and rationale (ADR-like)
- **Documentation** — generated docs, READMEs, API references
- **Plan** — task decomposition, implementation strategy
- **Specification** — formal requirements or contracts
- **Note** — Zettelkasten-style atomic knowledge unit
- **Relationship** — a discovered connection between entities

Open question: are these variants of an `Artifact` enum, or implementations of a `Document` trait? Tracked in gyres-24p.

## Where the Gyre emits

The Gyre is responsible for recognizing artifact-worthy outputs:

```
Agent step → response contains architectural decision
  → Gyre detects decision pattern
  → Gyre emits Artifact::Decision to ArtifactStore
  → Decision is searchable for future RAG retrieval
```

Detection can be:
- **Pattern-based:** regex/keyword matching on agent output
- **Agent-based:** a dedicated "observer" agent that evaluates outputs
- **Explicit:** the agent itself signals "this is a decision" via metadata

## Relationship to Memory

Artifacts and memories are related but distinct:

| | Memory (MemoryStore) | Artifact (ArtifactStore) |
|---|---|---|
| Audience | The agent (machine) | Humans + agents |
| Format | Structured for retrieval | Structured for reading |
| Lifecycle | Mutable (updated, consolidated, forgotten) | Append-mostly (immutable once emitted) |
| Purpose | Context for future steps | Record of what happened |
| Example | "User prefers TypeScript" | ADR: "Chose TypeScript over Python because..." |

Both can be backed by the same database. Both are searchable. The distinction is semantic, not technical.

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
