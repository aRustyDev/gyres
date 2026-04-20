//! # gyres
//!
//! A Rust agent harness — general-purpose feedback loop framework
//! for LLM, RL, and beyond.
//!
//! This is the convenience re-export crate. Enable feature flags
//! to pull in specific subsystems:
//!
//! - `llm` (default) — LLM agent loop, tool registry, context management
//! - `tracing` (default) — Telemetry backends (Langfuse, OTEL)
//! - `runtime` — Async executor, worktree-aware sessions
//! - `polar` — Embedded permission policy engine
//! - `mcp` — Model Context Protocol bridge
//! - `full` — Everything

#![forbid(unsafe_code)]

pub use gyres_core::*;

#[cfg(feature = "llm")]
pub use gyres_llm as llm;

#[cfg(feature = "runtime")]
pub use gyres_runtime as runtime;

#[cfg(feature = "polar")]
pub use gyres_polar as polar;

#[cfg(feature = "tracing")]
pub use gyres_tracing as telemetry;

#[cfg(feature = "mcp")]
pub use gyres_mcp as mcp;
