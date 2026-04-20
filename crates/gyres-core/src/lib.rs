//! # gyres-core
//!
//! Core traits and types for the Gyres agent harness.
//!
//! This crate defines the domain-agnostic abstractions that all Gyres
//! components build on: the `Agent` and `Gyre` traits, `GyreContext`,
//! and the trait interfaces for permissions, state, and telemetry.

#![forbid(unsafe_code)]

pub mod agent;
pub mod context;
pub mod error;
pub mod gyre;
pub mod permissions;
pub mod state;
pub mod telemetry;
