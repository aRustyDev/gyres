---
number: 15
title: Commit to tokio as the async runtime
date: 2026-04-21
status: accepted
---

# 15. Commit to tokio as the async runtime

Date: 2026-04-21

## Status

Accepted

## Context

Every async implementation in gyres (store backends, streaming, CLI, providers) depends on this decision. The question: commit to tokio, or stay runtime-agnostic to support async-std, smol, or future runtimes?

Research findings (see `docs/src/research/agent-infrastructure-landscape.md`):

- Every Rust agent crate (rig, swarm-rs, AutoAgents, rs-agent) depends on tokio directly.
- Every key downstream dependency (reqwest, tonic, axum) requires tokio.
- async-std is officially deprecated. Its successor smol has a niche audience.
- Runtime agnosticism requires spawn/timer/sync abstractions at every call site — substantial complexity for zero practical benefit.
- WASM constraints are platform constraints, not runtime constraints. tokio in WASM supports `sync`, `macros`, `io-util`, `rt`, `time`. Browser WASM lacks `fs`/`net`/`process`/`signal` regardless of runtime. WASM support would require capability-layer abstraction (`trait FileSystem`, `trait HttpClient`), not runtime abstraction.

## Decision

Commit to tokio. Use specific feature flags (`rt`, `sync`, `time`, `macros`) rather than `features = ["full"]` to keep compile times lower and make the WASM-compatible subset explicit.

gyres-core remains runtime-free (no tokio dependency). Crates that need async runtime features (gyres-runtime, gyres-store, gyres-llm, gyres-mcp) depend on tokio directly.

## Consequences

- All async code can use `tokio::spawn`, `JoinSet`, `tokio::sync::*`, `tokio::time::*`, `tokio::select!` directly. No abstraction layers.
- Users must use tokio. This is not a meaningful constraint — they already have tokio via reqwest/tonic/axum.
- If WASM support is needed (gyres-8va), it will be addressed via capability traits and feature flags on individual crates, not by swapping the runtime.
- gyres-core's async traits use `Pin<Box<dyn Future>>` or RPITIT — these are runtime-agnostic by nature. Only the implementations bring in tokio.
