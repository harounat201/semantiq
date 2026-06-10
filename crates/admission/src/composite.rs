use crate::AdmissionPolicy;
use async_trait::async_trait;
use semantiq_types::{AdmissionDecision, LlmResponse, Query};

/// ANDs multiple policies: all must Accept for the query to be admitted.
pub struct CompositePolicy {
    policies: Vec<Box<dyn AdmissionPolicy>>,
}

impl CompositePolicy {
    pub fn new(policies: Vec<Box<dyn AdmissionPolicy>>) -> Self {
        Self { policies }
    }
}

#[async_trait]
impl AdmissionPolicy for CompositePolicy {
    async fn evaluate(&self, query: &Query, response: &LlmResponse) -> AdmissionDecision {
        for policy in &self.policies {
            if policy.evaluate(query, response).await == AdmissionDecision::Deny {
                return AdmissionDecision::Deny;
            }
        }
        AdmissionDecision::Accept
    }
}
