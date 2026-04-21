---
status: accepted
date: 2026-04-20
tags: [telemetry, gyres-core]
---

# Span-Based Telemetry API Over Flat Events

## Context

Telemetry can be modeled as flat events (SpanStart/SpanEnd pairs) or as an explicit span tree (start_span returns SpanId, used as parent for children).

## Decision

Span-based API from the start:

```rust
fn start_span(&self, name: &str, parent: Option<SpanId>) -> SpanId;
fn end_span(&self, id: SpanId);
fn set_attribute(&self, span: SpanId, key: &str, value: &str);
fn record_event(&self, span: SpanId, event: &str);
fn flush(&self);
```

## Consequences

- Parent-child relationships are explicit at creation time — no reconstruction needed.
- Maps directly to OTEL spans and Langfuse observations.
- NoopTelemetry is trivial (return 0, ignore everything).
- Avoids a breaking change when integrating with Langfuse/OTEL later.
- Slightly more complex than flat events — Gyre implementations pass SpanIds through the loop.

## Alternatives Considered

- **Flat events (SpanStart/SpanEnd)**: Simpler trait but the sink must reconstruct the tree by matching Start/End pairs. Fragile with async/out-of-order events. Would require a breaking change to add span IDs later.
