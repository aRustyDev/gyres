# Agent Loop Data Flow

> Where Observation, Thought, Action, and Feedback land in the store architecture.

## The Loop

An agent's core cycle is Observe, Think, Act, Feedback (O/T/A/F):

```
Observe → Think → Act → Feedback → Observe → ...
```

Each element is captured automatically as part of the session (StateStore) and as telemetry spans (TelemetrySink). This is the **raw loop data** — ephemeral, session-scoped, operational.

## Raw Loop Data vs Extracted Knowledge

The raw O/T/A/F stream and the knowledge extracted from it are different things stored in different places:

| Loop element | Raw event destination | Extracted knowledge destination |
|---|---|---|
| **Observation** | StateStore (session turn) | "This codebase uses X pattern" → MemoryStore |
| **Thought** | StateStore (turn) + TelemetrySink (reasoning span) | "I concluded X because Y" → MemoryStore |
| **Action** | TelemetrySink (tool call span) | Created task → TaskStore; wrote ADR → DocumentStore |
| **Feedback** | StateStore (session turn) | "Approach X failed because Y" → MemoryStore |

The raw events flow automatically — StateStore captures turns, TelemetrySink captures spans. No special logic needed.

The knowledge extraction is the Gyre's responsibility. The Gyre observes the agent's outputs and decides what to promote:

```
Agent produces step output
  → Gyre evaluates: is there durable knowledge here?
    → Learned something reusable?   → MemoryStore.store()
    → Created a structured artifact? → DocumentStore / TaskStore
    → Just operational trace?        → TelemetrySink (already captured)
    → Ephemeral conversation?        → StateStore (already captured)
```

## The Gyre as Extractor

This is the "implicit side-effects" mechanism made concrete. The agent doesn't need to explicitly say "save this as a memory" or "create an ADR." The Gyre recognizes valuable byproducts and routes them:

```
Agent step → response contains architectural reasoning
  → Gyre detects decision pattern
  → Gyre emits Document (ADR) to DocumentStore
  → Gyre stores rationale as MemoryEntry in MemoryStore
  → Both are searchable via ArtifactStore for future RAG retrieval
```

Detection strategies (from most to least autonomous):
- **Pattern-based:** regex/keyword matching on agent output
- **Agent-based:** a dedicated observer agent that evaluates outputs
- **Explicit:** the agent signals via structured output ("this is a decision")

## Data Flow Diagram

```
                        ┌─────────────────────────────────────────┐
                        │              Agent Loop                  │
                        │   Observe → Think → Act → Feedback      │
                        └──────────────────┬──────────────────────┘
                                           │
                              Gyre extracts / routes
                                           │
                    ┌──────────┬───────────┼───────────┬──────────┐
                    ▼          ▼           ▼           ▼          ▼
              StateStore  TelemetrySink  MemoryStore  TaskStore  DocumentStore
              (sessions)  (spans/logs)  (knowledge)  (work DAG) (structured docs)
                MutStore     Sink        MutStore    MutStore    MutStore
                    │          │           │           │          │
                    │          │           ▼           ▼          ▼
                    │          │    ┌──────────────────────────────┐
                    │          │    │       ArtifactStore          │
                    │          │    │   (cross-type read view)     │
                    │          │    │        RefStore              │
                    │          │    └──────────────────────────────┘
                    │          │
                    ▼          ▼
               Not artifacts — not searchable via ArtifactStore
```

StateStore and TelemetrySink are outside the artifact system. Sessions are operational context, not work products. Telemetry is instrumentation. Only the three right-hand stores (Memory, Task, Document) hold Artifacts, and ArtifactStore queries across them.

## Thoughts and Reasoning

Agent reasoning (chain-of-thought, extended thinking, scratchpads) is captured in two places by default:

1. **StateStore** — as part of the conversation turn, so the agent has reasoning context for future steps within the session
2. **TelemetrySink** — as a reasoning span, so operators can debug/audit why the agent made a decision

Like all loop elements, the Gyre can promote specific reasoning to MemoryStore if it contains durable insight. The reasoning itself is ephemeral; the knowledge it produces is what persists.

## Design Principle

The agent loop generates a firehose of data. Most of it is ephemeral. The architecture separates concerns by durability and audience:

- **Ephemeral, machine-only:** StateStore (session context for the running agent)
- **Ephemeral, operator-facing:** TelemetrySink (debugging, audit, cost tracking)
- **Durable, agent-facing:** MemoryStore (knowledge the agent recalls later)
- **Durable, human+agent-facing:** TaskStore, DocumentStore (structured work products)
- **Durable, cross-type search:** ArtifactStore (unified retrieval across all durable stores)
