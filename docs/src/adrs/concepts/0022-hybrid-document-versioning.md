---
number: 22
title: Hybrid document versioning — mutable edits, immutable snapshots on transitions
date: 2026-04-22
status: accepted
---

# 22. Hybrid document versioning — mutable edits, immutable snapshots on transitions

Date: 2026-04-22

## Status

Accepted

## Context

Documents (ADRs, PRDs, Roadmaps, Plans) have lifecycle states (Draft → Proposed → Accepted → Superseded). The question: how does content versioning interact with lifecycle transitions?

The original concept doc described artifacts as "append-mostly (immutable once emitted)." But Documents are mutable — an ADR in Draft state gets edited repeatedly before being Proposed. Fully append-only means every typo fix creates a new artifact. Fully mutable means no history.

Alternatives considered:

1. **Fully append-only** — Every edit creates a new version. Full history, but expensive for frequent edits. A Draft ADR with 50 minor edits produces 50 versions, most meaningless.

2. **Fully mutable** — Documents are edited in-place. Lifecycle status is just a field. No version history. Simple, but you lose the ability to see what an ADR said when it was Accepted vs when it was later Amended.

3. **Hybrid** — Edits within a lifecycle state are mutable (in-place). Lifecycle transitions create immutable snapshots. Minor edits don't produce noise; significant state changes are preserved.

## Decision

Option 3: **Hybrid versioning.**

- **Edits within a state** — mutable, in-place. Like working tree changes in git.
- **Lifecycle transitions** — create an immutable `VersionSnapshot`. Like commits in git.
- **Terminal states** — document becomes fully immutable. Further edits are rejected.

```
Document created (Draft)
  ├── edit content ← mutable, no snapshot
  ├── edit content ← mutable, no snapshot
  ├── transition: Draft → Proposed
  │     └── ═══ Snapshot v1 ═══
  ├── edit content ← mutable
  ├── transition: Proposed → Accepted
  │     └── ═══ Snapshot v2 ═══
  ├── transition: Accepted → Superseded
        └── ═══ Snapshot v3 ═══ (terminal, frozen)
```

Each Document type defines its own lifecycle via an associated `Status: Lifecycle` type:

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

DocumentStore enforces the rules:
- `set_status()` validates transitions via `Lifecycle::valid_transitions()`
- Valid transition → snapshot current content + status, then apply new status
- Invalid transition → error
- Terminal status → document frozen, edits rejected

## Consequences

- **Meaningful version history.** Snapshots correspond to lifecycle milestones, not noise from minor edits. Reviewing history shows "what it looked like when Proposed" and "what it looked like when Accepted."
- **VCS-like operations.** The snapshot chain enables `versions()`, `at_version(n)`, and future `diff(v1, v2)`.
- **Delta specs pattern supported.** OpenSpec-style delta specifications (ADDED/MODIFIED/REMOVED) map directly — the delta is the mutable edit, archiving is a lifecycle transition that creates a snapshot.
- **Per-type lifecycle safety.** Each Document type defines its own valid states and transitions via the Lifecycle trait. An ADR can't enter PrdStatus::Review. The associated type on Document enforces this at compile time for typed code paths, with runtime validation in DocumentStore for serialized/dynamic paths.
- **Backend flexibility.** SQLite stores snapshots as a versions table. SurrealDB can use temporal queries. Git-based backends could map snapshots to actual commits. The model is storage-agnostic.
- **Mutable-within-state means no guaranteed history of edits within a state.** If you need every keystroke, that's a StateStore/session concern, not a Document concern. This is intentional — Documents track lifecycle milestones, not edit granularity.
