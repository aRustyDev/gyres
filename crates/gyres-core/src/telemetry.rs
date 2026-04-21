/// Unique identifier for a telemetry span.
pub type SpanId = u64;

/// Span-based telemetry sink. Always present in
/// [`GyreContext`](crate::context::GyreContext), even if no-op.
///
/// Implementations route to Langfuse, OTEL, stdout, or `/dev/null`.
/// Parent-child relationships are established at span creation via
/// the `parent` parameter.
///
/// Implementations should buffer internally and drop on overflow
/// (matching Langfuse SDK behavior). Never panic in the export path.
pub trait TelemetrySink: Send + Sync {
    /// Start a new span. Returns a [`SpanId`] for referencing it.
    /// Pass `parent: Some(id)` to nest under an existing span.
    fn start_span(&self, name: &str, parent: Option<SpanId>) -> SpanId;

    /// End a span.
    fn end_span(&self, id: SpanId);

    /// Set a key-value attribute on a span.
    fn set_attribute(&self, span: SpanId, key: &str, value: &str);

    /// Record a point-in-time event within a span.
    fn record_event(&self, span: SpanId, event: &str);

    /// Flush all buffered telemetry data.
    fn flush(&self);
}

/// No-op telemetry sink for testing or when telemetry is disabled.
pub struct NoopTelemetry;

impl TelemetrySink for NoopTelemetry {
    fn start_span(&self, _name: &str, _parent: Option<SpanId>) -> SpanId {
        0
    }
    fn end_span(&self, _id: SpanId) {}
    fn set_attribute(&self, _span: SpanId, _key: &str, _value: &str) {}
    fn record_event(&self, _span: SpanId, _event: &str) {}
    fn flush(&self) {}
}
