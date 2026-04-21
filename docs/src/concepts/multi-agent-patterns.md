# Multi-Agent Orchestration Patterns

## The Agent + Gyre + Task Triangle

Three primitives interact to enable multi-agent orchestration:

```
        Task
       ╱    ╲
      ╱      ╲
   Agent ─── Gyre
```

- **Agent** — what does the work (step, feedback, reset)
- **Gyre** — how the work loop runs (strategy, termination, tool dispatch)
- **Task** — what work needs to be done (decomposable, dependency-tracked)

Single-agent: one Agent, one Gyre, one Task.
Multi-agent: multiple Agents, potentially multiple Gyres, a task graph.

## How they compose

### Level 1: Single agent, single task (MVP)

```
ConversationGyre.run(llm_agent, ctx, strategy)
```

The Gyre drives one Agent to complete one implicit task.

### Level 2: Single agent, task decomposition

```
Task: "Add authentication to the API"
├── Subtask: "Design auth schema"
├── Subtask: "Implement middleware"  (blocked by schema)
└── Subtask: "Write tests"          (blocked by middleware)
```

One Agent works through the task graph sequentially. The Gyre:
1. Queries `ctx.tasks.ready_tasks()` to find unblocked work
2. Presents the next task to the Agent as an observation
3. Agent completes the task
4. Gyre marks it complete, checks for new ready tasks
5. Loop until task graph is empty

This is what beads does for humans. The Gyre automates it.

### Level 3: Multiple agents, single task graph (Supervisor)

```
SupervisorGyre
├── Agent A (coder) ──── works on "Implement middleware"
├── Agent B (reviewer) ── reviews Agent A's output
└── Agent C (tester) ──── writes tests once middleware is done
```

The SupervisorGyre:
1. Queries `ctx.tasks.ready_tasks()`
2. Assigns tasks to agents based on role/capability
3. Runs each agent in its own sub-Gyre (ConversationGyre, ReviewGyre, etc.)
4. Collects outcomes, marks tasks complete
5. Routes feedback between agents (reviewer → coder)

### Level 4: Dynamic agent coordination (Swarm)

No central supervisor. Agents discover work and coordinate peer-to-peer:

```
SwarmGyre
├── Agent A ── sees "schema" is ready ── claims it
├── Agent B ── sees nothing ready ── waits
├── Agent A ── completes "schema" ── "middleware" becomes ready
├── Agent B ── sees "middleware" is ready ── claims it
└── ...
```

The SwarmGyre:
1. Each agent independently queries `ctx.tasks.ready_tasks()`
2. Agents claim tasks atomically (CAS on task status)
3. Agents run independently, using shared MemoryStore for coordination
4. ArtifactStore captures decisions/docs as implicit side-effects

### Level 5: Hierarchical task decomposition (HTD)

An agent *creates* the task graph as it goes:

```
Task: "Build a web app"
  ↓ Agent decomposes
├── "Set up project structure"
├── "Implement backend API"
│   ↓ Agent decomposes further
│   ├── "Define routes"
│   ├── "Implement handlers"
│   └── "Add database layer"
└── "Build frontend"
```

The HTD Gyre:
1. Presents top-level task to a "planner" agent
2. Planner decomposes into subtasks, writes them to TaskStore
3. "Worker" agents pick up leaf tasks
4. When a subtask is too complex, the worker can decompose further
5. Recursive until all tasks are atomic enough to complete

## What gyres-core needs to support this

### Agent Registry

Multiple agents running concurrently need discovery:

```rust
pub trait AgentRegistry: Send + Sync {
    /// Register an agent with its capabilities.
    fn register(&self, id: &AgentId, capabilities: &AgentCapabilities)
        -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    /// Find agents matching a capability query.
    fn find(&self, query: &CapabilityQuery)
        -> Pin<Box<dyn Future<Output = Result<Vec<AgentId>, GyreError>> + Send + '_>>;

    /// Get the current status of an agent.
    fn status(&self, id: &AgentId)
        -> Pin<Box<dyn Future<Output = Result<AgentStatus, GyreError>> + Send + '_>>;
}
```

### Agent Messaging

Agents need to communicate (supervisor → worker, reviewer → coder):

```rust
pub trait MessageBus: Send + Sync {
    /// Send a message to a specific agent.
    fn send(&self, to: &AgentId, message: AgentMessage)
        -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    /// Receive pending messages for an agent.
    fn receive(&self, agent: &AgentId)
        -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, GyreError>> + Send + '_>>;

    /// Broadcast to all agents.
    fn broadcast(&self, message: AgentMessage)
        -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;
}
```

### GyreContext additions for multi-agent

```rust
pub struct GyreContext {
    // ...existing...
    pub agents: Option<Arc<dyn AgentRegistry>>,
    pub messages: Option<Arc<dyn MessageBus>>,
}
```

Optional — single-agent Gyres don't need them.

## Task ↔ Agent Assignment

The question: who decides which agent works on which task?

### Option A: Gyre decides (centralized)

The orchestrating Gyre assigns tasks to agents. This is the Supervisor pattern:

```rust
// Inside SupervisorGyre::run()
let ready = ctx.tasks.ready_tasks().await?;
for task in ready {
    let agent_id = self.assign(&task, ctx).await?;  // Gyre's logic
    let agent = ctx.agents.get(&agent_id)?;
    let sub_gyre = ConversationGyre;
    sub_gyre.run(agent, &ctx.with_agent(agent_id), &strategy).await?;
    ctx.tasks.update_status(&task.id, TaskStatus::Complete).await?;
}
```

### Option B: Agent self-selects (decentralized)

The agent picks its own work from the task graph. This is the Swarm pattern:

```rust
// Inside SwarmGyre::run() — each agent runs this independently
loop {
    let ready = ctx.tasks.ready_tasks().await?;
    let task = self.select(&ready, agent)?;  // agent's preference
    if ctx.tasks.try_claim(&task.id, &ctx.agent_id).await? {
        let result = agent.step(&task.as_observation()).await?;
        ctx.tasks.update_status(&task.id, TaskStatus::Complete).await?;
    }
}
```

### Option C: Hybrid (recommended)

A Coordinator Gyre decomposes and prioritizes. Agents claim from the ready queue. The coordinator intervenes on conflicts or bottlenecks.

## Relationship to existing design

| Concept | Where it lives | New? |
|---|---|---|
| Agent trait | gyres-core | No — unchanged |
| Gyre trait | gyres-core | No — orchestration Gyres are just Gyre impls |
| Task + TaskStore | gyres-core | Yes — new trait |
| AgentRegistry | gyres-core or gyres-multi | Yes — new trait |
| MessageBus | gyres-core or gyres-multi | Yes — new trait |
| SupervisorGyre | gyres-multi | Yes — Gyre implementation |
| SwarmGyre | gyres-multi | Yes — Gyre implementation |
| HtdGyre | gyres-multi | Yes — Gyre implementation |

The key insight: **multi-agent orchestration is a Gyre implementation concern, not a core trait change.** The core Agent and Gyre traits work unchanged. What's new is:
1. TaskStore (task graph)
2. AgentRegistry (discovery)
3. MessageBus (communication)
4. Orchestration Gyre implementations

## Open questions

1. Should AgentRegistry and MessageBus be on GyreContext, or passed separately to multi-agent Gyres?
2. How does task decomposition interact with permissions? (Can Agent A create tasks for Agent B?)
3. Should there be a formal "delegation" operation (Agent A delegates to Agent B with specific permissions)?
4. How does the Gyre know when the entire task graph is complete (not just one task)?
