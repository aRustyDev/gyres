---
number: 16
title: Crate namespace strategy
date: 2026-04-21
status: accepted
---

# 16. Crate namespace strategy

Date: 2026-04-21

## Status

Accepted

## Context

With 7 crates published and more planned, we needed naming conventions and a clear namespace strategy. Research covered: tokio, serde, tower, axum, bevy, sqlx, diesel, sea-orm naming patterns; crates.io squatting policy (RFC 3463); hyphen vs underscore conventions.

Key findings:
- Hyphens are the majority convention in the modern Rust async ecosystem (tokio, tower, axum, sqlx).
- Flat naming unless a clear category has multiple variants.
- RFC 3463 prohibits publishing empty crates to reserve names.
- Diesel uses feature flags for backends; sqlx uses separate crates. Feature flags are simpler for users.

## Decision

### Workspace crates (10 total)

| Crate | Purpose | Status |
|---|---|---|
| `gyres` | Re-export umbrella | Published |
| `gyres-core` | Agent, Gyre, GyreContext, Store traits, types | Published |
| `gyres-runtime` | Executor, sessions, config loading | Published |
| `gyres-polar` | Forked Oso Polar engine for permissions | Published |
| `gyres-llm` | LLM providers, context pipeline, tool system | Published |
| `gyres-tracing` | Telemetry bridge (TelemetrySink to tracing/OTEL) | Published |
| `gyres-mcp` | MCP protocol bridge | Published |
| `gyres-store` | Store backend implementations, feature-gated | Future |
| `gyres-rl` | RL Environment trait, RL-specific Gyres | Future |
| `gyres-orchestra` | Multi-agent orchestration: AgentRegistry, MessageBus, Supervisor/Swarm/HTD Gyres | Future |

### Naming conventions

- **Hyphens** in Cargo.toml package names (Rust code uses underscores automatically).
- **Flat naming** — no sub-namespaces except where justified.
- **No name squatting** — only publish when there's real code behind the crate.

### Store backends: single crate with feature flags

`gyres-store` with feature flags: `backend-sqlite`, `backend-surreal`, `backend-memory`, `backend-all`.

Rationale: simpler dependency story for users (one crate to add), the `Stores::from_config()` factory lives naturally in one place, and feature flags keep unused backend dependencies out of the build. If a backend grows complex enough to warrant extraction (heavy deps, large code surface), it can be split out later.

### Dropped proposals

- **`gyres-plugin`** — dropped. "Plugin" conflates several concerns (tool packs, strategy packs, provider packs) that are better served by existing crates (`gyres-llm` for tools/providers, trait implementations for strategies). Runtime plugin loading (WASM, shared libraries) is a Tier 2 feature — if needed, it's either a `gyres-runtime` concern or a new crate at that point.
- **`gyres-memory`**, **`gyres-task`**, **`gyres-artifacts`** — dropped. MemoryStore, TaskStore, and ArtifactStore are traits in `gyres-core`. Implementations live in `gyres-store` backends. No separate crates needed.
- **`gyres-multi`** — replaced by `gyres-orchestra`. "Multi" is too vague; "orchestra" is a noun that evokes coordination without being overly long.

## Consequences

- 10 crates total (7 current + 3 future). Tighter than the 13+ originally proposed.
- Store trait definitions stay in `gyres-core`. Backend implementations go in `gyres-store`.
- Users add `gyres-store = { features = ["backend-sqlite"] }` — one dependency, one feature flag.
- `gyres-orchestra` name signals "this is where multi-agent lives" without overloading any existing crate.
- Future crates follow the same conventions: hyphens, flat, publish only with real code.
