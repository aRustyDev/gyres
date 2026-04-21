//! # gyres-core
//!
//! Domain-agnostic traits and types for the Gyres agent harness.
//!
//! This crate defines the core abstractions that all Gyres components build on:
//!
//! - [`Agent`](agent::Agent) — what the agent does (step, feedback, reset)
//! - [`Gyre`](gyre::Gyre) — how the feedback loop runs (the execution strategy)
//! - [`GyreContext`](context::GyreContext) — shared infrastructure (permissions, state, telemetry)
//!
//! Domain-specific concerns (LLM messages, RL state vectors) are expressed
//! through generic associated types, not hard-coded in this crate.

#![forbid(unsafe_code)]

pub mod agent;
pub mod config;
pub mod context;
pub mod error;
pub mod gyre;
pub mod permissions;
pub mod state;
pub mod telemetry;
pub mod types;
