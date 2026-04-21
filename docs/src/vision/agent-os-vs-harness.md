# Harness and OS: Concepts and Seams

> Living document. Updated as we discover boundaries during development.
> Research grounding: [agent-infrastructure-landscape.md](../research/agent-infrastructure-landscape.md)

## What We Mean By These Terms

We are building both a **harness** and an **OS** in the same project. The terms describe different _concerns_, not different products or codebases.

**Harness concerns** are about running one agent well: the feedback loop, tool dispatch, context management, permissions, session persistence, telemetry. The harness wraps a model and turns it into a functional agent. In the community, this is what Anthropic describes in ["Effective harnesses for long-running agents"](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) — the scaffolding that provides "hands, eyes, memory, and safety boundaries."

**OS concerns** are about managing many agents as a system: identity and lifecycle, scheduling and resource allocation, inter-agent communication, shared memory, isolation, and governance. In the community, AIOS (Rutgers, COLM 2025) and Rivet agentOS are the reference points — they map traditional OS concepts (processes, syscalls, schedulers, IPC) onto agent infrastructure.

**The seam** is where a concern that was internal to one agent becomes shared infrastructure that multiple agents depend on. This is the boundary we are mapping.

## Seams By Concern

Each concern exists on a spectrum from single-agent to multi-agent. The seam is the point where the abstraction must change to support the transition. We identify the seam, what changes at that boundary, and which Gyres abstractions sit on each side.

### Execution

| | Harness (single-agent) | Seam | OS (multi-agent) |
|---|---|---|---|
| **What runs** | One Agent, one Gyre, one loop | An agent needs to delegate work to another agent | Multiple Agents, potentially multiple Gyres, concurrent loops |
| **Who drives** | The Gyre calls `agent.step()` in a loop | A Gyre needs to spawn and manage sub-Gyres | An orchestrating Gyre (Supervisor, Swarm, HTD) manages agent lifecycles |
| **Lifecycle** | Start → run → done | An agent can be paused, resumed, or killed by another | Process-like lifecycle: spawned → running → suspended → terminated |
| **Gyres abstraction** | `Gyre::run()` | Gyre implementations that manage other Gyres | Orchestration Gyres (SupervisorGyre, SwarmGyre) |

**What changes at the seam:** The Gyre goes from "drives one loop" to "manages many loops." This doesn't require new core traits — orchestration Gyres are just Gyre implementations. But it requires new infrastructure: a way to spawn agents, track their state, and coordinate their work.

**Open question:** Does Gyres need an explicit process model (spawn, kill, signal, resource limits), or is that an implementation detail of orchestration Gyres?

### Memory and State

| | Harness | Seam | OS |
|---|---|---|---|
| **Scope** | Private to one agent. Session state, conversation history. | Agent A produces knowledge that Agent B needs | Shared across agents. Knowledge graphs, blackboard pattern. |
| **Persistence** | StateStore: save/load sessions | Artifacts become inputs to other agents | MemoryStore: semantic search, graph traversal, cross-agent retrieval |
| **Ownership** | The agent owns its state | Who owns shared state? Who can write? | Infrastructure owns it. Agents have read/write access governed by permissions. |
| **Gyres abstraction** | `StateStore` on `GyreContext` | `ArtifactStore` as write-side, `MemoryStore` as read-side | `MemoryStore`, `GraphMemoryStore`, memory consolidation Gyres |

**What changes at the seam:** Memory goes from "my notes" to "our knowledge base." The critical design question is ownership and consistency — when Agent A writes a memory entry, when does Agent B see it? Do we need transactions? Conflict resolution?

**Insight from Rivet agentOS:** They solve this with a unified filesystem abstraction — all state is mounted as a directory tree, and agents access it through standard path-based operations. Gyres takes a trait-based approach instead (StateStore, MemoryStore), which is more type-safe but requires explicit design of the sharing model.

### Identity and Discovery

| | Harness | Seam | OS |
|---|---|---|---|
| **Identity** | `AgentId` — a string label, mainly for telemetry and state keying | A second agent needs to find and interact with the first | `AgentId` + capabilities, role, status, resource envelope |
| **Discovery** | Not needed. One agent, known at startup. | "Find me an agent that can review code" | AgentRegistry: register, query by capability, check status |
| **Capabilities** | Implicit in the Agent implementation | Need to advertise what an agent can do | Explicit capability declarations, matchable queries |
| **Gyres abstraction** | `AgentId` on `GyreContext` | — | `AgentRegistry` on `GyreContext` (optional) |

**What changes at the seam:** Identity goes from a label to a contract. In single-agent mode, the agent's capabilities are whatever its `step()` implementation does. In multi-agent mode, capabilities need to be declared, queryable, and matchable — so a supervisor can decide which agent to assign work to.

### Communication

| | Harness | Seam | OS |
|---|---|---|---|
| **Channels** | Agent ↔ Environment (observations in, actions out) | Agent A needs to send feedback to Agent B | Agent ↔ Agent (MessageBus), Agent ↔ Task graph |
| **Pattern** | Request-response (step/action) | One-way messages, pub-sub, or direct addressing? | Multiple patterns: direct send, broadcast, topic-based |
| **Protocol** | Internal (Rust function calls) | Need a message format and routing | Internal (trait calls) or external (MCP, A2A) |
| **Gyres abstraction** | `Agent::step()`, `Agent::feedback()` | — | `MessageBus` on `GyreContext` (optional) |

**What changes at the seam:** Communication goes from "the loop feeds me observations" to "other agents can talk to me." The Gyre is no longer the sole source of an agent's input. This is where MCP (agent↔tool) and A2A (agent↔agent) become relevant — they are the protocol-level expression of this seam.

**Design principle:** Inter-agent communication within a single Gyres process should use trait-based MessageBus (fast, typed). Cross-process or cross-machine communication should use A2A or similar wire protocols. The seam between local and remote communication is a separate concern from the harness/OS seam.

### Permissions and Isolation

| | Harness | Seam | OS |
|---|---|---|---|
| **Scope** | One agent, one permission policy | Agent A spawns Agent B — what can B do? | Per-agent permission policies, delegation chains |
| **Trust** | The user trusts (or constrains) the agent | How much should Agent A trust Agent B's outputs? | Zero-trust between agents. Verify, don't assume. |
| **Resources** | Unbounded (or user-configured limits) | Agent A consumes tokens that Agent B's budget needs | Per-agent resource envelopes: token budgets, time limits, tool access |
| **Isolation** | Process-level (the harness sandbox) | Do agents share a sandbox or get isolated ones? | Per-agent sandboxing: filesystem, network, credential scoping |
| **Gyres abstraction** | `PermissionGate` on `GyreContext` | — | Permission delegation, resource budgeting, sandbox management |

**What changes at the seam:** Permissions go from "what can this agent do?" to "what can this agent allow another agent to do?" This is the capability delegation problem. In OS terms: if I can read a file, can I grant that permission to a subprocess?

**Insight from AIOS:** They model this explicitly with syscalls and access control. Rivet uses per-agent credential scoping and network namespaces. Gyres has the PermissionGate filter chain, which can model delegation via Polar policy rules — but the delegation semantics need to be defined.

### Persistence and Sessions

| | Harness | Seam | OS |
|---|---|---|---|
| **Unit** | One session = one agent's conversation history | Agent A's session references Agent B's outputs | Sessions as a graph: parent sessions, child sessions, shared context |
| **Lifecycle** | Create → append turns → close | Session needs to survive agent restarts, hand off between agents | Durable sessions with snapshot/restore, migration between agents |
| **Git awareness** | Worktree-first: 1 session ↔ 1 worktree | Multiple agents working in the same worktree | Coordinated git operations: who commits? merge conflicts? |
| **Gyres abstraction** | `StateStore`, `SessionState` | — | Session graph, session migration, coordinated persistence |

**What changes at the seam:** Sessions go from "my conversation" to "our project." The critical question: when Agent A delegates a task to Agent B, does B get a child session? A fork of A's session? Its own independent session with a reference back?

### Scheduling and Resources

| | Harness | Seam | OS |
|---|---|---|---|
| **Scheduling** | The Gyre runs the agent. There's nothing to schedule. | Two agents need the same LLM endpoint | Priority, fairness, preemption. Who runs when. |
| **Resources** | Whatever the system provides | Total token budget needs to be divided | Resource accounting: tokens, API calls, compute time, per agent |
| **Backpressure** | Rate limiting at the provider level | Agent A is producing work faster than Agent B can consume | System-level backpressure: slow producers, buffer, drop, or prioritize |
| **Gyres abstraction** | Provider-level rate limiting | — | Scheduler, ResourceBudget, BackpressurePolicy |

**What changes at the seam:** Resource management goes from "use what you need" to "share what we have." This is a classic OS scheduling problem. AIOS addresses it directly with FIFO and Round Robin schedulers. For Gyres, the question is whether scheduling is a core concern or an orchestration Gyre concern.

**Current lean:** Scheduling is an orchestration Gyre concern, not a core trait. A SupervisorGyre decides who runs when. Resource budgets are configuration on GyreContext, enforced by the permission system.

### Telemetry and Observability

| | Harness | Seam | OS |
|---|---|---|---|
| **Granularity** | Per-agent spans: step, tool call, feedback | Need to see causality across agents | Distributed tracing: agent A's span is parent of agent B's span |
| **Attribution** | One agent, clear ownership | Which agent caused this cost? This error? | Per-agent cost attribution, error propagation trees |
| **Gyres abstraction** | `TelemetrySink`, `parent_span` on `GyreContext` | — | Distributed span propagation, cross-agent trace correlation |

**What changes at the seam:** Telemetry goes from "what is this agent doing?" to "what is the system doing?" The existing `parent_span` on GyreContext already supports this — when a supervisor spawns a sub-agent, it passes its span as the parent. But the tooling to visualize and query cross-agent traces is an OS-level concern.

## Architecture Principle

**The harness IS the kernel. The OS features are userspace.**

`gyres-core` defines the kernel: Agent, Gyre, GyreContext, Store traits.

Everything beyond core is a combination of:
1. A **Store implementation** (backend crate)
2. A **Gyre implementation** (orchestration pattern)
3. An **Agent implementation** (domain-specific behavior)
4. A **Strategy struct** (per-run configuration)

No new trait types should be needed in core to support OS features. The seam between harness and OS is crossed by _implementations_, not by new abstractions. A SupervisorGyre is still a `Gyre<SupervisorAgent>`. Memory consolidation is still a specialized Gyre. Task planning is still a specialized Agent.

The richness comes from implementations, not abstractions.

## Crate Boundary Principle

A crate should exist when:
- It has a distinct set of dependencies (e.g., sqlite, surrealdb, anthropic SDK)
- It's independently useful (someone might want gyres-llm without gyres-rl)
- It represents a feature-gate boundary (compile-time opt-in)

A crate should NOT exist just because it's a "concept" — concepts can be modules within a crate.

## Open Questions (to resolve as we build)

1. **Process model**: Does Gyres need explicit agent lifecycle states (spawned/running/suspended/terminated) as a core type, or is this an orchestration Gyre implementation detail?
2. **Memory sharing**: What consistency model for cross-agent memory? Eventual consistency? Read-after-write? Transactions?
3. **Permission delegation**: When Agent A spawns Agent B, how are B's permissions derived from A's? Subset? Explicit grant? Policy-based?
4. **Session graph**: How do child sessions relate to parent sessions? Fork semantics? Shared context?
5. **Scheduling location**: Core concern (GyreContext carries a scheduler) or orchestration Gyre concern?
6. **Resource budgets**: Configuration on GyreContext (simple) or a ResourceManager trait (flexible)?
7. **Local vs remote agents**: Where is the seam between in-process multi-agent (trait calls) and cross-process (A2A protocol)?
