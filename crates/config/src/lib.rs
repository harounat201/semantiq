use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    // --- auth ---
    pub api_key: String,

    // --- openai ---
    pub openai_api_key: String,

    // --- anthropic ---
    #[serde(default)]
    pub anthropic_api_key: Option<String>,

    // --- llm ---
    /// "openai" or "anthropic". Default: "openai"
    #[serde(default = "defaults::llm_provider")]
    pub llm_provider: String,

    /// Model for completions. Default: "gpt-4o-mini"
    #[serde(default = "defaults::llm_model")]
    pub llm_model: String,

    // --- postgres ---
    pub database_url: String,

    // --- redis ---
    pub redis_url: String,

    // --- similarity threshold ---
    /// Cosine distance cutoff for a vector hit (lower = stricter). Default: 0.08
    #[serde(default = "defaults::similarity_threshold")]
    pub similarity_threshold: f64,

    // --- admission ---
    /// How many times a query must appear before it's eligible for caching. Default: 3
    #[serde(default = "defaults::admission_frequency")]
    pub admission_frequency: u32,

    // --- cache TTL ---
    /// Redis TTL in seconds. Default: 86400 (24h)
    #[serde(default = "defaults::cache_ttl_secs")]
    pub cache_ttl_secs: u64,

    // --- server ---
    #[serde(default = "defaults::port")]
    pub port: u16,
}

mod defaults {
    pub fn similarity_threshold() -> f64 { 0.08 }
    pub fn admission_frequency() -> u32 { 3 }
    pub fn cache_ttl_secs() -> u64 { 86_400 }
    pub fn port() -> u16 { 8080 }
    pub fn llm_provider() -> String { "openai".to_string() }
    pub fn llm_model() -> String { "gpt-4o-mini".to_string() }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        envy::from_env::<Config>().context(
            "Failed to load config from environment. Check that API_KEY, OPENAI_API_KEY, DATABASE_URL, and REDIS_URL are set."
        )
    }
}
