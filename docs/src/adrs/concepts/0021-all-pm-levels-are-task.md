---
number: 21
title: All PM hierarchy levels are Task with TaskKind discriminator
date: 2026-04-22
status: accepted
---

# 21. All PM hierarchy levels are Task with TaskKind discriminator

Date: 2026-04-22

## Status

Accepted

## Context

The standard PM hierarchy (Theme → Initiative → Epic → Story → Task → Subtask) needs to be represented in gyres. Each level has different fields, different status lifecycles, and different owners. The question: separate Rust types per level, or a single Task type with a discriminator?

Alternatives considered:

1. **Separate types** — `struct Theme`, `struct Epic`, `struct Story`, each implementing Artifact. Type-safe per level, but duplicates all DAG operations (blocking, dependency resolution, status rollup, tree traversal) across 6+ types. Each needs its own store or store methods.

2. **Higher levels as Documents** — Theme is a Roadmap, Epic is a PRD, only Stories and below are Tasks. The hierarchy crosses the Document/Task boundary. But Themes and Epics have operational semantics (status, assignment, decomposition) that Documents don't — they're work items, not documents.

3. **Single Task type with TaskKind discriminator** — All levels are `Task { kind: TaskKind }`. The TaskStore DAG handles hierarchy uniformly. Status validation is per-kind at runtime.

## Decision

Option 3: **Single Task type with TaskKind discriminator.**

```rust
pub enum TaskKind {
    Theme, Initiative, Epic, Story, Task, Subtask,
    Custom(String),  // telemetry-logged
}
```

The hierarchy is modeled via `parent: Option<TaskId>` — a tree edge separate from the `blocked_by`/`blocks` dependency DAG. Parent-child is hierarchy; blocked-by is dependency. These are distinct relationships.

A single `TaskStatus` enum covers all levels. Valid statuses are validated at runtime per kind (e.g., Themes can only be Active/Retired, Subtasks can only be Ready/Complete).

Kind-specific fields (story points, acceptance criteria, estimates) live in `metadata: serde_json::Value`, not as first-class struct fields. A field is first-class only if TaskStore needs to query/filter on it.

### Auto-transition rules

Parent tasks react to children's status changes:
- When any child becomes Ready → parent bubbles to Ready (if below Ready)
- When last Ready child leaves Ready → parent reverts to kind's resting state
- When all children Complete → parent becomes Ready (eligible for explicit closure)
- Creating a child does NOT change parent status (children can be aspirational)

## Consequences

- **One Store, one type.** TaskStore handles all PM levels uniformly. No duplication of graph operations.
- **Flexible depth.** A solo developer uses flat `Task { kind: Task }`. An enterprise uses all 6 levels. The model supports both.
- **Runtime, not compile-time, kind validation.** A Theme set to InReview is caught at runtime, not by the type checker. This is the trade-off for a single type.
- **Kind-specific fields are untyped.** Story points, acceptance criteria, etc. are `serde_json::Value` in metadata. Type safety for these is left to consumers. This avoids a combinatorial explosion of typed fields per kind.
- **External tool mapping is straightforward.** TaskKind maps directly to Jira's hierarchy levels, Linear's issue types, etc. `Custom(String)` handles platform-specific levels.
- **Parent hierarchy (tree) is separate from blocking dependencies (DAG).** A Task has at most one parent — hierarchy is a tree, not a DAG. Cross-cutting concerns use `blocked_by` or `relate()` edges.
