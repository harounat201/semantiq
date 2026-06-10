use anyhow::Result;
use reqwest::Client;
use semantiq_types::LlmResponse;
use serde::Deserialize;
use std::time::Instant;
use tracing::instrument;

const CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct LlmClient {
    api_key: String,
    client: Client,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

impl LlmClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
        }
    }

    #[instrument(skip(self, query))]
    pub async fn complete(&self, query: &str) -> Result<LlmResponse> {
        let body = serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [{ "role": "user", "content": query }]
        });

        let start = Instant::now();

        let resp = self
            .client
            .post(CHAT_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;
        let status_ok = resp.status().is_success();

        let content = if status_ok {
            let parsed: ChatResponse = resp.json().await?;
            parsed
                .choices
                .into_iter()
                .next()
                .map(|c| c.message.content)
                .unwrap_or_default()
        } else {
            let status = resp.status().as_u16();
            tracing::warn!(status, "LLM returned non-2xx, not admitting to cache");
            resp.text().await.unwrap_or_default()
        };

        Ok(LlmResponse {
            content,
            status_ok,
            latency_ms,
        })
    }
}
