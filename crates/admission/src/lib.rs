pub mod composite;
pub mod frequency;

use async_trait::async_trait;
use semantiq_types::{AdmissionDecision, LlmResponse, Query};

/// A single admission rule. Multiple rules are composed via `CompositePolicy`.
#[async_trait]
pub trait AdmissionPolicy: Send + Sync {
    async fn evaluate(&self, query: &Query, response: &LlmResponse) -> AdmissionDecision;
}
