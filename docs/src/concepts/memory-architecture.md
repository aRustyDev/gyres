# Memory Architecture

## MemoryStore on GyreContext

### What it enables

Every Gyre can:
- Store knowledge that persists across steps within a run
- Store knowledge that persists across runs (cross-session)
- Retrieve relevant context before each agent step
- Build a knowledge graph over time

### MemoryStore trait

```rust
pub trait MemoryStore: Send + Sync {
    /// Store a memory entry.
    fn store(&self, entry: &MemoryEntry)
        -> Pin<Box<dyn Future<Output = Result<MemoryId, GyreError>> + Send + '_>>;

    /// Recall memories matching a query.
    fn recall(&self, query: &MemoryQuery)
        -> Pin<Box<dyn Future<Output = Result<Vec<MemoryEntry>, GyreError>> + Send + '_>>;

    /// Remove a memory entry.
    fn forget(&self, id: &MemoryId)
        -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    /// Update an existing memory entry.
    fn update(&self, id: &MemoryId, entry: &MemoryEntry)
        -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;
}
```

### GraphMemoryStore extension

```rust
/// Extension trait for graph-capable memory backends.
/// Not required — agents that only need key-value memory use MemoryStore.
pub trait GraphMemoryStore: MemoryStore {
    /// Create a relationship between two memory entries.
    fn relate(&self, from: &MemoryId, edge: &str, to: &MemoryId)
        -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    /// Traverse relationships from a starting node.
    fn traverse(&self, from: &MemoryId, edge: &str, depth: usize)
        -> Pin<Box<dyn Future<Output = Result<Vec<MemoryEntry>, GyreError>> + Send + '_>>;

    /// Execute a subgraph query.
    fn query_subgraph(&self, query: &GraphQuery)
        -> Pin<Box<dyn Future<Output = Result<Vec<MemoryEntry>, GyreError>> + Send + '_>>;
}
```

### Core types

```rust
pub type MemoryId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: MemoryId,
    pub content: String,
    pub kind: MemoryKind,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    /// Agent that created this memory.
    pub source: AgentId,
    /// Embedding vector for semantic search (optional).
    pub embedding: Option<Vec<f32>>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryKind {
    /// Factual knowledge about the project/domain.
    Fact,
    /// User preference or behavioral observation.
    Preference,
    /// A decision that was made and why.
    Decision,
    /// A procedure or workflow.
    Procedure,
    /// A relationship between entities.
    Relationship,
    /// Ephemeral working memory (cleared on reset).
    Working,
    /// Custom type.
    Custom(String),
}

pub struct MemoryQuery {
    /// Text query for semantic/keyword search.
    pub text: Option<String>,
    /// Filter by memory kind.
    pub kinds: Option<Vec<MemoryKind>>,
    /// Filter by source agent.
    pub source: Option<AgentId>,
    /// Maximum results.
    pub limit: usize,
    /// Minimum relevance score (0.0-1.0) for semantic search.
    pub min_relevance: Option<f32>,
}

pub struct GraphQuery {
    /// Starting node(s).
    pub roots: Vec<MemoryId>,
    /// Edge types to follow.
    pub edge_types: Option<Vec<String>>,
    /// Maximum traversal depth.
    pub max_depth: usize,
}
```

## GyreContext impact

### Before (current)

```rust
pub struct GyreContext {
    pub agent_id: AgentId,
    pub session_id: Option<SessionId>,
    pub parent_span: Option<SpanId>,
    pub permissions: Arc<dyn PermissionGate>,
    pub state: Arc<dyn StateStore>,
    pub config: Arc<Config>,
    pub telemetry: Arc<dyn TelemetrySink>,
}
```

### After (with stores)

```rust
pub struct GyreContext {
    // Identity
    pub agent_id: AgentId,
    pub session_id: Option<SessionId>,
    pub parent_span: Option<SpanId>,

    // Infrastructure
    pub permissions: Arc<dyn PermissionGate>,
    pub config: Arc<Config>,
    pub telemetry: Arc<dyn TelemetrySink>,

    // Stores
    pub state: Arc<dyn StateStore>,
    pub memory: Arc<dyn MemoryStore>,
    pub tasks: Arc<dyn TaskStore>,
    pub artifacts: Arc<dyn ArtifactStore>,
}
```

### Pros

- Memory is always available — every Gyre can store and recall without extra setup
- Consistent API for all domains (LLM agents and RL agents use the same MemoryStore)
- Shared backend means memory, state, tasks, and artifacts can share a database
- Multi-agent coordination through shared MemoryStore (blackboard pattern)
- GraphMemoryStore enables knowledge graph construction as an implicit side-effect

### Cons

- GyreContext grows (now 10 fields). More to construct, more to mock in tests.
  - Mitigation: builder pattern with sensible defaults (InMemoryBackend for all stores)
  - Mitigation: `GyreContext::minimal(agent_id)` constructor that uses no-op/in-memory for everything
- Simple agents that don't need memory still carry an Arc<dyn MemoryStore>.
  - Mitigation: the InMemory backend is ~zero cost if unused
  - Alternative: make memory Optional? But then every access needs .as_ref().unwrap()
- Backend crate becomes load-bearing — every project depends on at least one backend.
  - Mitigation: InMemoryBackend ships with gyres-core, no external deps

### Decision

MemoryStore goes on GyreContext as a required field. InMemoryBackend is the zero-cost default. Graph capabilities are an extension trait, not required.

## Memory in the agent loop

How a Gyre uses memory during a conversation:

```
1. Load session → get conversation history from StateStore
2. Recall relevant memories → MemoryStore.recall(user_query)
3. Inject memories into system prompt or context
4. Agent.step(observation)
5. If agent produced useful knowledge → MemoryStore.store(...)
6. If agent made a decision → ArtifactStore.emit(decision)
7. If agent discovered a relationship → GraphMemoryStore.relate(...)
8. Repeat
```

Steps 5-7 are the "implicit side-effects" — the Gyre automatically captures knowledge, decisions, and relationships as the agent works.

## Embedding generation

MemoryEntry has an optional `embedding: Vec<f32>`. Who generates it?

Options:
1. **The MemoryStore backend** — backend calls an embedding model when entries are stored.
   Pro: transparent. Con: backend needs LLM access.
2. **The Gyre** — generates embedding before calling store().
   Pro: backend stays simple. Con: Gyre needs embedding model access.
3. **A dedicated embedding service** — MemoryStore delegates to an embedding provider.
   Pro: decoupled. Con: another service to configure.

Recommendation: Option 2 for now. The Gyre (or a helper) generates embeddings before storing. The MemoryStore just persists what it's given. This keeps backends simple and avoids coupling them to LLM providers.
