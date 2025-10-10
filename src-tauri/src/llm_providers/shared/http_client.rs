use super::errors::LLMProviderError;
use reqwest::Client;
use std::time::Duration;

/// Shared HTTP client configuration for all LLM providers
pub struct HttpClientBuilder;

impl HttpClientBuilder {
    /// Build a configured HTTP client with optimal settings
    pub fn build(timeout_seconds: u64) -> Result<Client, LLMProviderError> {
        Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .pool_max_idle_per_host(10) // Connection pooling
            .pool_idle_timeout(Duration::from_secs(90))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(LLMProviderError::NetworkError)
    }
}
