# Agent OS vs Agent Harness

## The Evolution

Gyres started as an **agent harness** — scaffolding that holds an agent together and runs a feedback loop. The core abstractions (Agent, Gyre, GyreContext) reflect this.

Through design discussions, the scope expanded to an **agent operating system** — a platform that manages memory, scheduling, identity, persistence, communication, and composition of many agents.

## The Seams

### What's "harness" (single-agent loop primitives)

- Agent trait (step, feedback, reset)
- Gyre trait (run loop with strategy)
- GyreContext (shared infrastructure)
- StepResult (control flow)
- Permissions (PermissionGate)
- Session persistence (StateStore)
- Telemetry (TelemetrySink)

### What's "OS" (multi-agent / system-level)

- Task decomposition and dependency graph (TaskStore)
- Agent registry and discovery (AgentRegistry)
- Inter-agent messaging (MessageBus)
- Shared memory and knowledge graphs (MemoryStore, GraphMemoryStore)
- Implicit side-effects (ArtifactStore)
- Orchestration strategies (Supervisor, Swarm, HTD Gyres)
- Memory consolidation (background maintenance)
- Resource budgeting (cost tracking, token limits)
- Checkpoint/rollback (speculative execution)
- Plugin/ecosystem (marketplace, skill packs)

### Where they meet

The "OS" features build on "harness" primitives — they don't replace them.

A SupervisorGyre is still a `Gyre<SupervisorAgent>`. It just happens to manage other agents internally, using AgentRegistry and MessageBus from GyreContext.

Memory consolidation is a specialized Gyre that runs periodically and operates on MemoryStore rather than an external environment.

Task planning is a specialized Agent whose observations are task graphs and whose actions are decomposition decisions.

**The harness IS the kernel. The OS features are userspace.**

## Architecture Principle

gyres-core defines the kernel: Agent, Gyre, GyreContext, Store traits.

Everything else is a combination of:
1. A Store implementation (backend crate)
2. A Gyre implementation (orchestration pattern)
3. An Agent implementation (domain-specific behavior)
4. A Strategy struct (per-run configuration)

No new trait types are needed in core beyond what the Store + Agent + Gyre triangle provides. The richness comes from implementations, not abstractions.

## Crate Boundary Principle

A crate should exist when:
- It has a distinct set of dependencies (e.g., sqlite, surrealdb, anthropic SDK)
- It's independently useful (someone might want gyres-llm without gyres-rl)
- It represents a feature-gate boundary (compile-time opt-in)

A crate should NOT exist just because it's a "concept" — concepts can be modules within a crate.
