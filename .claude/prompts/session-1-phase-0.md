# Session 1: Phase 0 Foundation Decisions

We're continuing work on **gyres** (`~/code/oss/gyres`), a Rust agent OS. Session 0 bootstrapped the project — repos created, gyres-core spec'd, 115 beads issues tracked with full dependency graph.

## Context to load

Before starting, read these files to understand where we left off:
1. `.claude/plans/issue-dependency-graph.md` — the phased work order (we're starting Phase 0)
2. `docs/src/vision/agent-os-vs-harness.md` — the OS vs harness framing
3. `docs/src/concepts/store-abstraction.md` — store architecture (feeds into namespace discussion)
4. `crates/gyres-core/SPEC.md` — current spec (the decisions these 3 issues may modify)

## Issues to work

Run `bd ready` in `~/code/oss/gyres` to see unblocked work. This session tackles:

### 1. `gyres-4zz` — Agent Harness vs Agent OS deep dive
`bd show gyres-4zz` for full context. This is the conceptual foundation — where are the architectural seams between "harness" (single-agent loop) and "OS" (multi-agent management)? Output should be written to `docs/src/vision/` and inform the namespace decision.

### 2. `gyres-3zu` — Crate namespace strategy  
`bd show gyres-3zu` for full context. Blocked by gyres-4zz. Once we understand the OS/harness boundary, decide: which crate names to claim (gyres-memory? gyres-multi? gyres-orchestration?), naming conventions, what belongs where.

### 3. `gyres-169` — Tokio decision
`bd show gyres-169` for full context. Can be done in parallel with 4zz/3zu. This gates every async implementation. Strong lean toward committing to tokio — verify that's right by examining the constraints.

## How to work

- Use `bd update <id> --claim` when starting each issue
- Write findings to `docs/src/` (vision, concepts, or adrs as appropriate)
- Create ADRs via `adrs` CLI for architectural decisions
- Use `bd close <id> --reason "..."` when resolved
- Push git + `bd dolt push` when done

## What gates on these decisions

After this session, 37 currently-blocked issues become unblockable. The tokio decision alone unblocks: store abstraction, streaming, WASM compat, error recovery. The namespace decision unblocks: versioning strategy. The OS/harness deep dive informs everything.
