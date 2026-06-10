use tracing::info;

pub struct RequestMetrics {
    pub query_hash: String,
    pub vector_hit: bool,
    pub kv_hit: bool,
    pub admitted: bool,
    pub latency_ms: u64,
}

/// Emit a single structured log line per request.
/// This feeds CloudWatch Logs Insights queries.
pub fn emit(m: &RequestMetrics) {
    info!(
        query_hash = %m.query_hash,
        vector_hit = m.vector_hit,
        kv_hit = m.kv_hit,
        admitted = m.admitted,
        latency_ms = m.latency_ms,
        "request completed"
    );
}
