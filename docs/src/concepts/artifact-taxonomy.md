# Artifact Taxonomy

> The type system for agent work products, store architecture, and data flow.
> Resolves: gyres-b1h. Informed by: gyres-asn, gyres-wy3, gyres-24p, gyres-3m1.

## Core Principle

An **Artifact** is anything an agent produces — explicitly or as an implicit side-effect of work. The Artifact trait is the base type for all durable agent work products. The distinction between artifact types is ontological: a Task *is* an Artifact, a Memory *is* an Artifact, an ADR *is* an Artifact.

Ephemeral data (session turns, telemetry spans) is not an Artifact. Only durable, cross-session work products qualify.

## Type Hierarchy

```
Artifact (trait)
├── Task              — PM hierarchy via `kind` field (theme|epic|story|task|subtask)
├── MemoryEntry       — agent knowledge, mutable, recallable
└── Document (trait: Artifact)
    ├── Adr           — Proposed → Accepted → Deprecated → Superseded
    ├── Prd           — Draft → Review → Approved → Amended
    ├── Roadmap       — Draft → Active → Archived
    ├── Plan          — Draft → Active → Complete
    ├── FeatureSpec   — Draft → Accepted → Implemented
    └── Custom(String)— telemetry-logged to signal new kinds needed
```

### Artifact Trait

The base trait for all agent work products. Self-describing getters only — mutations go through stores.

```rust
pub trait Artifact: Send + Sync {
    fn id(&self) -> &ArtifactId;
    fn producer(&self) -> &AgentId;
    fn title(&self) -> &str;
    fn content(&self) -> &str;
    fn kind(&self) -> ArtifactKind;
    fn created_at(&self) -> SystemTime;
    fn updated_at(&self) -> SystemTime;
    fn metadata(&self) -> &serde_json::Value;
}
```

### Document Trait

Extends Artifact with lifecycle status (via associated type), version history, and optional file-system path.

```rust
pub trait Lifecycle: Serialize + DeserializeOwned + Clone + PartialEq {
    fn is_terminal(&self) -> bool;
    fn valid_transitions(&self) -> Vec<Self>;
}

pub trait Document: Artifact {
    type Status: Lifecycle;
    fn status(&self) -> &Self::Status;
    fn path(&self) -> Option<&Path>;
}
```

Each document type defines its own lifecycle states:

```rust
enum AdrStatus { Proposed, Accepted, Deprecated, Superseded }
impl Lifecycle for AdrStatus { ... }

enum PrdStatus { Draft, Review, Approved, Amended }
impl Lifecycle for PrdStatus { ... }
```

A blanket impl provides Artifact for all Document types:

```rust
impl<T: Document> Artifact for T { ... }
```

Task and MemoryEntry implement Artifact directly (they are not Documents).

### Kind Enums

```rust
pub enum ArtifactKind {
    Task,
    Memory,
    Document(DocumentKind),
}

pub enum DocumentKind {
    Adr,
    Prd,
    Roadmap,
    Plan,
    FeatureSpec,
    Custom(String),
}
```

`Custom(String)` usage emits a telemetry event so operators can identify emerging document types and promote them to first-class variants.

## Store Architecture (CQRS with Shared Storage)

Three access pattern roles:

| Role | Trait | Writes | Reads | Implemented by |
|------|-------|--------|-------|----------------|
| **Sink** | `Sink` | Fire-and-forget | No | TelemetrySink |
| **MutStore** | `MutStore` | CRUD + domain ops | Domain-specific queries | TaskStore, MemoryStore, DocumentStore |
| **RefStore** | `RefStore` | No | Cross-type search | ArtifactStore |

### Domain Stores (MutStore — Write Side)

Concrete structs wrapping `Arc<dyn Backend>`. Domain-specific operations are inherent methods, not on the MutStore trait.

**TaskStore** — manages the PM hierarchy as a DAG:

```rust
pub struct TaskStore { backend: Arc<dyn Backend> }
impl MutStore for TaskStore { ... }
impl TaskStore {
    fn ready_tasks(&self) -> Result<Vec<Task>, GyreError>;
    fn blockers(&self, id: &TaskId, depth: Option<usize>) -> Result<Vec<Task>, GyreError>;
    fn dependents(&self, id: &TaskId, depth: Option<usize>) -> Result<Vec<Task>, GyreError>;
    fn add_dependency(&self, blocked: &TaskId, blocked_by: &TaskId) -> Result<(), GyreError>;
}
```

Themes, Epics, Stories, Tasks, and Subtasks are all `Task` with a `kind` field. The hierarchy is modeled via parent-child blocking relationships in the DAG. Future adapter crates map between gyres Tasks and external platforms (Jira, Linear, Asana).

**MemoryStore** — mutable agent knowledge:

```rust
pub struct MemoryStore { backend: Arc<dyn Backend> }
impl MutStore for MemoryStore { ... }
impl MemoryStore {
    fn recall(&self, query: &str, filter: &MemoryFilter) -> Result<Vec<MemoryEntry>, GyreError>;
    fn forget(&self, id: &MemoryId) -> Result<(), GyreError>;
    fn traverse(&self, from: &MemoryId, relation: &str, depth: Option<usize>) -> Result<Vec<MemoryEntry>, GyreError>;
}
```

Loose observations, learned knowledge, and feedback lessons are all MemoryEntries. The Artifact/Memory audience distinction ("machine vs human+agent") is not a type boundary — memories are artifacts with a mutable access pattern.

**DocumentStore** — structured documents with lifecycle and versioning:

```rust
pub struct DocumentStore { backend: Arc<dyn Backend> }
impl MutStore for DocumentStore { ... }
impl DocumentStore {
    fn set_status<D: Document>(&self, id: &ArtifactId, status: D::Status) -> Result<(), GyreError>;
    fn versions(&self, id: &ArtifactId) -> Result<Vec<VersionSnapshot>, GyreError>;
    fn at_version(&self, id: &ArtifactId, version: u32) -> Result<Option<VersionSnapshot>, GyreError>;
}
```

### ArtifactStore (RefStore — Read Side)

Read-only cross-type query view. **Has no data of its own.** Queries the same backend tables that domain stores write to.

```rust
pub struct ArtifactStore { backend: Arc<dyn Backend> }
impl RefStore for ArtifactStore { ... }
impl ArtifactStore {
    fn search(&self, query: &str, filter: &ArtifactFilter) -> Result<Vec<ArtifactRef>, GyreError>;
    fn similar(&self, id: &ArtifactId) -> Result<Vec<ArtifactRef>, GyreError>;
    fn get(&self, id: &ArtifactId) -> Result<Option<ArtifactRef>, GyreError>;
    fn related(&self, id: &ArtifactId, relation: Option<&str>) -> Result<Vec<ArtifactRef>, GyreError>;
}
```

### Shared Storage — No Consistency Gap

All stores share one `Arc<dyn Backend>` instance. ArtifactStore's `search()` is a cross-table query (e.g., SQLite UNION + FTS5, SurrealDB cross-collection) over the same rows that domain stores write. No projection step, no event bus, no eventual consistency problem.

### Relationships

Relationships are store-managed edges, not embedded on artifacts. This keeps artifacts as value types and lets the Backend choose the storage strategy (graph DB uses native edges, SQLite uses a join table).

```rust
// Write side: domain stores create relationships between artifacts they manage.
// e.g., TaskStore can relate a Task to another Task or to a Document.
fn relate(&self, from: &ArtifactId, relation: &str, to: &ArtifactId) -> Result<(), GyreError>;

// Read side: ArtifactStore queries relationships across all artifact types.
// This is a RefStore operation — it reads relationship edges written by domain stores.
fn related(&self, id: &ArtifactId, relation: Option<&str>) -> Result<Vec<ArtifactRef>, GyreError>;
```

## Document Versioning (Hybrid Model)

Edits within a lifecycle state are mutable (in-place). Lifecycle transitions create immutable snapshots.

```rust
pub struct VersionSnapshot {
    pub version: u32,
    pub content: String,
    pub status_before: serde_json::Value,
    pub status_after: serde_json::Value,
    pub author: AgentId,
    pub created_at: SystemTime,
}
```

### Flow

```
Document created (Draft)
  │
  ├── edit content ← mutable, in-place, no snapshot
  ├── edit content ← mutable, in-place, no snapshot
  │
  ├── transition: Draft → Proposed
  │     └── ═══ Snapshot v1 ═══  (captures content + status change)
  │
  ├── edit content ← mutable again
  │
  ├── transition: Proposed → Accepted
  │     └── ═══ Snapshot v2 ═══
  │
  ├── transition: Accepted → Superseded { by: new_adr_id }
        └── ═══ Snapshot v3 ═══  (terminal — document is frozen)
```

### Rules

- `set_status()` validates transitions via `Lifecycle::valid_transitions()`
- Valid transition: snapshot current state, then apply new status
- Invalid transition: return error
- Terminal status (`is_terminal() == true`): document becomes fully immutable, further edits rejected

### VCS-like Operations

The version chain enables:
- `versions()` — list all snapshots (like git log)
- `at_version(n)` — retrieve content at a specific snapshot (like git checkout)
- Future: `diff(v1, v2)` — compare snapshots

## Agent Loop Data Flow

The agent loop (Observe, Think, Act, Feedback) generates ephemeral data and durable knowledge. The Gyre is responsible for recognizing which outputs are worth promoting to durable stores.

See [agent-loop-data-flow.md](agent-loop-data-flow.md) for the full data flow diagram and routing rules.

### Summary

| Data | Store | Role | Why |
|------|-------|------|-----|
| Raw O/T/A/F turns | StateStore | MutStore | Ephemeral session context |
| Spans, metrics, reasoning traces | TelemetrySink | Sink | Operational instrumentation |
| Learned knowledge, observations | MemoryStore | MutStore | Durable, agent-recallable |
| Work decomposition (theme→task) | TaskStore | MutStore | Structured DAG with status transitions |
| ADRs, PRDs, specs, plans | DocumentStore | MutStore | Structured docs with lifecycle + versioning |
| Cross-type "give me context" | ArtifactStore | RefStore | Read-only view over Memory+Task+Document |

StateStore and TelemetrySink are **outside** the artifact system. Only MemoryStore, TaskStore, and DocumentStore hold Artifacts.

## Implicit Side-Effects

The Gyre watches agent output and promotes valuable content to the appropriate domain store. The agent doesn't need to explicitly say "save this as an ADR" — the Gyre detects the pattern and routes it.

Detection strategies (from most to least autonomous):
1. **Pattern-based:** regex/keyword matching on agent output
2. **Agent-based:** a dedicated observer agent that evaluates outputs
3. **Explicit:** the agent signals via structured output metadata

See [implicit-side-effects.md](implicit-side-effects.md) for the original concept. This document supersedes the store design in that document.

## Deferred Concerns

- **RAG retrieval mechanics** — embedding strategy, ranking, chunking, hybrid search. Tracked in gyres-27b.
- **External PM tool adapters** — Jira/Linear/Asana sync via adapter crates. Seam: Task `kind` field + `metadata` + future `external_refs` field.
- **FDD/BDD artifacts** — feature specs and BDD scenarios map to Document (FeatureSpec kind) and Task respectively. Details in gyres-3m1.

## Relationship to Other Documents

- [store-abstraction.md](store-abstraction.md) — the Backend/Store architecture this builds on
- [agent-loop-data-flow.md](agent-loop-data-flow.md) — O/T/A/F routing rules
- [implicit-side-effects.md](implicit-side-effects.md) — the original vision, now refined here
- [agent-os-vs-harness.md](../vision/agent-os-vs-harness.md) — the Memory/State seam this resolves
