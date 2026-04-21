# Store Abstraction: Separate Traits, Shared Backends

## The Distinction

**Store** = the API a consumer uses. Defines *what* operations are available. Domain-specific.

**Backend** = the implementation that fulfills one or more Store traits. Defines *how* data is persisted. Infrastructure-specific.

```
Consumer layer          Store traits (domain-specific APIs)
                        ┌──────────┐ ┌───────────┐ ┌─────────┐ ┌──────────────┐
                        │StateStore│ │MemoryStore│ │TaskStore│ │ArtifactStore │
                        └────┬─────┘ └─────┬─────┘ └────┬────┘ └──────┬───────┘
                             │             │             │             │
Adapter layer           Store ←→ Backend binding (1:1 at runtime)
                             │             │             │             │
                        ┌────┴─────────────┴─────────────┴─────────────┴───────┐
Infrastructure layer    │                    Backend                            │
                        │  (SqliteBackend, SurrealBackend, InMemoryBackend)     │
                        └──────────────────────────────────────────────────────┘
```

A Store is bound to exactly one Backend at runtime. The binding is configurable but singular — you don't split MemoryStore across two databases.

A Backend can satisfy multiple Store traits simultaneously. One `SqliteBackend` instance can be the backing store for StateStore, MemoryStore, TaskStore, and ArtifactStore — different tables/collections in the same database.

## Store Traits

### StateStore
- **Shape:** Key-value (SessionId → SessionState)
- **Read:** load_session, list_sessions
- **Write:** save_session, append_turn
- **Query:** filter by worktree

### MemoryStore
- **Shape:** Key-value + vector embeddings + relationships (graph-capable)
- **Read:** recall by query (semantic search, keyword, relationship traversal)
- **Write:** store entries, relate entries, forget
- **Query:** semantic search, graph traversal, keyword match

### TaskStore
- **Shape:** Graph (tasks with many-to-many blocking relationships)
- **Read:** get by TaskId, list by status, find ready (unblocked) tasks
- **Write:** create, update status, add/remove dependencies
- **Query:** blocked-by traversal, dependency resolution, critical path

### ArtifactStore
- **Shape:** Append-heavy, read via search/RAG
- **Read:** search by type/content (RAG retrieval), list by type
- **Write:** emit artifacts (decisions, documentation, memory entries)
- **Query:** full-text search, type filtering, temporal ordering
- **Note:** ArtifactStore replaces the earlier ArtifactSink concept. Making it a Store (not just a Sink) enables RAG retrieval — artifacts the agent produces can become context the agent consumes.

## Backend Trait

```rust
/// Marker trait for storage backends. A backend can implement
/// multiple Store traits. Feature-gated at compile time.
pub trait Backend: Send + Sync + 'static {
    /// Human-readable name for diagnostics.
    fn name(&self) -> &str;

    /// Check connectivity / readiness.
    fn health_check(&self) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;
}
```

## Backend Implementations

| Backend | Feature flag | StateStore | MemoryStore | TaskStore | ArtifactStore |
|---|---|---|---|---|---|
| InMemoryBackend | `backend-memory` | ✓ | ✓ (no semantic search) | ✓ | ✓ |
| JsonFileBackend | `backend-json` | ✓ | ✗ | ✗ | ✓ (append) |
| SqliteBackend | `backend-sqlite` | ✓ | ✓ (with FTS5) | ✓ (recursive CTE) | ✓ |
| SurrealBackend | `backend-surreal` | ✓ | ✓ (native graph + vector) | ✓ (native graph) | ✓ |
| `backend-all` | enables all | — | — | — | — |

## Store ↔ Backend Binding (1:1 Singleton)

Each Store is backed by exactly one Backend at runtime. Enforced via the type system:

```rust
/// Binds a Store trait to a specific Backend instance.
/// Created by the storage factory, not by the user.
pub struct BoundStore<S: ?Sized> {
    inner: Arc<dyn S>,
}
```

The storage factory creates all stores from a single config, ensuring consistent backend selection:

```rust
pub struct StorageConfig {
    pub backend: BackendConfig,
}

pub enum BackendConfig {
    InMemory,
    JsonFiles { base_dir: PathBuf },
    Sqlite { path: PathBuf },
    Surreal { url: String },
}

pub struct Stores {
    pub state: Arc<dyn StateStore>,
    pub memory: Arc<dyn MemoryStore>,
    pub tasks: Arc<dyn TaskStore>,
    pub artifacts: Arc<dyn ArtifactStore>,
}

impl Stores {
    pub fn from_config(config: &StorageConfig) -> Result<Self, GyreError> {
        match &config.backend {
            BackendConfig::Sqlite { path } => {
                let backend = Arc::new(SqliteBackend::open(path)?);
                Ok(Stores {
                    state: backend.clone(),
                    memory: backend.clone(),
                    tasks: backend.clone(),
                    artifacts: backend,
                })
            }
            // ...
        }
    }
}
```

## GyreContext Integration

GyreContext carries individual store trait objects (not the Stores bundle):

```rust
pub struct GyreContext {
    pub agent_id: AgentId,
    pub session_id: Option<SessionId>,
    pub parent_span: Option<SpanId>,
    pub permissions: Arc<dyn PermissionGate>,
    pub state: Arc<dyn StateStore>,
    pub memory: Arc<dyn MemoryStore>,
    pub tasks: Arc<dyn TaskStore>,
    pub artifacts: Arc<dyn ArtifactStore>,
    pub config: Arc<Config>,
    pub telemetry: Arc<dyn TelemetrySink>,
}
```

The executor creates Stores from config and distributes the individual Arc<dyn ...> into GyreContext. This keeps GyreContext's API clean (each field is one trait) while the backend sharing is an implementation detail.

## Crate Organization

```
gyres-core/
├── src/state.rs         # StateStore trait
├── src/memory.rs        # MemoryStore trait + GraphMemoryStore extension
├── src/task.rs          # TaskStore trait
├── src/artifact.rs      # ArtifactStore trait
└── src/backend.rs       # Backend trait, BackendConfig, Stores factory

gyres-store/             # All backend implementations, feature-gated
├── src/memory.rs        # InMemoryBackend (default, always available)
├── src/sqlite.rs        # SqliteBackend (feature = "backend-sqlite")
└── src/surreal.rs       # SurrealBackend (feature = "backend-surreal")
```

Store traits live in gyres-core (consumers depend on them). Backend implementations live in `gyres-store` — a single crate with feature-gated backends (ADR 0016). Users add `gyres-store = { features = ["backend-sqlite"] }`. The `InMemoryBackend` is always available (no feature flag) for testing and as the `GyreContext::minimal()` default.

## TaskStore: Graph Shape

Tasks form a DAG (directed acyclic graph) via blocking relationships:

```
Task A ──blocks──→ Task C
Task B ──blocks──→ Task C
Task C ──blocks──→ Task D
```

TaskStore operations reflect this:

```rust
pub trait TaskStore: Send + Sync {
    fn create_task(&self, task: &TaskDef)
        -> Pin<Box<dyn Future<Output = Result<TaskId, GyreError>> + Send + '_>>;

    fn get_task(&self, id: &TaskId)
        -> Pin<Box<dyn Future<Output = Result<Option<Task>, GyreError>> + Send + '_>>;

    fn update_status(&self, id: &TaskId, status: TaskStatus)
        -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    fn add_dependency(&self, blocked: &TaskId, blocked_by: &TaskId)
        -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    fn remove_dependency(&self, blocked: &TaskId, blocked_by: &TaskId)
        -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    /// Find tasks that have no unresolved blockers.
    fn ready_tasks(&self)
        -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;

    fn list_tasks(&self, filter: &TaskFilter)
        -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;

    /// Get all tasks that block a given task (transitive).
    fn blockers(&self, id: &TaskId, depth: Option<usize>)
        -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;

    /// Get all tasks blocked by a given task (transitive).
    fn dependents(&self, id: &TaskId, depth: Option<usize>)
        -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;
}
```

## ArtifactStore: Write-Heavy + RAG Retrieval

```rust
pub trait ArtifactStore: Send + Sync {
    fn emit(&self, artifact: &Artifact)
        -> Pin<Box<dyn Future<Output = Result<ArtifactId, GyreError>> + Send + '_>>;

    /// Search artifacts for RAG context retrieval.
    fn search(&self, query: &str, filter: &ArtifactFilter)
        -> Pin<Box<dyn Future<Output = Result<Vec<Artifact>, GyreError>> + Send + '_>>;

    fn get(&self, id: &ArtifactId)
        -> Pin<Box<dyn Future<Output = Result<Option<Artifact>, GyreError>> + Send + '_>>;

    fn list(&self, filter: &ArtifactFilter)
        -> Pin<Box<dyn Future<Output = Result<Vec<ArtifactMeta>, GyreError>> + Send + '_>>;
}
```
