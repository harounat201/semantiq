use anyhow::Result;
use semantiq_admission::{
    composite::CompositePolicy, frequency::FrequencyPolicy, AdmissionPolicy,
};
use semantiq_cache::RedisKvStore;
use semantiq_config::Config;
use semantiq_embedding::{openai::OpenAIEmbedder, Embedder};
use semantiq_vector::{pg::PgVectorStore, VectorStore};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub embedder: Arc<dyn Embedder>,
    pub vector_store: Arc<dyn VectorStore>,
    pub kv_store: Arc<RedisKvStore>,
    pub admission: Arc<dyn AdmissionPolicy>,
    pub pool: PgPool,
}

impl AppState {
    pub fn new(config: Config, pool: PgPool) -> Result<Self> {
        let kv_store = Arc::new(RedisKvStore::new(&config.redis_url)?);

        let admission: Arc<dyn AdmissionPolicy> = Arc::new(CompositePolicy::new(vec![
            Box::new(FrequencyPolicy::new(
                kv_store.clone(),
                config.admission_frequency,
            )),
        ]));

        Ok(Self {
            embedder: Arc::new(OpenAIEmbedder::new(&config.openai_api_key)),
            vector_store: Arc::new(PgVectorStore::new(pool.clone())),
            kv_store,
            admission,
            pool,
            config,
        })
    }
}
