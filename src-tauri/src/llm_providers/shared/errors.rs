use thiserror::Error;

/// Errors that can occur when interacting with LLM providers
#[derive(Debug, Error)]
pub enum LLMProviderError {
    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

// Note: No need for From implementation - Box<dyn Error>::from() already works
// for any type that implements Error trait, which LLMProviderError does via thiserror
