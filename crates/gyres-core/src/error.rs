use thiserror::Error;

#[derive(Error, Debug)]
pub enum GyreError {
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("agent error: {0}")]
    Agent(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("state store error: {0}")]
    State(String),

    #[error("telemetry error: {0}")]
    Telemetry(String),

    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
