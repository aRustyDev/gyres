# Agent Infrastructure Landscape (April 2026)

> Research conducted for gyres-4zz. Grounding material for the harness/OS seams analysis.

## The Emerging Stack

The community has converged on a rough layering of agent infrastructure, though no single vendor occupies just one layer cleanly:

| Layer | What it provides | Examples |
|---|---|---|
| **Protocol** | Wire-level interop between agents, tools, and services | MCP (Anthropic → Linux Foundation), A2A (Google → Linux Foundation) |
| **Library/SDK** | Primitives: model calls, streaming, tool calling | rig (Rust), Vercel AI SDK, Claude Agent SDK, OpenAI Agents SDK |
| **Framework** | Abstractions for assembling agent logic | LangChain/LangGraph, CrewAI, Microsoft Agent Framework 1.0 |
| **Runtime** | Durable execution, pause/resume, fault tolerance | LangGraph (persistent), Temporal-based systems |
| **Harness** | Opinionated defaults, pre-built tools, governance, session management | Claude Code, OpenHarness |
| **Platform** | Deploy, monitor, scale, dashboards | Azure AI Foundry, Vertex AI Agent Engine, LangSmith |
| **OS** | Process model, scheduling, isolation, IPC, resource governance | AIOS (academic), Rivet agentOS |

This is not a strict linear progression. Products span multiple layers: Anthropic's Claude Agent SDK includes orchestration; Microsoft's "framework" includes platform features; Google's "platform" defines protocols. The boundaries blur in practice.

## What the Community Calls a "Harness"

### Origin

The term entered mainstream agent discourse via Anthropic's engineering blog post ["Effective harnesses for long-running agents"](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents). In software engineering, a "harness" wraps something and runs it (test harness, benchmark harness). Applied to agents, it means: the scaffolding that wraps a model to make it a functional agent.

### Community definition

[Parallel.ai](https://parallel.ai/articles/what-is-an-agent-harness): "The complete infrastructure that wraps around an LLM to make it a functional agent. The model provides intelligence; the harness provides hands, eyes, memory, and safety boundaries."

[Analytics Vidhya](https://www.analyticsvidhya.com/blog/2025/12/agent-frameworks-vs-runtimes-vs-harnesses/) taxonomy: "Frameworks define *what* an agent is, runtimes handle *durable execution*, and harnesses wrap both with opinionated defaults, testing, and governance."

### Six responsibilities (consistent across sources)

1. Tool integration layer (dispatch, sandboxing, result handling)
2. Memory and state management across sessions
3. Context engineering and curation (what the model sees)
4. Planning and decomposition (task breakdown)
5. Verification and guardrails (safety, permissions)
6. Modularity and extensibility (plugins, skills, tool packs)

### Key distinction from framework

A framework gives building blocks. A harness gives a working system with defaults. The harness is **opinionated and pre-configured** — you configure it, not assemble it.

### Projects using harness framing

- **Claude Code** (Anthropic) — the reference implementation. Multi-step planning, persistent context, sandboxing, three-tier permissions, modular Skills system.
- **OpenHarness** ([HKUDS/OpenHarness](https://github.com/HKUDS/OpenHarness)) — open-source agent harness with agent loop, tool integration, memory, governance, multi-agent delegation.
- **Con** — GPU-accelerated terminal with built-in agent harness using rig underneath.

Notable: LangChain, CrewAI, AutoGen, OpenAI Agents SDK, Mastra, Vercel AI SDK all call themselves "frameworks" or "SDKs," not harnesses. The harness label is newer and more specific.

### Community sentiment

The dominant narrative in 2026: **"The model is commodity. The harness determines success."** ([Medium: "2025 Was Agents. 2026 Is Agent Harnesses."](https://aakashgupta.medium.com/2025-was-agents-2026-is-agent-harnesses-heres-why-that-changes-everything-073e9877655e))

Strong preference for minimal harnesses over complex scaffolding. Recurring observation: "Simpler harnesses often outperform complex scaffolding. The model is smart enough. The harness just prevents catastrophic failures."

## What the Community Calls an "Agent OS"

### Real implementations

**AIOS (Rutgers University, COLM 2025)** — The most academically rigorous effort. Embeds LLM into an OS kernel abstraction layer providing: agent scheduling (FIFO, Round Robin), context snapshot/restore for LLM state switching, memory management, storage management, tool dispatch via syscalls, and access control. Claims 2.1x faster execution vs. naive serving.
- [Paper](https://arxiv.org/abs/2403.16971) | [GitHub](https://github.com/agiresearch/AIOS)

**Rivet agentOS** — Production-oriented. WebAssembly and V8 isolates with ~6ms cold starts (516x faster than containers). Per-agent CPU/memory limits, network namespace isolation, unified filesystem abstraction (mount S3/SQLite/GDrive as directories), credential scoping, automatic transcript persistence, session resumption, durable workflows. Their pitch: "agentOS abstracts operational requirements agents need, independent of framework or LLM."
- [Site](https://rivet.dev/agent-os/) | [GitHub](https://github.com/rivet-dev/agent-os)

**Microsoft (Windows as Agentic Platform)** — Reframes Windows itself as an agentic layer. Agent 365 is a control plane for discovering, managing, and securing agents across an organization. Microsoft Agent Framework 1.0 (April 2026) merges Semantic Kernel + AutoGen. They call the OS "an orchestration layer, not just an interface."
- [Agent Framework](https://devblogs.microsoft.com/agent-framework/microsoft-agent-framework-version-1-0/)

**Claude Code (community framing)** — Anthropic doesn't brand it as an OS, but [MindStudio's analysis](https://www.mindstudio.ai/blog/agentic-os-architecture-claude-code-skills-workflows) argues it constitutes a five-layer "agentic OS": brand context, memory, skills, orchestration, self-maintenance. Called it an "accidental OS."

### Four features that distinguish OS from framework

The community converges on these as the OS-level concerns:

1. **Process model and scheduling** — agents as managed processes with lifecycle, preemption, resource allocation
2. **Isolation and security** — sandboxing, per-agent resource limits, network namespaces, credential scoping
3. **Persistent state and memory** — at the infrastructure level, not left to application code
4. **Inter-agent communication primitives** — IPC for coordination, analogous to Unix pipes or message queues

As one analysis put it: "LangChain is not a runtime in the OS sense; it's more like a low-level framework for assembling one." ([UX Magazine](https://uxmag.com/articles/the-rise-of-agent-runtime-platforms-whos-building-the-os-for-agents))

### The central argument for Agent OS

[Nagendra Gupta (Medium)](https://medium.com/emergent-intelligence/multi-agent-os-why-ai-will-need-its-own-operating-system-0f706993ba97): "Agents fail because they don't know how to run, not because they can't think." Intelligence is no longer the bottleneck — execution is. Multi-step, autonomous agents are distributed systems requiring formal coordination.

[Marc Bara (Medium)](https://medium.com/@marc.bara.iniesta/who-is-building-the-agent-native-operating-system-c6bae5a5a3f5): LLM state (context windows, KV caches) is too costly and complex for each agent to manage independently — it needs kernel-level solutions.

### Skepticism

- "Agent OS" is currently a **positioning claim, not a recognized product category**. Marc Bara's own assessment: "None of them solved all four" essential requirements.
- Gartner predicts 40%+ of agentic AI projects will be canceled by end of 2027 due to costs, unclear value, or inadequate risk controls.
- Gary Marcus: "AI Agents have, so far, mostly been a dud."
- Best agent tested completed only ~24% of tasks autonomously (IBM).
- The "agent washing" problem — vendors rebrand chatbots and RPA as "agentic" without substance.

## How Major Players Frame Their Infra

| Player | Self-description | Actual scope |
|---|---|---|
| **Anthropic** | "Runtime/SDK" + "Protocol" (MCP) | SDK, harness (Claude Code), protocol |
| **OpenAI** | "Lightweight framework" (Agents SDK) | SDK, framework, added harness + sandboxing April 2026 |
| **Google** | "Protocol" (A2A) + "Platform" (Vertex AI) | Protocol, platform |
| **Microsoft** | "Enterprise framework" (Agent Framework 1.0) | Framework spanning into platform territory |
| **LangChain** | "Agent engineering platform" | Framework + platform (LangSmith) |
| **CrewAI** | "Multi-agent framework" | Framework |
| **rig** | "Rust library for LLM applications" | Library |

Notable: None of the major vendors call their shipping product an "Agent OS." The term lives in academia (AIOS), startups (Rivet), and aspirational framing.

## Protocol Layer: MCP and A2A

Two protocols are emerging as the interop standard:

- **MCP (Model Context Protocol)** — Anthropic, donated to Linux Foundation December 2025. Agent-to-tool communication. November 2025 spec added async capabilities, auth, and long-running task support.
- **A2A (Agent-to-Agent)** — Google, donated to Linux Foundation. Agent-to-agent communication. Launched April 2025 with 50+ partners, v1.0 by early 2026 with 150+ organizations.

The analogy to OS concepts: MCP is like device drivers / filesystem syscalls (agent talks to tools/data). A2A is like IPC / networking (agents talk to each other). Together they form the interface layer, but they are application-level protocols (JSON-RPC, gRPC), not kernel interfaces.

## The "Framework Squeeze"

[Tony Kipkemboi](https://www.tonykipkemboi.com/blog/agent-frameworks-getting-squeezed): AI labs are building down into orchestration (Claude Agent SDK, OpenAI Agents SDK) while automation platforms are building up into reasoning — compressing the framework layer from both sides. Frameworks may primarily serve consultancies and system integrators going forward.

**Agentic AI Foundation (AAIF)** — Founded December 2025 by Anthropic, OpenAI, and Block, later joined by Google, Microsoft, and AWS. A neutral consortium for open standards. The closest thing to agent infrastructure governance.

## Rust Agent Ecosystem

The Rust agent ecosystem saw 16x growth in GitHub stars from 2024 to 2026 ([OSS Insight](https://ossinsight.io/blog/rust-ai-agent-infrastructure-2026)):

| Project | Positioning | Maturity |
|---|---|---|
| [rig](https://rig.rs/) | Library: unified model interface, 20+ providers, 10+ vector stores, WASM-compatible | Most mature |
| [swarm-rs](https://github.com/fcn06/swarm) | Multi-agent orchestration SDK with MCP/A2A | Early |
| [AutoAgents](https://github.com/liquidos-ai/AutoAgents) | Multi-agent framework, type-safe, structured tool calling | Early |
| [rs-agent](https://lib.rs/crates/rs-agent) | Production orchestrator with tool calling, memory, multi-agent | Early |
| [ai-agents](https://crates.io/crates/ai-agents) | YAML-spec-driven agent definitions | Early |

**No Rust project occupies the harness or OS layer.** rig is a library. Everything else is early-stage framework work. This is the gap Gyres targets.

Community trend: infrastructure teams converging on Rust for agent runtimes, CLI tools, sandboxes, and security layers because memory safety matters when agents have filesystem/shell access.
