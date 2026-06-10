use crate::{VectorError, VectorHit, VectorStore};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use semantiq_types::{CacheEntry, EmbeddingVector};
use sqlx::{FromRow, PgPool};
use tracing::instrument;
use uuid::Uuid;

pub struct PgVectorStore {
    pool: PgPool,
}

impl PgVectorStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct CacheRow {
    id: Uuid,
    query_hash: String,
    redis_key: String,
    created_at: NaiveDateTime,
    distance: Option<f64>,
}

fn vec_to_pg(vec: &EmbeddingVector) -> String {
    format!(
        "[{}]",
        vec.0.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",")
    )
}

#[async_trait]
impl VectorStore for PgVectorStore {
    #[instrument(skip(self, vec))]
    async fn search(
        &self,
        vec: &EmbeddingVector,
        threshold: f64,
    ) -> Result<Option<VectorHit>, VectorError> {
        let vec_lit = vec_to_pg(vec);

        let row = sqlx::query_as::<_, CacheRow>(
            r#"
            SELECT id, query_hash, redis_key, created_at,
                   (embedding <=> $1::vector) AS distance
            FROM cache_entries
            ORDER BY embedding <=> $1::vector
            LIMIT 1
            "#,
        )
        .bind(&vec_lit)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| {
            let distance = r.distance.unwrap_or(f64::MAX);
            tracing::debug!(distance, threshold, "vector search distance");

            if distance <= threshold {
                Some(VectorHit {
                    entry: CacheEntry {
                        id: r.id,
                        query_hash: r.query_hash,
                        redis_key: r.redis_key,
                        created_at: r.created_at.and_utc(),
                    },
                    distance,
                })
            } else {
                None
            }
        }))
    }

    #[instrument(skip(self, vec))]
    async fn insert(
        &self,
        query_hash: &str,
        redis_key: &str,
        vec: &EmbeddingVector,
    ) -> Result<Uuid, VectorError> {
        let id = Uuid::new_v4();
        let vec_lit = vec_to_pg(vec);
        let now = Utc::now().naive_utc();

        sqlx::query(
            r#"
            INSERT INTO cache_entries (id, query_hash, redis_key, embedding, created_at)
            VALUES ($1, $2, $3, $4::vector, $5)
            "#,
        )
        .bind(id)
        .bind(query_hash)
        .bind(redis_key)
        .bind(&vec_lit)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    #[instrument(skip(self))]
    async fn prune_older_than(&self, days: i64) -> Result<u64, VectorError> {
        let result = sqlx::query(
            "DELETE FROM cache_entries WHERE created_at < NOW() - make_interval(days => $1)",
        )
        .bind(days as i32)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
