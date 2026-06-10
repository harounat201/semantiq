use anyhow::Result;
use semantiq_admission::{composite::CompositePolicy, frequency::FrequencyPolicy, AdmissionPolicy};
use semantiq_cache::{KvStore, RedisKvStore};
use semantiq_config::Config;
use semantiq_embedding::{openai::OpenAIEmbedder, Embedder};
use semantiq_vector::{pg::PgVectorStore, VectorStore};
use sqlx::PgPool;
use std::sync::Arc;
use crate::llm::{AnthropicClient, LlmProvider, OpenAiClient};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub embedder: Arc<dyn Embedder>,
    pub vector_store: Arc<dyn VectorStore>,
    pub kv_store: Arc<dyn KvStore>,
    pub admission: Arc<dyn AdmissionPolicy>,
    pub llm: Arc<dyn LlmProvider>,
    pub pool: PgPool,
}

impl AppState {
    pub fn new(config: Config, pool: PgPool) -> Result<Self> {
        let kv_store: Arc<dyn KvStore> = Arc::new(RedisKvStore::new(&config.redis_url)?);

        let admission: Arc<dyn AdmissionPolicy> = Arc::new(CompositePolicy::new(vec![
            Box::new(FrequencyPolicy::new(kv_store.clone(), config.admission_frequency)),
        ]));

        let llm: Arc<dyn LlmProvider> = match config.llm_provider.as_str() {
            "anthropic" => {
                let key = config.anthropic_api_key.clone()
                    .ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY required when LLM_PROVIDER=anthropic"))?;
                Arc::new(AnthropicClient::new(key, &config.llm_model))
            }
            _ => Arc::new(OpenAiClient::new(&config.openai_api_key, &config.llm_model)),
        };

        Ok(Self {
            embedder: Arc::new(OpenAIEmbedder::new(&config.openai_api_key)),
            vector_store: Arc::new(PgVectorStore::new(pool.clone())),
            kv_store,
            admission,
            llm,
            pool,
            config,
        })
    }
}
