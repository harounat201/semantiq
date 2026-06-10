use crate::AdmissionPolicy;
use async_trait::async_trait;
use semantiq_cache::KvStore;
use semantiq_types::{AdmissionDecision, LlmResponse, Query};
use std::sync::Arc;
use tracing::instrument;

const FREQ_SET_KEY: &str = "semantiq:query_frequency";

pub struct FrequencyPolicy {
    store: Arc<dyn KvStore>,
    threshold: u32,
}

impl FrequencyPolicy {
    pub fn new(store: Arc<dyn KvStore>, threshold: u32) -> Self {
        Self { store, threshold }
    }
}

#[async_trait]
impl AdmissionPolicy for FrequencyPolicy {
    #[instrument(skip(self, response), fields(hash = %query.hash))]
    async fn evaluate(&self, query: &Query, response: &LlmResponse) -> AdmissionDecision {
        // Reject errored responses immediately — never cache failures.
        if !response.status_ok {
            tracing::debug!("response not ok, denying admission");
            return AdmissionDecision::Deny;
        }

        let score = match self.store.increment_score(FREQ_SET_KEY, &query.hash).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "frequency increment failed, defaulting to deny");
                return AdmissionDecision::Deny;
            }
        };

        tracing::debug!(score, threshold = self.threshold, "frequency check");

        if score >= self.threshold as f64 {
            AdmissionDecision::Accept
        } else {
            AdmissionDecision::Deny
        }
    }
}
