# Roadmap Tiers

## Tier 1 — MVP + Core OS

### Multi-agent orchestration
- Biggest goal. Agent registry, messaging, task-based delegation.
- Orchestration patterns as Gyre implementations (Supervisor, Swarm, HTD).
- Shared memory as coordination primitive (blackboard pattern).

### Environment abstraction
- Make the RL path first-class: `Environment` trait alongside `Agent` and `Gyre`.
- Explore alternative agent types beyond LLM and RL.

### Deployment: CLI/TUI
- Local execution only. Interactive terminal interface.
- Worktree-first git integration.

### Memory
- Auto-memory: local/remote + semi/fully-structured + text/graph/relational.
- MemoryStore + GraphMemoryStore traits.
- Embedded graph DB backend (SurrealDB or Cozo).
- Memory as implicit side-effect of agent work.

## Tier 2 — Ecosystem

### Plugin system
- Parity with Claude Code Skills/Plugin marketplaces.
- Support non-Claude plugin marketplaces too.
- Tool packs, strategy packs, provider packs.

### Safety: Reward hacking detection
- RL-specific: detect when agents exploit reward functions.

## Tier 3 — Learning

### Prompt optimization (DSPy-style)
- Optimize prompts across episodes based on outcomes.

### Policy improvement across episodes
- RL: policy gradient updates between episodes.
- LLM: strategy adaptation based on success metrics.

### Meta-learning
- Learning to learn: agents that improve their own learning strategies.

### Memory consolidation
- Periodic compression, synthesis, and cross-referencing of accumulated memories.
- Background "consolidator agent" pattern (from ClawSpring).
- Research alternatives.

## Tier 4 — Platform

### Deployment modes
- Distributed agents across machines.
- Serverless (Lambda/Cloud Run).
- Remote execution (agents on cloud infra).
- Headless daemon mode.
- GUI/App (desktop, web).

### Safety: Full suite
- Output filtering / guardrails.
- Constitutional AI-style self-critique.
- Sandboxing beyond file permissions (network isolation, resource limits).

## Cross-Tier Concerns

These span multiple tiers and evolve incrementally:

### Implicit side-effects
- ADR, documentation, memory entries generated automatically as agents work.
- ArtifactStore enables RAG retrieval of agent-produced content.
- Tier 1 (basic artifact emission) → Tier 3 (knowledge graph construction).

### Observability
- Per-agent span-based telemetry (Tier 1).
- Multi-agent system observability (Tier 1 with multi-agent).
- Resource budgeting (Tier 2).
- Real-time dashboard (Tier 4).

### Store abstraction
- Separate traits, shared backends.
- InMemory + JSON (Tier 1) → SQLite (Tier 1) → SurrealDB (Tier 2) → distributed (Tier 4).
