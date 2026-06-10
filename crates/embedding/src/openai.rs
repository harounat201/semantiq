use crate::{EmbedError, Embedder};
use async_trait::async_trait;
use reqwest::Client;
use semantiq_types::EmbeddingVector;
use serde::Deserialize;
use tracing::instrument;

const EMBED_URL: &str = "https://api.openai.com/v1/embeddings";
const MODEL: &str = "text-embedding-3-small";

pub struct OpenAIEmbedder {
    api_key: String,
    client: Client,
}

impl OpenAIEmbedder {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
        }
    }
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

#[derive(Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

#[async_trait]
impl Embedder for OpenAIEmbedder {
    #[instrument(skip(self, text), fields(chars = text.len()))]
    async fn embed(&self, text: &str) -> Result<EmbeddingVector, EmbedError> {
        if text.is_empty() {
            return Err(EmbedError::EmptyQuery);
        }

        let body = serde_json::json!({ "input": text, "model": MODEL });

        let resp = self
            .client
            .post(EMBED_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = resp.text().await.unwrap_or_default();
            return Err(EmbedError::Api { status, body });
        }

        let parsed: EmbedResponse = resp.json().await?;
        parsed
            .data
            .into_iter()
            .next()
            .map(|d| EmbeddingVector(d.embedding))
            .ok_or(EmbedError::NoData)
    }
}
