use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::GyreError;
use crate::types::AgentId;

/// Unique identifier for a task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

/// Task lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Waiting for blockers to resolve.
    Blocked,
    /// All blockers resolved, ready to be claimed.
    Ready,
    /// Claimed by an agent, work in progress.
    InProgress,
    /// Work completed successfully.
    Complete,
    /// Work failed or was abandoned.
    Failed,
    /// Intentionally cancelled.
    Cancelled,
}

/// Definition for creating a new task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDef {
    /// Human-readable title.
    pub title: String,
    /// Full description of what needs to be done.
    pub description: String,
    /// Task IDs that must complete before this task can start.
    pub blocked_by: Vec<TaskId>,
    /// Arbitrary structured metadata.
    pub metadata: serde_json::Value,
}

/// A task in the task graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    /// Agent currently working on this task, if any.
    pub assignee: Option<AgentId>,
    /// Tasks that must complete before this one.
    pub blocked_by: Vec<TaskId>,
    /// Tasks that this one blocks.
    pub blocks: Vec<TaskId>,
    pub metadata: serde_json::Value,
}

/// Filter for task queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskFilter {
    pub status: Option<TaskStatus>,
    pub assignee: Option<AgentId>,
    pub limit: Option<usize>,
}

/// Task graph persistence and query.
///
/// Tasks form a DAG via blocking relationships. The store manages
/// status transitions and dependency resolution.
///
/// Dyn-compatible for use behind `Arc<dyn TaskStore>` in
/// [`GyreContext`](crate::context::GyreContext).
pub trait TaskStore: Send + Sync {
    /// Create a new task. Returns its assigned ID.
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
    fn update_status(
        &self,
        id: &TaskId,
        status: TaskStatus,
    ) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

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

    /// Find tasks with no unresolved blockers (all blockers Complete or Cancelled).
    fn ready_tasks(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;

    /// List tasks matching a filter.
    fn list_tasks(
        &self,
        filter: &TaskFilter,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Task>, GyreError>> + Send + '_>>;

    /// Get all tasks that block a given task (transitive).
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
}
