---
status: accepted
date: 2026-04-20
tags: [telemetry, tracing, cross-cutting]
---

# TelemetrySink in Core, tracing Bridge in gyres-tracing

## Context

Rust's `tracing` crate is the standard for structured logging and instrumentation. Langfuse and OTEL are span-based observability platforms. Both could serve as the telemetry mechanism.

## Decision

Two separate concerns:

1. **`tracing` crate** = developer diagnostics ("what is the code doing?"). Used directly by gyres internals via `tracing::info!()`, `tracing::span!()`, etc.
2. **`TelemetrySink` trait** = agent observability ("what is the agent doing?"). Defined in gyres-core, routes to Langfuse/OTEL.

`gyres-core` defines `TelemetrySink` without depending on the `tracing` crate.
`gyres-tracing` provides a `tracing::Layer` that bridges `tracing` spans to `TelemetrySink`.

## Consequences

- gyres-core has no `tracing` dependency — lighter, more portable.
- Users who want `tracing` integration get it via gyres-tracing.
- Users who don't want `tracing` use `TelemetrySink` directly.
- Both paths can coexist — `tracing` for dev logging, `TelemetrySink` for Langfuse.
- The bridge in gyres-tracing means `tracing::span!()` macros can automatically flow to Langfuse if desired.
