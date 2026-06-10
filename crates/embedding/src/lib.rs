pub mod openai;
pub mod preprocess;

use async_trait::async_trait;
use semantiq_types::EmbeddingVector;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("empty query after preprocessing")]
    EmptyQuery,
    #[error("OpenAI API error {status}: {body}")]
    Api { status: u16, body: String },
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("no embeddings returned")]
    NoData,
}

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<EmbeddingVector, EmbedError>;
}
