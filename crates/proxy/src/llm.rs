use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use semantiq_types::LlmResponse;
use serde::Deserialize;
use std::time::Instant;
use tracing::instrument;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, query: &str) -> Result<LlmResponse>;
}

// --- OpenAI ---

pub struct OpenAiClient {
    api_key: String,
    model: String,
    client: Client,
}

impl OpenAiClient {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self { api_key: api_key.into(), model: model.into(), client: Client::new() }
    }
}

#[derive(Deserialize)]
struct OpenAiResponse { choices: Vec<OpenAiChoice> }
#[derive(Deserialize)]
struct OpenAiChoice { message: OpenAiMessage }
#[derive(Deserialize)]
struct OpenAiMessage { content: String }

#[async_trait]
impl LlmProvider for OpenAiClient {
    #[instrument(skip(self, query))]
    async fn complete(&self, query: &str) -> Result<LlmResponse> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{ "role": "user", "content": query }]
        });

        let start = Instant::now();
        let resp = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;
        let status_ok = resp.status().is_success();

        let content = if status_ok {
            resp.json::<OpenAiResponse>().await?
                .choices.into_iter().next()
                .map(|c| c.message.content)
                .unwrap_or_default()
        } else {
            tracing::warn!(status = resp.status().as_u16(), "OpenAI returned non-2xx");
            resp.text().await.unwrap_or_default()
        };

        Ok(LlmResponse { content, status_ok, latency_ms })
    }
}

// --- Anthropic ---

pub struct AnthropicClient {
    api_key: String,
    model: String,
    client: Client,
}

impl AnthropicClient {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self { api_key: api_key.into(), model: model.into(), client: Client::new() }
    }
}

#[derive(Deserialize)]
struct AnthropicResponse { content: Vec<AnthropicContent> }
#[derive(Deserialize)]
struct AnthropicContent { text: String }

#[async_trait]
impl LlmProvider for AnthropicClient {
    #[instrument(skip(self, query))]
    async fn complete(&self, query: &str) -> Result<LlmResponse> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": [{ "role": "user", "content": query }]
        });

        let start = Instant::now();
        let resp = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;
        let status_ok = resp.status().is_success();

        let content = if status_ok {
            resp.json::<AnthropicResponse>().await?
                .content.into_iter().next()
                .map(|c| c.text)
                .unwrap_or_default()
        } else {
            tracing::warn!(status = resp.status().as_u16(), "Anthropic returned non-2xx");
            resp.text().await.unwrap_or_default()
        };

        Ok(LlmResponse { content, status_ok, latency_ms })
    }
}
