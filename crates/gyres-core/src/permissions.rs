use std::future::Future;
use std::pin::Pin;

/// A request to perform an action, evaluated by the permission chain.
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    // TODO: agent identity, action, resource, context
}

/// The result of evaluating a permission request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Action is allowed.
    Allow,
    /// Action is denied with a reason.
    Deny(String),
    /// This gate has no opinion — pass to the next gate in the chain.
    Defer,
}

/// A single gate in the permission filter chain.
/// Dyn-compatible for use behind `Arc<dyn PermissionGate>` in GyreContext.
pub trait PermissionGate: Send + Sync {
    /// Evaluate a permission request.
    fn evaluate(
        &self,
        request: &PermissionRequest,
    ) -> Pin<Box<dyn Future<Output = Verdict> + Send + '_>>;
}
