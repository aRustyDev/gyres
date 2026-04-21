use std::future::Future;
use std::pin::Pin;

use crate::error::GyreError;

/// Marker trait for storage backends.
///
/// A backend is a concrete storage engine (SQLite, SurrealDB, in-memory)
/// that can satisfy one or more Store traits. One backend instance can
/// back all four store traits simultaneously — different tables or
/// collections in the same database.
///
/// Backends live in the `gyres-store` crate, feature-gated per engine.
/// This trait lives in `gyres-core` so store traits can reference it.
///
/// # Lifecycle
///
/// Backends are created by the storage factory from [`StorageConfig`].
/// The factory returns a [`Stores`] bundle with individual store trait
/// objects. Backends should be cheap to clone (wrap internals in `Arc`).
///
/// # Health checks
///
/// [`health_check`](Backend::health_check) verifies connectivity and
/// readiness. Call it at startup and periodically for monitoring.
/// Backends that need no connection (in-memory) return `Ok(())`.
pub trait Backend: Send + Sync + 'static {
    /// Human-readable name for diagnostics (e.g., "sqlite", "surreal", "memory").
    fn name(&self) -> &str;

    /// Check connectivity and readiness.
    ///
    /// For database backends: verify the connection is alive.
    /// For in-memory backends: always returns `Ok(())`.
    fn health_check(&self) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>>;

    /// Graceful shutdown. Flush pending writes, close connections.
    ///
    /// Default implementation does nothing.
    fn shutdown(&self) -> Pin<Box<dyn Future<Output = Result<(), GyreError>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

/// Configuration for selecting and initializing a storage backend.
///
/// Used by the storage factory in `gyres-store` to construct the
/// appropriate backend and bind it to all store traits.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum StorageConfig {
    /// In-memory storage. No persistence. Default for testing.
    Memory,

    /// SQLite file-based storage.
    Sqlite {
        /// Path to the SQLite database file.
        path: std::path::PathBuf,
    },

    /// SurrealDB storage (native graph + vector support).
    Surreal {
        /// Connection URL (e.g., "ws://localhost:8000").
        url: String,
    },
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self::Memory
    }
}
