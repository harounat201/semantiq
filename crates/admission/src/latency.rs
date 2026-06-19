use crate::AdmissionPolicy;
use async_trait::async_trait;
use semantiq_types::{AdmissionDecision, LlmResponse, Query};
use tracing::instrument;

pub struct LatencyPolicy {
    min_latency_ms: u64,
}

impl LatencyPolicy {
    pub fn new(min_latency_ms: u64) -> Self {
        Self { min_latency_ms }
    }
}

#[async_trait]
impl AdmissionPolicy for LatencyPolicy {
    #[instrument(skip(self, query), fields(hash = %query.hash))]
    async fn evaluate(&self, query: &Query, response: &LlmResponse) -> AdmissionDecision {
        if response.latency_ms >= self.min_latency_ms {
            tracing::debug!(
                latency_ms = response.latency_ms,
                min = self.min_latency_ms,
                "latency check passed"
            );
            AdmissionDecision::Accept
        } else {
            tracing::debug!(
                latency_ms = response.latency_ms,
                min = self.min_latency_ms,
                "latency below threshold, denying admission"
            );
            AdmissionDecision::Deny
        }
    }
}
