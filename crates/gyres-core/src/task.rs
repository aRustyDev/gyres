use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::GyreError;
use crate::types::AgentId;

/// Unique identifier for a task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

/// The hierarchy level of a task in the PM hierarchy.
///
/// All PM levels (Theme through Subtask) are modeled as `Task` with
/// a `kind` discriminator. This avoids duplicating DAG operations
/// across separate types.
///
/// `Custom(String)` usage is telemetry-logged to signal when new kinds
/// should be promoted to first-class variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Organization-wide strategic direction. Annual/multi-quarter scope.
    Theme,
    /// Cross-team program advancing a theme. Quarterly scope.
    Initiative,
    /// Shippable capability for a single team. 2-8 weeks scope.
    Epic,
    /// User-facing increment of value. Fits in one sprint.
    Story,
    /// Concrete technical work item. Hours to 1-2 days.
    Task,
    /// Atomic action. Minutes to hours. Leaf node.
    Subtask,
    /// Extension point for domain-specific or external platform levels.
    /// Telemetry-logged when encountered.
    Custom(String),
}

/// Task lifecycle status.
///
/// A single enum covering all hierarchy levels. Valid statuses and
/// transitions are validated at runtime per [`TaskKind`] by the
/// [`TaskStore`] implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    // --- Planning states (themes, initiatives) ---
    /// Proposed but not yet active.
    Proposed,
    /// Actively being pursued.
    Active,
    /// No longer relevant. Terminal for themes.
    Retired,

    // --- Execution states (epics, stories, tasks) ---
    /// Waiting for blockers to resolve.
    Blocked,
    /// All blockers resolved, ready to be claimed.
    Ready,
    /// Claimed by an agent, work in progress.
    InProgress,
    /// Work complete, under review.
    InReview,
    /// Work completed successfully. Terminal.
    Complete,
    /// Work failed or was abandoned. Terminal.
    Failed,
    /// Intentionally cancelled. Terminal.
    Cancelled,
}

impl TaskStatus {
    /// Whether this status represents a terminal (final) state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Retired
                | TaskStatus::Complete
                | TaskStatus::Failed
                | TaskStatus::Cancelled
        )
    }
}

/// Returns the valid statuses for a given task kind.
///
/// Used by [`TaskStore`] to validate status transitions at runtime.
/// `Custom` kinds are permissive — all statuses are allowed.
pub fn valid_statuses(kind: &TaskKind) -> &'static [TaskStatus] {
    use TaskStatus::*;
    match kind {
        TaskKind::Theme => &[Active, Retired],
        TaskKind::Initiative => &[Proposed, Active, Complete, Cancelled],
        TaskKind::Epic => &[Ready, InProgress, Complete, Cancelled],
        TaskKind::Story => &[Ready, InProgress, InReview, Complete, Cancelled],
        TaskKind::Task => &[Blocked, Ready, InProgress, Complete, Failed, Cancelled],
        TaskKind::Subtask => &[Ready, Complete],
        TaskKind::Custom(_) => &[
            Proposed, Active, Retired, Blocked, Ready, InProgress, InReview, Complete, Failed,
            Cancelled,
        ],
    }
}

/// Definition for creating a new task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDef {
    /// Human-readable title.
    pub title: String,
    /// Full description of what needs to be done.
    pub description: String,
    /// Hierarchy level.
    pub kind: TaskKind,
    /// Parent in the hierarchy tree (not a blocking dependency).
    pub parent: Option<TaskId>,
    /// Task IDs that must complete before this task can start.
    pub blocked_by: Vec<TaskId>,
    /// Arbitrary structured metadata.
    ///
    /// Kind-specific fields live here: priority, story points,
    /// acceptance criteria, estimates, sprint assignment, etc.
    pub metadata: serde_json::Value,
}

/// A task in the task graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    /// Agent that created this task (Artifact provenance).
    pub producer: AgentId,
    pub title: String,
    pub description: String,
    /// Hierarchy level.
    pub kind: TaskKind,
    /// Current lifecycle status. Valid values depend on `kind`.
    pub status: TaskStatus,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    /// Agent currently working on this task (RACI: Responsible).
    pub assignee: Option<AgentId>,
    /// Parent in the hierarchy tree. A task has at most one parent.
    /// Hierarchy is a tree; dependencies (`blocked_by`/`blocks`) are a DAG.
    pub parent: Option<TaskId>,
    /// Tasks that must complete before this one (dependency DAG).
    pub blocked_by: Vec<TaskId>,
    /// Tasks that this one blocks (dependency DAG).
    pub blocks: Vec<TaskId>,
    /// Arbitrary structured metadata.
    ///
    /// Kind-specific fields: priority, story points, acceptance
    /// criteria, estimates, sprint assignment, start/due dates,
    /// RACI accountable agent, external refs, etc.
    pub metadata: serde_json::Value,
}

/// Partial update for a task's mutable fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Filter for task queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskFilter {
    /// Filter by lifecycle status.
    pub status: Option<TaskStatus>,
    /// Filter by hierarchy level.
    pub kind: Option<TaskKind>,
    /// Filter by assigned agent.
    pub assignee: Option<AgentId>,
    /// Filter by parent (direct children of a specific task).
    pub parent: Option<TaskId>,
    /// Maximum number of results.
    pub limit: Option<usize>,
}

/// Task graph persistence, hierarchy traversal, and query.
///
/// Tasks form a tree via `parent` (hierarchy) and a DAG via
/// `blocked_by`/`blocks` (dependencies). The store manages status
/// transitions, validates them per [`TaskKind`], and enforces
/// auto-transition rules.
///
/// ## Auto-transition rules
///
/// 1. **Readiness propagation:** When a child becomes `Ready`, if the
///    parent is below `Ready`, the parent transitions to `Ready`.
/// 2. **Readiness revert:** When the last `Ready` child leaves `Ready`
///    and no siblings are `Ready`, the parent reverts to its kind's
///    resting state.
/// 3. **Completion signal:** When all children are `Complete`, the
///    parent transitions to `Ready` (eligible for explicit closure).
/// 4. **No creation side-effects:** Creating a child does not change
///    parent status.
/// 5. **Status validation:** `update_status()` checks
///    [`valid_statuses`] for the task's kind.
///
/// Dyn-compatible for use behind `Arc<dyn TaskStore>` in
/// [`GyreContext`](crate::context::GyreContext).
pub trait TaskStore: Send + Sync {
    /// Create a new task. Returns its assigned ID.
    ///
    /// The store assigns an ID, sets `created_at`/`updated_at`,
    /// and derives the initial status from the task's kind.
    /// Does NOT trigger parent auto-transitions (rule 4).
    fn create_task(
        &self,
        task: &TaskDef,
    ) -> Pin<Box<dyn Future<Output = Result<TaskId, GyreError>> + Send + '_>>;

    /// Get a task by ID.
    fn get_task(
        &self,
        id: &TaskId,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Task>, GyreError>> + Send + '_>>;

    /// Update a task's status.
    ///
    /// Validates the transition against [`valid_statuses`] for the
    /// task's kind. Enforces auto-transition rules on the parent.
    fn update_status(
        &self,
        id: &TaskId,
        status: TaskStatus,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    /// Update a task's mutable fields (title, description, metadata).
    fn update_task(
        &self,
        id: &TaskId,
        update: &TaskUpdate,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    /// Assign a task to an agent (or unassign with `None`).
    fn assign(
        &self,
        id: &TaskId,
        assignee: Option<AgentId>,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    /// Move a task to a new parent (or make it a root with `None`).
    fn reparent(
        &self,
        id: &TaskId,
        new_parent: Option<TaskId>,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    // --- Dependency graph ---

    /// Add a blocking dependency: `blocked` cannot start until `blocked_by` completes.
    fn add_dependency(
        &self,
        blocked: &TaskId,
        blocked_by: &TaskId,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    /// Remove a blocking dependency.
    fn remove_dependency(
        &self,
        blocked: &TaskId,
        blocked_by: &TaskId,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    // --- Queries ---

    /// Find tasks with no unresolved blockers (all blockers Complete or Cancelled).
    fn ready_tasks(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;

    /// List tasks matching a filter.
    fn list_tasks(
        &self,
        filter: &TaskFilter,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;

    /// Get all tasks that block a given task (transitive dependency walk).
    /// `depth` limits traversal; `None` means unlimited.
    fn blockers(
        &self,
        id: &TaskId,
        depth: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;

    /// Get all tasks blocked by a given task (transitive).
    fn dependents(
        &self,
        id: &TaskId,
        depth: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;

    // --- Tree operations ---

    /// Get direct children of a task (one level down in the hierarchy).
    fn children(
        &self,
        id: &TaskId,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;

    /// Get ancestors of a task (walk parent pointers to root).
    fn ancestors(
        &self,
        id: &TaskId,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;

    /// Get all descendants of a task (recursive subtree).
    fn subtree(
        &self,
        id: &TaskId,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;
}
