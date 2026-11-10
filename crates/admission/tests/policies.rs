use async_trait::async_trait;
use semantiq_admission::{
    composite::CompositePolicy, frequency::FrequencyPolicy, latency::LatencyPolicy, AdmissionPolicy,
};
use semantiq_cache::{CacheError, KvStore};
use semantiq_types::{AdmissionDecision, LlmResponse, Query};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// In-memory KvStore; only the sorted-set ops are meaningful for admission.
#[derive(Default)]
struct MemStore {
    scores: Mutex<HashMap<(String, String), f64>>,
    fail: bool,
}

impl MemStore {
    fn failing() -> Self {
        Self { fail: true, ..Default::default() }
    }
}

#[async_trait]
impl KvStore for MemStore {
    async fn get(&self, _key: &str) -> Result<Option<String>, CacheError> {
        Ok(None)
    }
    async fn set(&self, _key: &str, _value: &str, _ttl: Duration) -> Result<(), CacheError> {
        Ok(())
    }
    async fn increment_score(&self, set_key: &str, member: &str) -> Result<f64, CacheError> {
        if self.fail {
            return Err(CacheError::Redis(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "simulated failure",
            ))));
        }
        let mut scores = self.scores.lock().unwrap();
        let s = scores.entry((set_key.to_string(), member.to_string())).or_insert(0.0);
        *s += 1.0;
        Ok(*s)
    }
    async fn get_score(&self, set_key: &str, member: &str) -> Result<Option<f64>, CacheError> {
        Ok(self.scores.lock().unwrap().get(&(set_key.to_string(), member.to_string())).copied())
    }
    async fn ping(&self) -> Result<(), CacheError> {
        Ok(())
    }
}

fn query(hash: &str) -> Query {
    Query { raw: hash.into(), normalized: hash.into(), hash: hash.into() }
}

fn response(status_ok: bool, latency_ms: u64) -> LlmResponse {
    LlmResponse { content: "answer".into(), status_ok, latency_ms }
}

// ---- LatencyPolicy ----

#[tokio::test]
async fn latency_accepts_at_and_above_threshold() {
    let p = LatencyPolicy::new(500);
    assert_eq!(p.evaluate(&query("q"), &response(true, 500)).await, AdmissionDecision::Accept);
    assert_eq!(p.evaluate(&query("q"), &response(true, 1200)).await, AdmissionDecision::Accept);
}

#[tokio::test]
async fn latency_denies_below_threshold() {
    let p = LatencyPolicy::new(500);
    assert_eq!(p.evaluate(&query("q"), &response(true, 499)).await, AdmissionDecision::Deny);
}

#[tokio::test]
async fn latency_zero_threshold_admits_everything() {
    let p = LatencyPolicy::new(0);
    assert_eq!(p.evaluate(&query("q"), &response(true, 0)).await, AdmissionDecision::Accept);
}

// ---- FrequencyPolicy ----

#[tokio::test]
async fn frequency_accepts_once_threshold_reached() {
    let store = Arc::new(MemStore::default());
    let p = FrequencyPolicy::new(store, 3);
    let q = query("hot");
    assert_eq!(p.evaluate(&q, &response(true, 10)).await, AdmissionDecision::Deny);
    assert_eq!(p.evaluate(&q, &response(true, 10)).await, AdmissionDecision::Deny);
    assert_eq!(p.evaluate(&q, &response(true, 10)).await, AdmissionDecision::Accept);
    assert_eq!(p.evaluate(&q, &response(true, 10)).await, AdmissionDecision::Accept);
}

#[tokio::test]
async fn frequency_tracks_queries_independently() {
    let store = Arc::new(MemStore::default());
    let p = FrequencyPolicy::new(store, 2);
    assert_eq!(p.evaluate(&query("a"), &response(true, 10)).await, AdmissionDecision::Deny);
    assert_eq!(p.evaluate(&query("b"), &response(true, 10)).await, AdmissionDecision::Deny);
    assert_eq!(p.evaluate(&query("a"), &response(true, 10)).await, AdmissionDecision::Accept);
}

#[tokio::test]
async fn frequency_denies_errored_responses_without_counting() {
    let store = Arc::new(MemStore::default());
    let p = FrequencyPolicy::new(store.clone(), 1);
    assert_eq!(p.evaluate(&query("q"), &response(false, 10)).await, AdmissionDecision::Deny);
    assert_eq!(store.get_score("semantiq:query_frequency", "q").await.unwrap(), None);
}

#[tokio::test]
async fn frequency_denies_when_store_fails() {
    let p = FrequencyPolicy::new(Arc::new(MemStore::failing()), 1);
    assert_eq!(p.evaluate(&query("q"), &response(true, 10)).await, AdmissionDecision::Deny);
}

// ---- CompositePolicy ----

#[tokio::test]
async fn composite_requires_all_policies_to_accept() {
    let store = Arc::new(MemStore::default());
    let p = CompositePolicy::new(vec![
        Box::new(FrequencyPolicy::new(store, 1)),
        Box::new(LatencyPolicy::new(500)),
    ]);
    assert_eq!(p.evaluate(&query("fast"), &response(true, 100)).await, AdmissionDecision::Deny);
    assert_eq!(p.evaluate(&query("slow"), &response(true, 900)).await, AdmissionDecision::Accept);
}

#[tokio::test]
async fn composite_empty_accepts() {
    let p = CompositePolicy::new(vec![]);
    assert_eq!(p.evaluate(&query("q"), &response(true, 0)).await, AdmissionDecision::Accept);
}

/// CompositePolicy short-circuits on the first Deny. FrequencyPolicy must run
/// first (as wired in proxy/src/state.rs) so every request is counted, even
/// fast ones that LatencyPolicy would reject.
#[tokio::test]
async fn composite_frequency_first_counts_every_request() {
    let store = Arc::new(MemStore::default());
    let p = CompositePolicy::new(vec![
        Box::new(FrequencyPolicy::new(store.clone(), 10)),
        Box::new(LatencyPolicy::new(500)),
    ]);
    for _ in 0..3 {
        p.evaluate(&query("q"), &response(true, 100)).await;
    }
    assert_eq!(store.get_score("semantiq:query_frequency", "q").await.unwrap(), Some(3.0));
}
