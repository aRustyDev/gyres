//! # gyres-tracing
//!
//! Telemetry layer for the Gyres agent harness.
//! Implements `TelemetrySink` for Langfuse (via langfuse-rs),
//! OpenTelemetry, and stdout/no-op backends.

#![forbid(unsafe_code)]
