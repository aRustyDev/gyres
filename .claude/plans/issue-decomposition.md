# Issue Decomposition & Refinement Plan

## Problem

- 69 of 73 issues have no description — they're titles only
- Several issues cover 3-5 distinct topics that should be separate discussions
- Research tasks lack scoping: what specifically to investigate, what output to produce

## Assessment Categories

### A: Needs decomposition (too broad for one session)
### B: Needs description + discussion questions (right scope, lacks definition)
### C: Scoped and ready (can pick up as-is with minor description)

---

## P1 Issues — Decomposition

### gyres-wy3 [A] Project planning workflow
**Currently:** One massive issue covering 20+ PM concepts, FDD, BDD.
**Decompose into:**
- `wy3a` Survey PM concept hierarchy: Theme → Initiative → Epic → Story → Task → Subtask. Map to a type system. What's a node, what's an edge, what's metadata?
- `wy3b` Map Agile ceremonies/artifacts to Gyres: Sprints, Backlogs, PRDs, ROADMAPs, RACI. Which become TaskStore features vs ArtifactStore outputs vs Strategy config?
- `wy3c` FDD/BDD document types: Feature lists, feature specs, BDD scenarios. How do these relate to the Artifact enum? What's the implicit side-effect story?
- `wy3d` Design the Task type hierarchy: concrete fields, status transitions, decomposition API. Informed by wy3a-c.

### gyres-vut [A] Multi-agent orchestration deep dive
**Currently:** "full spectrum spawn→emergent coordination" — covers 5+ patterns.
**Decompose into:**
- `vut-a` Supervisor pattern: one coordinator delegates to workers. Task assignment, result collection, failure handling. Compare with CrewAI, AutoGen, Claude Code's Agent tool.
- `vut-b` Swarm pattern: peer-to-peer, self-selecting work. Task claiming, conflict resolution, shared state. Compare with OpenAI Swarm.
- `vut-c` Hierarchical Task Decomposition: recursive decomposition, planner+worker split. Compare with HuggingGPT, TaskWeaver.
- `vut-d` Debate/consensus patterns: multi-agent deliberation, voting, critique loops. Compare with Society of Mind, CAMEL.
- `vut-e` Design the orchestration Gyre API: given patterns from vut-a through vut-d, what's the minimal API surface? What goes in gyres-core vs gyres-multi?

### gyres-sk3 [A] CLI/TUI design
**Decompose into:**
- `sk3-a` REPL architecture: input handling, command parsing, history, readline/rustyline. Study claude-code and clawspring REPL patterns.
- `sk3-b` Streaming output rendering: token-by-token display, markdown rendering, code highlighting, spinner/progress. Depends on gyres-ms1 streaming decision.
- `sk3-c` Permission prompt UX: approve/deny/pattern/session/always flow. The graduated approval cache interaction. Study claude-code's permission prompt.
- `sk3-d` Slash command system: registry, parsing, built-in commands, user-defined commands. Study claude-code's ~85 slash commands — which are essential?

### gyres-rlu [A] Tool system
**Decompose into:**
- `rlu-a` Tool ↔ Permission integration: how does PermissionGate interact with ToolRegistry? Per-tool permission classification (read/write/execute). When does the Gyre check permissions — before dispatch, during, or both?
- `rlu-b` Dynamic tool loading: plugins, MCP servers as tool sources, hot-reload. Tool schema validation.
- `rlu-c` Built-in tool set: which tools ship with gyres-llm? Study claude-code's 40+ tools — which are core? Design the ToolDef interface for each.

### gyres-ehz [A] Provider trait
**Decompose into:**
- `ehz-a` Core Provider trait: send messages, receive responses. Sync and streaming variants. Model parameter control (temperature, max_tokens, stop sequences).
- `ehz-b` Rate limiting and retry: per-provider rate limits, exponential backoff, circuit breaker pattern. Where does this live — Provider impl, wrapper, or Gyre?
- `ehz-c` Multi-provider failover: try Provider A, fall back to B. Provider selection strategy. Health checks.
- `ehz-d` Anthropic provider impl: Claude-specific features (extended thinking, tool_use blocks, system prompt caching, prompt caching).

### gyres-9mr [A] Store abstraction
**Decompose into:**
- `9mr-a` Backend trait design: what's the interface? Health check, connection management, schema migration. One backend → multiple stores.
- `9mr-b` Factory pattern: StorageConfig → Stores bundle. Feature-gating strategy (backend-sqlite, backend-surreal, backend-memory, backend-all).
- `9mr-c` InMemoryBackend implementation: the zero-dep default. Must satisfy StateStore + MemoryStore + TaskStore + ArtifactStore.

### gyres-deh [B] MemoryStore + GraphMemoryStore
**Needs description — already designed in docs/src/concepts/memory-architecture.md.**
Discussion questions:
- Is the MemoryEntry schema right? What fields are missing?
- How does embedding generation work? Who calls the embedding model?
- What's the query API for semantic search vs keyword vs graph traversal?
- How does MemoryStore interact with the context assembly pipeline?

### gyres-5y9 [B] Context assembly pipeline
**Needs description.**
Discussion questions:
- What's the pipeline ordering? Memory → RAG → tools → artifacts → system prompt → user message?
- How does token budget allocation work? Each source gets a budget, or first-come-first-served?
- Who owns the pipeline? The Gyre? A dedicated ContextBuilder?
- How does it interact with ContextWindow (gyres-c43)?

### gyres-ms1 [B] Streaming decision
**Needs description.**
Discussion questions:
- Does Agent::step return a Stream<Item=ActionChunk> or a complete Action?
- If streaming, does the Gyre render tokens while buffering the full response?
- Is streaming a Provider concern (Provider returns a stream, Gyre collects) or an Agent concern?
- How does streaming interact with tool-use (tool calls arrive mid-stream)?

### gyres-169 [B] Tokio decision
**Needs description.**
Discussion questions:
- What are the actual constraints? WASM compat, embedded use, library vs application?
- What do we gain from tokio commit? tokio::spawn, tokio::sync, tokio::fs, tokio::process.
- What do we lose? Can't use in non-tokio runtimes (async-std, smol, embassy).
- What do competitors do? rig uses tokio. Most Rust async crates use tokio.

### gyres-4mo [B] Environment trait for RL
**Needs description.** Depends on gyres-ykd research.
Discussion questions:
- Classic RL env: reset() → state, step(action) → (state, reward, done, info). How does this map to our Agent trait?
- Vectorized environments (many envs in parallel)?
- Continuous vs discrete action/observation spaces?
- Gymnasium/Farama compatibility?

### gyres-ycc [B] AgentRegistry + MessageBus
**Needs description.** Depends on gyres-vut orchestration patterns.
Discussion questions:
- Are these on GyreContext or separate?
- Registry: how do agents register capabilities? Discovery protocol?
- MessageBus: point-to-point, pub/sub, or both? Message types?
- Lifecycle: who starts/stops agents? The executor? The supervisor gyre?

### gyres-pj7 [B] Agent identity & capabilities
**Has brief description.** Needs more depth.
Discussion questions:
- What IS a capability? A skill name? A tool set? A model + prompt template?
- How does capability matching work for task assignment?
- Static (declared at creation) vs dynamic (changes during execution)?
- How does this interact with permissions (capability-based access control)?

### gyres-a24 [C] M1: Finalize gyres-core
**Already scoped** in the implementation plan (.claude/plans/2026-04-20-gyres-mvp.md). 11 tasks with full code. Ready for implementation.

---

## P2 Issues — Decomposition

### gyres-1wk [A] Secret management
**Decompose into:**
- `1wk-a` SecretStore trait design: what's the API? get_secret(key), set_secret(key, value), list_keys(). Rotation, expiry, scoping (per-agent, per-project, global).
- `1wk-b` Research backend options: keyring/secure-enclave, vault, 1password CLI, encrypted file. Compare embedded vs external, complexity vs security.
- `1wk-c` PKI and mTLS for inter-agent auth: how does this interact with zero-trust (gyres-a4h)? Is this the same discussion or separate?

### gyres-blu [A] Survey multi-agent systems ecosystem
**Decompose into:**
- `blu-a` Survey Python frameworks: AutoGen, CrewAI, LangGraph, Swarm, CAMEL, MetaGPT, ChatDev. Architecture, patterns, strengths, gaps.
- `blu-b` Survey Rust/non-Python frameworks: rig, swarm-rs, agent-rs, Rivet agent-os. What exists in Rust specifically?
- `blu-c` Survey research: recent papers on multi-agent coordination, emergent behavior, scaling laws for agents. What's the state of the art?

### gyres-nu5 [A] Research: memory systems for agents
**Decompose into:**
- `nu5-a` Survey existing memory architectures: MemGPT, Letta, Zep, mem0, LangChain memory types. What patterns exist?
- `nu5-b` Short-term vs long-term vs working memory: how do these map to our MemoryStore + ContextWindow?
- `nu5-c` Episodic vs semantic vs procedural memory: should MemoryKind reflect these categories?

### gyres-t7q [A] Research: communication architectures for agents
**Decompose into:**
- `t7q-a` Message passing patterns: point-to-point, pub/sub, request/reply, streaming. Which does the MessageBus need?
- `t7q-b` Shared state patterns: blackboard, tuple spaces, CRDT-based. How does this interact with MemoryStore?
- `t7q-c` Protocol comparison: MCP, A2A, custom. What wire format? JSON-RPC, protobuf, custom?

### gyres-dvy + gyres-0zb [A] GraphDB + Vector+Graph research
**Merge and decompose into:**
- `dvy-a` Embedded graph DBs: surrealdb (embedded mode), cozo, oxigraph. Compare query languages, performance, Rust API quality.
- `dvy-b` Graph-as-library: petgraph, indradb. When is a library sufficient vs needing a database?
- `dvy-c` Vector search integration: does the graph DB need native vector support, or bolt on (lance, qdrant)?
- `dvy-d` Benchmark: simple CRUD + graph traversal + vector search on realistic agent workload.

### gyres-zui [A] Plugin/ecosystem system
**Decompose into:**
- `zui-a` Plugin architecture: trait-based, WASM, shared library (.so/.dylib), subprocess? Compare claude-code skills system.
- `zui-b` Discovery and distribution: crates.io, custom registry, git repos? How are plugins found, installed, updated?
- `zui-c` Plugin API surface: what can a plugin provide? Tools, strategies, providers, store backends, artifact sinks?

### Remaining P2 — [B] Need description only
These are right-scoped but need discussion questions added:
- gyres-4zz, gyres-3zu, gyres-0z5, gyres-tc1, gyres-edw, gyres-zv5, gyres-b0b,
  gyres-ntz, gyres-80f, gyres-k1g, gyres-vz2, gyres-8vk, gyres-a4h, gyres-zoh,
  gyres-yo6, gyres-w71, gyres-8va, gyres-woy, gyres-b56, gyres-hnn, gyres-rnm,
  gyres-8c4, gyres-r05, gyres-4za, gyres-d6i, gyres-q9i, gyres-7bf, gyres-k5b

---

## P3 Issues — Mostly [B] or [C]

These are ideation/research tasks that are naturally open-ended. They need scoping descriptions but not decomposition:
- gyres-24p, gyres-3qx, gyres-dxa, gyres-c43, gyres-3qo, gyres-p76,
  gyres-4zi, gyres-6yd, gyres-je8, gyres-u8u, gyres-qeo, gyres-cuc,
  gyres-jfg, gyres-2d4, gyres-roe, gyres-rha, gyres-id2

---

## Summary

| Category | Count | Action |
|---|---|---|
| [A] Needs decomposition | 12 issues → ~40 subtasks | Create child issues |
| [B] Needs description | ~45 issues | Add descriptions + discussion questions |
| [C] Ready as-is | ~6 issues | No action needed |
| Epics (M1-M6) | 6 | Already have implementation plans |
