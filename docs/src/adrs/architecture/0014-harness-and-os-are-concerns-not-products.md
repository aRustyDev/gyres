---
number: 14
title: Harness and OS are concerns not products
date: 2026-04-21
status: accepted
---

# 14. Harness and OS are concerns not products

Date: 2026-04-21

## Status

Accepted

## Context

Gyres started as an "agent harness" and expanded scope to an "agent operating system." The community uses these terms inconsistently — "Agent OS" is more positioning claim than recognized category (see `docs/src/research/agent-infrastructure-landscape.md`). We needed to decide: are we building a harness, an OS, or both? And what do those terms actually mean architecturally?

Research findings: the community associates "harness" with single-agent loop infrastructure (tool dispatch, context management, permissions, session persistence). "OS" implies process model, scheduling, isolation, inter-agent communication, and resource governance. AIOS (Rutgers) and Rivet agentOS are the reference implementations. No Rust project occupies either layer.

## Decision

Harness and OS describe different **concerns**, not different products or codebases. Gyres builds both.

- **Harness concerns**: running one agent well (feedback loop, tools, context, permissions, sessions, telemetry)
- **OS concerns**: managing many agents as a system (identity, lifecycle, scheduling, IPC, shared memory, isolation, governance)
- **The seam**: where a single-agent concern becomes shared multi-agent infrastructure

The harness IS the kernel (`gyres-core`: Agent, Gyre, GyreContext, Store traits). OS features are userspace — expressed as implementations of core traits, not new abstractions.

Seven seams identified between harness and OS concerns:

1. **Execution** — Gyre drives one loop → orchestration Gyre manages many loops
2. **Memory/State** — private session state → shared knowledge base
3. **Identity/Discovery** — AgentId label → AgentId + capabilities + registry
4. **Communication** — observations/actions through Gyre → inter-agent MessageBus
5. **Permissions/Isolation** — one policy → delegation chains, per-agent sandboxes
6. **Persistence/Sessions** — one conversation → session graphs, coordinated git
7. **Scheduling/Resources** — use what you need → share what we have

## Consequences

- OS features must be expressible as Gyre implementations, Agent implementations, Store implementations, or Strategy structs. If an OS feature requires a new core trait, that signals a design problem — revisit the core abstractions.
- Each seam has open questions (documented in `docs/src/vision/agent-os-vs-harness.md`) that will be resolved during implementation. This is intentional — we learn the seams by building, not by specifying upfront.
- The crate boundary principle is unaffected: crates exist for dependency boundaries and feature gates, not for conceptual boundaries.
- We avoid the "Agent OS" label as a marketing claim. The architecture stands on its own.
