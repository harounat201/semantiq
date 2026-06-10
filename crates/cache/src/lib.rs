use async_trait::async_trait;
use deadpool_redis::{Config as PoolConfig, Pool, Runtime};
use redis::AsyncCommands;
use std::time::Duration;
use thiserror::Error;
use tracing::instrument;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("pool error: {0}")]
    Pool(#[from] deadpool_redis::PoolError),
}

#[async_trait]
pub trait KvStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, CacheError>;
    async fn set(&self, key: &str, value: &str, ttl: Duration) -> Result<(), CacheError>;
    async fn increment_score(&self, set_key: &str, member: &str) -> Result<f64, CacheError>;
    async fn get_score(&self, set_key: &str, member: &str) -> Result<Option<f64>, CacheError>;
}

pub struct RedisKvStore {
    pool: Pool,
}

impl RedisKvStore {
    pub fn new(url: &str) -> anyhow::Result<Self> {
        let cfg = PoolConfig::from_url(url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl KvStore for RedisKvStore {
    #[instrument(skip(self))]
    async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        let mut conn = self.pool.get().await?;
        let val: Option<String> = conn.get(key).await?;
        Ok(val)
    }

    #[instrument(skip(self, value))]
    async fn set(&self, key: &str, value: &str, ttl: Duration) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let secs = ttl.as_secs();
        conn.set_ex::<_, _, ()>(key, value, secs).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn increment_score(&self, set_key: &str, member: &str) -> Result<f64, CacheError> {
        let mut conn = self.pool.get().await?;
        let score: f64 = conn.zincr(set_key, member, 1.0_f64).await?;
        Ok(score)
    }

    #[instrument(skip(self))]
    async fn get_score(&self, set_key: &str, member: &str) -> Result<Option<f64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let score: Option<f64> = conn.zscore(set_key, member).await?;
        Ok(score)
    }
}
