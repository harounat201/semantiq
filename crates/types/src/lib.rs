use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const EMBEDDING_DIM: usize = 1536;

/// Normalized query after preprocessing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub raw: String,
    pub normalized: String,
    pub hash: String,
}

/// A 1536-dim embedding vector from text-embedding-3-small.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingVector(pub Vec<f32>);

/// A row stored in Postgres + Redis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub id: Uuid,
    pub query_hash: String,
    pub redis_key: String,
    pub created_at: DateTime<Utc>,
}

/// Output of the admission pipeline for a given query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdmissionDecision {
    Accept,
    Deny,
}

/// What the proxy found (or didn't) in the cache layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheResult {
    /// Full hit: returned from Redis without touching the LLM.
    KvHit { response: String },
    /// Vector matched but Redis key was missing (stale entry).
    VectorHitKvMiss { entry: CacheEntry },
    /// Nothing found.
    Miss,
}

/// A raw LLM response with metadata needed for admission gating.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub status_ok: bool,
    pub latency_ms: u64,
}
