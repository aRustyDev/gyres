---
status: accepted
date: 2026-04-20
tags: [permissions, gyres-core]
---

# PermissionContext is a Non-Exhaustive Enum in a Vec

## Context

Permission policies need environmental context (branch, worktree, agent role) alongside the action and resource. The question was whether to use a struct with optional fields or an enum.

## Decision

`PermissionContext` is a `#[non_exhaustive]` enum. `PermissionRequest` carries `context: Vec<PermissionContext>`.

Variants: `Worktree`, `Branch`, `Commit`, `IsMainWorktree`, `AgentRole`, `Custom { key, value }`.

## Consequences

- Composable: the Gyre builds context incrementally, only adding what's available.
- Extensible: new variants added in minor versions without breaking policies.
- No Option ceremony: missing context is simply absent from the Vec.
- Polar policies iterate the list and match on variant types.

## Alternatives Considered

- **Struct with optional fields** (`worktree: Option<WorktreePath>, branch: Option<Branch>, ...`): Every field is optional because context varies by environment. Verbose, can't extend without breaking changes.
- **HashMap<String, Value>**: Stringly-typed, no compile-time safety, no IDE autocomplete.
