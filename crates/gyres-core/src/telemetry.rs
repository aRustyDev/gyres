/// A telemetry event emitted by a Gyre or Agent.
#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    // TODO: span-like structure, key-value attributes
}

/// Sink for telemetry data. Always present in GyreContext.
/// Implementations route to Langfuse, OTEL, stdout, or /dev/null.
pub trait TelemetrySink: Send + Sync {
    fn record(&self, event: TelemetryEvent);
    fn flush(&self);
}

/// No-op telemetry sink for testing or when telemetry is disabled.
pub struct NoopTelemetry;

impl TelemetrySink for NoopTelemetry {
    fn record(&self, _event: TelemetryEvent) {}
    fn flush(&self) {}
}
