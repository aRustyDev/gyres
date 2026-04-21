use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Top-level harness configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// How often to flush telemetry and state buffers.
    pub flush_interval: Duration,
    /// Maximum items in event queues before dropping.
    pub max_queue_size: usize,
    /// Base directory for agent config and state (`~/.agents/`).
    pub agents_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            flush_interval: Duration::from_secs(5),
            max_queue_size: 100_000,
            agents_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".agents"),
        }
    }
}
