# Issue Dependency Graph & Progressive Work Order

## Dependency Analysis

Issues are grouped into **phases** where each phase's outputs feed the next. Within a phase, items can be worked in parallel. The ordering is designed so knowledge compounds — each discussion builds on what came before.

---

## Phase 0: Foundational Decisions (gates everything)

These must be resolved first. Every other issue depends on at least one of these.

```
gyres-169  Decision: tokio or runtime-agnostic?
    ↓ gates: every async impl, store backends, CLI, providers
gyres-3zu  Evaluate crate namespace strategy: Agent OS vs Agent Harness
    ↓ gates: which crates to create, where traits live
gyres-4zz  Deep dive: Agent Harness vs Agent OS — differences, seams
    ↓ feeds: gyres-3zu (namespace strategy)
```

**Work order:**
1. `gyres-4zz` → `gyres-3zu` (understand OS vs harness, then decide crate namespaces)
2. `gyres-169` (tokio decision — can be parallel with above)

---

## Phase 1: Research That Informs Architecture

These are research tasks whose findings shape the design of core traits and stores. Do these BEFORE designing the features they inform.

```
gyres-d6i  Research: rig crate (competitor analysis)
    ↓ informs: what to build vs what exists, differentiation
gyres-blu  Research: survey multi-agent systems ecosystem
    ↓ informs: gyres-vut (orchestration patterns), gyres-ycc (registry/messaging)
gyres-ykd  Research: RL environment aspects and trait design
    ↓ informs: gyres-4mo (Environment trait)
gyres-nu5  Research: memory systems for agents
    ↓ informs: gyres-deh (MemoryStore), gyres-dvy (GraphDB choice)
gyres-t7q  Research: communication architectures for agents
    ↓ informs: gyres-ycc (AgentRegistry/MessageBus), gyres-a4h (zero-trust)
gyres-dvy  Research: GraphDB options
    ↓ informs: gyres-0zb (Vector+Graph DB), gyres-9mr (Backend trait)
gyres-dta  Research: context compression techniques
    ↓ informs: gyres-5y9 (context assembly pipeline), gyres-c43 (ContextWindow)
gyres-4za  Research: MCP spec evolution
    ↓ informs: gyres-wh0 (MCP bridge), gyres-rlu (tool system)
gyres-8vk  Define agent affordances
    ↓ informs: gyres-pj7 (agent capabilities), gyres-vut (orchestration)
```

**Work order** (parallel tracks):
- Track A (multi-agent): `gyres-blu` → `gyres-t7q` → `gyres-8vk`
- Track B (storage/memory): `gyres-nu5` → `gyres-dvy` → `gyres-0zb`
- Track C (LLM/tools): `gyres-d6i` + `gyres-dta` + `gyres-4za`
- Track D (RL): `gyres-ykd`

---

## Phase 2: Core Design Decisions

These are architectural decisions that must be resolved before implementation. Each depends on Phase 0 + relevant Phase 1 research.

### 2A: Core Trait Expansion (depends on Phase 0 + 1)

```
gyres-9mr  Store abstraction: Backend trait, factory, feature-gated backends
    ↓ depends on: gyres-169 (tokio), gyres-dvy (GraphDB research)
    ↓ gates: every store implementation
gyres-deh  MemoryStore + GraphMemoryStore traits
    ↓ depends on: gyres-9mr (store abstraction), gyres-nu5 (memory research)
    ↓ gates: gyres-5y9 (context pipeline), gyres-3qx (consolidation)
gyres-k1g  Task as first-class type with TaskStore
    ↓ depends on: gyres-9mr (store abstraction), gyres-wy3 (PM workflow)
    ↓ gates: gyres-vut (multi-agent orchestration)
gyres-vz2  ArtifactStore trait and GyreContext integration
    ↓ depends on: gyres-9mr (store abstraction)
    ↓ gates: gyres-24p (artifact types), gyres-tc1 (output routing)
gyres-pj7  Agent identity & capabilities
    ↓ depends on: gyres-8vk (affordances), gyres-blu (ecosystem survey)
    ↓ gates: gyres-ycc (registry), gyres-a4h (zero-trust)
```

### 2B: LLM Architecture Decisions (depends on Phase 0)

```
gyres-ms1  Streaming: where it fits in Agent/Gyre/Provider
    ↓ depends on: gyres-169 (tokio)
    ↓ gates: gyres-ehz (Provider trait), gyres-sk3 (CLI/TUI)
gyres-ehz  Provider trait: streaming, rate limiting, failover
    ↓ depends on: gyres-ms1 (streaming decision)
    ↓ gates: gyres-2g9 (M4: LLM agent loop), gyres-woy (model routing)
gyres-0z5  System prompt construction
    ↓ depends on: gyres-deh (MemoryStore), gyres-rlu (tool system)
    ↓ gates: gyres-5y9 (context pipeline)
gyres-5y9  Context assembly pipeline
    ↓ depends on: gyres-deh (memory), gyres-0z5 (prompt), gyres-dta (compression)
    ↓ gates: gyres-c43 (ContextWindow)
```

### 2C: Multi-Agent Architecture (depends on Phase 1 + 2A)

```
gyres-vut  Multi-agent orchestration deep dive
    ↓ depends on: gyres-blu (ecosystem), gyres-k1g (Task), gyres-pj7 (capabilities)
    ↓ gates: gyres-ycc (AgentRegistry/MessageBus)
gyres-ycc  AgentRegistry + MessageBus traits
    ↓ depends on: gyres-vut (orchestration patterns), gyres-pj7 (identity)
    ↓ gates: gyres-a4h (zero-trust), gyres-7bf (model orchestration)
gyres-wy3  Project planning workflow / Agile mapping
    ↓ depends on: standalone (can start anytime)
    ↓ gates: gyres-k1g (Task type design)
```

### 2D: Cross-Cutting Decisions (depends on Phase 0)

```
gyres-edw  Error recovery and retry strategy
    ↓ depends on: gyres-169 (tokio), gyres-ehz (Provider)
gyres-tc1  Output routing: fan-out from Action
    ↓ depends on: gyres-vz2 (ArtifactStore), gyres-deh (MemoryStore), gyres-ms1 (streaming)
gyres-yo6  Backpressure
    ↓ depends on: gyres-ms1 (streaming), gyres-ycc (messaging)
gyres-zoh  Logging strategy
    ↓ depends on: gyres-b0b (langfuse integration)
gyres-w71  Serialization formats
    ↓ depends on: gyres-ykd (RL research — defines throughput needs)
gyres-8va  WASM compatibility
    ↓ depends on: gyres-169 (tokio — WASM + tokio is constrained)
```

---

## Phase 3: Implementation Milestones (depends on Phase 2)

```
gyres-a24  M1: Finalize gyres-core types and traits
    ↓ depends on: Phase 2A decisions (stores, memory, tasks, artifacts on GyreContext)
    ↓ gates: M2, M3, M4
gyres-9yb  M2: Permission chain with Oso Polar fork
    ↓ depends on: gyres-a24
gyres-e66  M3: Worktree-aware sessions and telemetry
    ↓ depends on: gyres-a24, gyres-9mr (store backends)
gyres-2g9  M4: LLM agent loop with tool registry
    ↓ depends on: gyres-a24, gyres-ehz (Provider), gyres-rlu (tools), gyres-ms1 (streaming)
gyres-wh0  M5: MCP protocol bridge
    ↓ depends on: gyres-2g9, gyres-4za (MCP research)
gyres-gbu  M6: Integration tests and re-export crate
    ↓ depends on: all milestones
```

### Implementation support (parallel with milestones)

```
gyres-zv5  Config loading with figment
    ↓ gates: gyres-sk3 (CLI needs config)
gyres-sk3  CLI/TUI design
    ↓ depends on: gyres-ms1 (streaming), gyres-zv5 (config), gyres-ehz (Provider)
gyres-80f  Testing strategy
    ↓ depends on: gyres-a24 (need traits to test against)
gyres-b0b  gyres-tracing <-> langfuse-rs integration
    ↓ depends on: gyres-a24 (TelemetrySink finalized)
gyres-ntz  Versioning strategy
    ↓ depends on: gyres-3zu (crate namespace decision)
gyres-k5b  Documentation site
    ↓ can start anytime, grows with codebase
```

---

## Phase 4: Security & Operational Hardening (depends on Phase 3)

```
gyres-1wk  Secret management
    ↓ depends on: gyres-ehz (Provider needs keys), gyres-zv5 (config system)
gyres-a4h  ZeroTrust inter-agent comms
    ↓ depends on: gyres-ycc (AgentRegistry/MessageBus must exist first)
gyres-8c4  Graceful shutdown
    ↓ depends on: gyres-sk3 (CLI), gyres-e66 (sessions to save)
gyres-r05  Signal handling
    ↓ depends on: gyres-8c4 (shutdown is the response to signals)
gyres-hnn  Rate limiting
    ↓ depends on: gyres-ehz (Provider), gyres-rlu (tools)
gyres-b56  Cost tracking
    ↓ depends on: gyres-ehz (Provider — token counts), gyres-b0b (telemetry)
gyres-4zi  Resource budgeting
    ↓ depends on: gyres-b56 (cost tracking feeds budget decisions)
```

---

## Phase 5: Advanced Features (depends on Phase 3-4)

```
gyres-4mo  Environment trait for RL
    ↓ depends on: gyres-ykd (RL research), gyres-a24 (core traits), gyres-w71 (serialization)
gyres-woy  Model routing
    ↓ depends on: gyres-ehz (Provider), gyres-pj7 (capabilities)
gyres-rnm  PromptStore for prompt registry
    ↓ depends on: gyres-9mr (store abstraction)
gyres-3qo  Checkpoint/rollback
    ↓ depends on: gyres-deh (MemoryStore), gyres-a24 (Agent trait)
gyres-zui  Plugin/ecosystem system
    ↓ depends on: gyres-rlu (tool system), gyres-3zu (crate strategy)
gyres-c43  ContextWindow in gyres-llm
    ↓ depends on: gyres-5y9 (context pipeline), gyres-dta (compression research)
gyres-p76  Multi-agent observability
    ↓ depends on: gyres-ycc (registry), gyres-b0b (telemetry)
```

---

## Phase 6: Research & Ideation (can run anytime, feeds back into earlier phases)

These are open-ended explorations. They can start at any point and their findings feed back into design decisions.

```
gyres-7bf   Model orchestration techniques for multi-agent
gyres-q9i   Prompting orchestration for multi-agent
gyres-3qx   Memory consolidation (ClawSpring pattern + alternatives)
gyres-dxa   Stub Strategy implementations and explore seams
gyres-24p   ADR/SPEC/DOC as Artifact vs Document trait
gyres-6yd   Should we create langchain-rs?
gyres-je8   Should we create langgraph-rs?
gyres-u8u   A2A protocol research
gyres-qeo   Migration paths from other frameworks
gyres-cuc   Ideation: novel agent harness
gyres-jfg   Agent performance benchmarks
gyres-2d4   Scaled model evaluation framework
gyres-roe   Novel eval methods for multi-agent
gyres-rha   Finetuning Claude for max performance
gyres-id2   Large-scale RL on language models in Gyres
```

---

## Progressive Work Order (recommended session sequence)

Each session builds on prior sessions' outputs.

### Session 1: Foundation
- `gyres-4zz` Agent OS vs Harness deep dive
- `gyres-3zu` Crate namespace strategy (informed by above)
- `gyres-169` Tokio decision

### Session 2: Competitive & Ecosystem Research
- `gyres-d6i` Rig crate analysis
- `gyres-blu` Multi-agent ecosystem survey
- `gyres-ykd` RL environment research

### Session 3: Memory & Storage Research
- `gyres-nu5` Memory systems research
- `gyres-dvy` GraphDB options
- `gyres-0zb` Vector+Graph DB options

### Session 4: Store Abstraction & Core Expansion
- `gyres-9mr` Store abstraction design (informed by Session 3)
- `gyres-deh` MemoryStore traits (informed by Session 3)
- `gyres-vz2` ArtifactStore design

### Session 5: Multi-Agent Architecture
- `gyres-t7q` Communication architectures research
- `gyres-8vk` Agent affordances
- `gyres-pj7` Agent identity & capabilities
- `gyres-wy3` Project planning workflow / Task type

### Session 6: Multi-Agent Design
- `gyres-vut` Multi-agent orchestration deep dive (informed by Session 2, 5)
- `gyres-ycc` AgentRegistry + MessageBus traits
- `gyres-k1g` Task as first-class type (informed by gyres-wy3)

### Session 7: LLM Pipeline
- `gyres-dta` Context compression research
- `gyres-ms1` Streaming decision
- `gyres-ehz` Provider trait
- `gyres-4za` MCP spec evolution

### Session 8: Context & Routing
- `gyres-0z5` System prompt construction
- `gyres-5y9` Context assembly pipeline (informed by Session 3, 7)
- `gyres-tc1` Output routing

### Session 9: Implementation Sprint — Core
- `gyres-a24` M1: Finalize gyres-core (all decisions resolved)
- `gyres-80f` Testing strategy
- `gyres-zv5` Config loading with figment

### Session 10: Implementation Sprint — Infrastructure
- `gyres-9yb` M2: Permission chain
- `gyres-e66` M3: Sessions and telemetry
- `gyres-b0b` Langfuse integration

### Session 11: Implementation Sprint — LLM
- `gyres-2g9` M4: LLM agent loop
- `gyres-rlu` Tool system
- `gyres-sk3` CLI/TUI

### Session 12: Security & Operations
- `gyres-1wk` Secret management
- `gyres-a4h` Zero-trust inter-agent
- `gyres-edw` Error recovery
- `gyres-8c4` + `gyres-r05` Graceful shutdown + signals

### Sessions 13+: Advanced features, research, ideation
(flexible ordering based on what's most valuable)
