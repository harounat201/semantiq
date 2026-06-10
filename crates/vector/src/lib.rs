pub mod pg;

use async_trait::async_trait;
use semantiq_types::{CacheEntry, EmbeddingVector};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum VectorError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

/// A vector hit with the associated distance score for logging/tuning.
#[derive(Debug)]
pub struct VectorHit {
    pub entry: CacheEntry,
    /// Cosine distance (0 = identical, 2 = opposite). Lower is better.
    pub distance: f64,
}

#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Find the closest cached entry within `threshold` cosine distance.
    /// Always returns the raw distance so callers can log it for threshold tuning.
    async fn search(
        &self,
        vec: &EmbeddingVector,
        threshold: f64,
    ) -> Result<Option<VectorHit>, VectorError>;

    /// Insert a new cache entry with its embedding. Returns the assigned UUID.
    async fn insert(
        &self,
        query_hash: &str,
        redis_key: &str,
        vec: &EmbeddingVector,
    ) -> Result<Uuid, VectorError>;

    /// Remove entries older than `days` days (TTL pruning).
    async fn prune_older_than(&self, days: i64) -> Result<u64, VectorError>;
}
