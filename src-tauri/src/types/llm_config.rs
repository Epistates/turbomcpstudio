use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM Provider configuration with secure credential storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfiguration {
    /// Currently active provider
    pub active_provider: Option<String>,
    /// Provider configurations (without sensitive data)
    pub providers: HashMap<String, LLMProviderConfig>,
    /// Global configuration
    pub global_config: GlobalLLMConfig,
}

/// Configuration for a specific LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderConfig {
    /// Provider type (openai, anthropic, etc.)
    pub provider_type: LLMProviderType,
    /// Display name for the provider
    pub display_name: String,
    /// Whether this provider is enabled
    pub enabled: bool,
    /// Default model to use
    pub default_model: String,
    /// Available models for this provider
    pub available_models: Vec<String>,
    /// Custom base URL (optional)
    pub base_url: Option<String>,
    /// Organization ID (OpenAI specific)
    pub organization: Option<String>,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Timeout in seconds
    pub timeout_seconds: u64,
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    /// Cost configuration for usage tracking
    pub cost_config: CostConfig,
    /// 2024 API capabilities
    pub capabilities: LLMProviderCapabilities,
}

/// Supported LLM provider types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LLMProviderType {
    OpenAI,
    Anthropic,
    Local,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute limit
    pub requests_per_minute: u32,
    /// Tokens per minute limit (if applicable)
    pub tokens_per_minute: Option<u32>,
    /// Enable exponential backoff
    pub exponential_backoff: bool,
    /// Initial backoff delay in milliseconds
    pub initial_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds
    pub max_backoff_ms: u64,
}

/// Cost configuration for usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// Cost per 1k input tokens
    pub input_cost_per_1k: f64,
    /// Cost per 1k output tokens
    pub output_cost_per_1k: f64,
    /// Cost per 1k thinking tokens (Claude 4.1 series)
    pub thinking_cost_per_1k: Option<f64>,
    /// Currency (USD, EUR, etc.)
    pub currency: String,
}

/// 2025 LLM provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderCapabilities {
    /// Support for structured outputs with 100% reliability (OpenAI gpt-4o-2024-08-06+)
    pub supports_structured_outputs: bool,
    /// Models that support structured outputs
    pub structured_output_models: Vec<String>,
    /// Maximum tokens for structured outputs
    pub max_structured_output_tokens: Option<u32>,
    /// Support for batch processing (OpenAI Batch API, Anthropic Message Batches)
    pub supports_batch_processing: bool,
    /// Batch processing discount percentage
    pub batch_discount_percentage: Option<f64>,
    /// Support for parallel function calling
    pub supports_parallel_function_calling: bool,
    /// Support for function calling with strict mode
    pub supports_strict_function_calling: bool,
    /// Support for vision inputs (images)
    pub supports_vision: bool,
    /// Supported image formats for vision
    pub supported_image_formats: Vec<String>,
    /// Support for audio inputs/outputs
    pub supports_audio: bool,
    /// Supported audio formats
    pub supported_audio_formats: Vec<String>,
    /// Support for streaming responses
    pub supports_streaming: bool,
    /// Support for tool use/function calling
    pub supports_function_calling: bool,
    /// Support for computer use (Claude 3.5+ beta)
    pub supports_computer_use: bool,
    /// Support for thinking tokens pricing (Claude 4.1 series)
    pub supports_thinking_tokens: bool,
    /// Maximum context window in tokens
    pub max_context_tokens: Option<u32>,
}

/// Global LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalLLMConfig {
    /// Enable usage tracking
    pub enable_usage_tracking: bool,
    /// Daily spending limit in currency units
    pub daily_spending_limit: Option<f64>,
    /// Monthly spending limit in currency units
    pub monthly_spending_limit: Option<f64>,
    /// Enable cost warnings
    pub enable_cost_warnings: bool,
    /// Warning threshold (percentage of limit)
    pub cost_warning_threshold: f64,
    /// Default temperature for sampling
    pub default_temperature: f64,
    /// Default max tokens
    pub default_max_tokens: Option<u32>,
}

/// Request for updating LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLLMConfigRequest {
    pub provider_id: String,
    pub config: LLMProviderConfig,
}

/// Request for setting API key (handled securely)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetAPIKeyRequest {
    pub provider_id: String,
    pub api_key: String,
}

/// Response containing provider status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderStatus {
    pub provider_id: String,
    pub display_name: String,
    pub provider_type: LLMProviderType,
    pub enabled: bool,
    pub configured: bool,
    pub active: bool,
    pub available_models: Vec<String>,
    pub base_url: Option<String>,
    pub last_error: Option<String>,
    pub usage_stats: Option<ProviderUsageStats>,
}

/// Usage statistics for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsageStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost: f64,
    pub average_response_time_ms: f64,
    pub last_request_at: Option<String>,
}

impl Default for LLMConfiguration {
    fn default() -> Self {
        let mut providers = HashMap::new();

        // OpenAI configuration - September 2025 latest models
        providers.insert(
            "openai".to_string(),
            LLMProviderConfig {
                provider_type: LLMProviderType::OpenAI,
                display_name: "OpenAI".to_string(),
                enabled: false,
                default_model: "gpt-5".to_string(), // Latest GPT-5 released August 2025
                available_models: vec![
                    "gpt-5".to_string(),      // Latest flagship model - $1.25/$10 per 1M tokens
                    "gpt-5-mini".to_string(), // 80% performance, 20% cost - $0.25/$2 per 1M tokens
                    "gpt-5-nano".to_string(), // Basic tasks - $0.05/$0.40 per 1M tokens
                    "gpt-4o-2024-08-06".to_string(), // Structured outputs model
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                ],
                base_url: None,
                organization: None,
                max_retries: 3,
                timeout_seconds: 30,
                rate_limit: RateLimitConfig {
                    requests_per_minute: 100,         // Higher limits for 2025
                    tokens_per_minute: Some(1000000), // 1M token context support
                    exponential_backoff: true,
                    initial_backoff_ms: 1000,
                    max_backoff_ms: 30000,
                },
                cost_config: CostConfig {
                    input_cost_per_1k: 1.25, // GPT-5 actual pricing: $1.25 per 1M tokens (September 2025)
                    output_cost_per_1k: 10.00, // $10 per 1M tokens (includes reasoning tokens)
                    thinking_cost_per_1k: None, // OpenAI counts reasoning tokens as output tokens
                    currency: "USD".to_string(),
                },
                capabilities: LLMProviderCapabilities {
                    supports_structured_outputs: true,
                    structured_output_models: vec![
                        "gpt-5".to_string(),   // GPT-5 supports structured outputs
                        "gpt-4.1".to_string(), // GPT-4.1 enhanced capabilities
                        "gpt-4o-2024-08-06".to_string(),
                        "gpt-4o".to_string(),
                        "gpt-4o-mini".to_string(),
                    ],
                    max_structured_output_tokens: Some(200000), // 200K token support
                    supports_batch_processing: true,
                    batch_discount_percentage: Some(50.0),
                    supports_parallel_function_calling: true,
                    supports_strict_function_calling: true,
                    supports_vision: true,
                    supported_image_formats: vec![
                        "jpeg".to_string(),
                        "png".to_string(),
                        "gif".to_string(),
                        "webp".to_string(),
                    ],
                    supports_audio: true, // GPT-5 has audio capabilities
                    supported_audio_formats: vec!["wav".to_string(), "mp3".to_string()],
                    supports_streaming: true,
                    supports_function_calling: true,
                    supports_computer_use: false, // OpenAI doesn't have computer use yet
                    supports_thinking_tokens: true, // GPT-5 has reasoning_effort parameter
                    max_context_tokens: Some(400000), // 272K input + 128K reasoning/output = 400K total
                },
            },
        );

        // OpenAI GPT-5 Nano - Ultra-affordable option (September 2025)
        providers.insert(
            "openai-nano".to_string(),
            LLMProviderConfig {
                provider_type: LLMProviderType::OpenAI,
                display_name: "OpenAI GPT-5 Nano".to_string(),
                enabled: false,
                default_model: "gpt-5-nano".to_string(),
                available_models: vec![
                    "gpt-5-nano".to_string(),
                    "gpt-4o-mini".to_string(), // Fallback option
                ],
                base_url: None,
                organization: None,
                max_retries: 3,
                timeout_seconds: 30,
                rate_limit: RateLimitConfig {
                    requests_per_minute: 1000,        // Higher limits for nano model
                    tokens_per_minute: Some(5000000), // 5M tokens for ultra-cheap model
                    exponential_backoff: true,
                    initial_backoff_ms: 500,
                    max_backoff_ms: 15000,
                },
                cost_config: CostConfig {
                    input_cost_per_1k: 0.05,    // GPT-5 Nano actual pricing: $0.05 per 1M tokens
                    output_cost_per_1k: 0.40,   // $0.40 per 1M tokens - 25x cheaper than GPT-5
                    thinking_cost_per_1k: None, // OpenAI counts reasoning tokens as output
                    currency: "USD".to_string(),
                },
                capabilities: LLMProviderCapabilities {
                    supports_structured_outputs: true,
                    structured_output_models: vec!["gpt-5-nano".to_string()],
                    max_structured_output_tokens: Some(32768), // More limited than flagship
                    supports_batch_processing: true,
                    batch_discount_percentage: Some(50.0),
                    supports_parallel_function_calling: false, // Limited capabilities
                    supports_strict_function_calling: true,
                    supports_vision: false, // Nano model limitations
                    supported_image_formats: vec![],
                    supports_audio: false,
                    supported_audio_formats: vec![],
                    supports_streaming: true,
                    supports_function_calling: true,
                    supports_computer_use: false, // OpenAI doesn't have computer use yet
                    supports_thinking_tokens: false, // OpenAI uses different pricing model
                    max_context_tokens: Some(128000), // 128K context for nano model
                },
            },
        );

        // Claude 4 Sonnet - September 2025 latest model (May 2025 release)
        providers.insert(
            "claude-4-sonnet".to_string(),
            LLMProviderConfig {
                provider_type: LLMProviderType::Anthropic,
                display_name: "Claude 4 Sonnet".to_string(),
                enabled: false,
                default_model: "claude-4-sonnet".to_string(),
                available_models: vec![
                    "claude-4-sonnet".to_string(),
                    "claude-3-5-sonnet-20241022".to_string(), // Fallback
                ],
                base_url: None,
                organization: None,
                max_retries: 3,
                timeout_seconds: 45, // Higher timeout for complex reasoning
                rate_limit: RateLimitConfig {
                    requests_per_minute: 100,
                    tokens_per_minute: Some(1000000), // 1M token context support
                    exponential_backoff: true,
                    initial_backoff_ms: 1000,
                    max_backoff_ms: 45000,
                },
                cost_config: CostConfig {
                    input_cost_per_1k: 3.00, // Claude 4 Sonnet pricing (September 2025)
                    output_cost_per_1k: 15.00,
                    thinking_cost_per_1k: None, // Standard Sonnet doesn't use thinking tokens
                    currency: "USD".to_string(),
                },
                capabilities: LLMProviderCapabilities {
                    supports_structured_outputs: true,
                    structured_output_models: vec!["claude-4-sonnet".to_string()],
                    max_structured_output_tokens: Some(200000), // 200K token support
                    supports_batch_processing: true,
                    batch_discount_percentage: Some(50.0),
                    supports_parallel_function_calling: false, // Anthropic doesn't support parallel yet
                    supports_strict_function_calling: true,
                    supports_vision: true,
                    supported_image_formats: vec![
                        "jpeg".to_string(),
                        "png".to_string(),
                        "gif".to_string(),
                        "webp".to_string(),
                        "pdf".to_string(),
                    ],
                    supports_audio: false,
                    supported_audio_formats: vec![],
                    supports_streaming: true,
                    supports_function_calling: true,
                    supports_computer_use: true, // Claude 3.5+ computer use capabilities
                    supports_thinking_tokens: false, // Standard Sonnet model
                    max_context_tokens: Some(200000), // 200K context window
                },
            },
        );

        // Claude 4 Opus - Premium reasoning model (September 2025)
        providers.insert(
            "claude-4-opus".to_string(),
            LLMProviderConfig {
                provider_type: LLMProviderType::Anthropic,
                display_name: "Claude 4 Opus".to_string(),
                enabled: false,
                default_model: "claude-4-opus".to_string(),
                available_models: vec![
                    "claude-4-opus".to_string(),
                    "claude-4-sonnet".to_string(), // Fallback
                ],
                base_url: None,
                organization: None,
                max_retries: 3,
                timeout_seconds: 60, // Higher timeout for complex reasoning
                rate_limit: RateLimitConfig {
                    requests_per_minute: 50, // Lower rate limit for premium model
                    tokens_per_minute: Some(500000),
                    exponential_backoff: true,
                    initial_backoff_ms: 1500,
                    max_backoff_ms: 60000,
                },
                cost_config: CostConfig {
                    input_cost_per_1k: 15.00,   // Claude 4 Opus premium pricing
                    output_cost_per_1k: 75.00,  // 5x more expensive than Sonnet
                    thinking_cost_per_1k: None, // Standard Opus doesn't use thinking tokens
                    currency: "USD".to_string(),
                },
                capabilities: LLMProviderCapabilities {
                    supports_structured_outputs: true,
                    structured_output_models: vec!["claude-4-opus".to_string()],
                    max_structured_output_tokens: Some(200000),
                    supports_batch_processing: true,
                    batch_discount_percentage: Some(50.0),
                    supports_parallel_function_calling: false,
                    supports_strict_function_calling: true,
                    supports_vision: true,
                    supported_image_formats: vec![
                        "jpeg".to_string(),
                        "png".to_string(),
                        "gif".to_string(),
                        "webp".to_string(),
                        "pdf".to_string(),
                    ],
                    supports_audio: false,
                    supported_audio_formats: vec![],
                    supports_streaming: true,
                    supports_function_calling: true,
                    supports_computer_use: true,
                    supports_thinking_tokens: false, // Standard Opus model
                    max_context_tokens: Some(200000),
                },
            },
        );

        // Claude 4.1 Opus - Revolutionary thinking tokens model (September 2025)
        providers.insert(
            "claude-4.1-opus".to_string(),
            LLMProviderConfig {
                provider_type: LLMProviderType::Anthropic,
                display_name: "Claude 4.1 Opus (Thinking Tokens)".to_string(),
                enabled: false,
                default_model: "claude-4.1-opus".to_string(),
                available_models: vec![
                    "claude-4.1-opus".to_string(),
                    "claude-4-opus".to_string(), // Fallback
                ],
                base_url: None,
                organization: None,
                max_retries: 3,
                timeout_seconds: 120, // Much higher timeout for thinking token processing
                rate_limit: RateLimitConfig {
                    requests_per_minute: 30, // Lower rate limit for premium thinking model
                    tokens_per_minute: Some(300000),
                    exponential_backoff: true,
                    initial_backoff_ms: 2000,
                    max_backoff_ms: 120000,
                },
                cost_config: CostConfig {
                    input_cost_per_1k: 20.00, // Claude 4.1 Opus premium pricing
                    output_cost_per_1k: 80.00,
                    thinking_cost_per_1k: Some(40.00), // Revolutionary thinking tokens pricing
                    currency: "USD".to_string(),
                },
                capabilities: LLMProviderCapabilities {
                    supports_structured_outputs: true,
                    structured_output_models: vec!["claude-4.1-opus".to_string()],
                    max_structured_output_tokens: Some(200000),
                    supports_batch_processing: true,
                    batch_discount_percentage: Some(50.0),
                    supports_parallel_function_calling: false,
                    supports_strict_function_calling: true,
                    supports_vision: true,
                    supported_image_formats: vec![
                        "jpeg".to_string(),
                        "png".to_string(),
                        "gif".to_string(),
                        "webp".to_string(),
                        "pdf".to_string(),
                    ],
                    supports_audio: false,
                    supported_audio_formats: vec![],
                    supports_streaming: true,
                    supports_function_calling: true,
                    supports_computer_use: true,
                    supports_thinking_tokens: true, // Key feature - thinking tokens!
                    max_context_tokens: Some(200000),
                },
            },
        );

        // LOCAL PROVIDERS - No API keys required, local hosting
        // LM Studio - Popular GUI for GGUF models
        providers.insert(
            "lmstudio".to_string(),
            LLMProviderConfig {
                provider_type: LLMProviderType::Local,
                display_name: "LM Studio".to_string(),
                enabled: false,
                default_model: "local-model".to_string(),
                available_models: vec![
                    "local-model".to_string(),
                    "llama-3.1-8b-instruct".to_string(),
                    "llama-3.1-70b-instruct".to_string(),
                    "qwen2.5-72b-instruct".to_string(),
                ],
                base_url: Some("http://localhost:1234/v1".to_string()),
                organization: None,
                max_retries: 3,
                timeout_seconds: 60,
                rate_limit: RateLimitConfig {
                    requests_per_minute: 1000, // No rate limits for local
                    tokens_per_minute: None,
                    exponential_backoff: false,
                    initial_backoff_ms: 0,
                    max_backoff_ms: 0,
                },
                cost_config: CostConfig {
                    input_cost_per_1k: 0.0, // Free local usage
                    output_cost_per_1k: 0.0,
                    thinking_cost_per_1k: None,
                    currency: "USD".to_string(),
                },
                capabilities: LLMProviderCapabilities {
                    supports_structured_outputs: true,
                    structured_output_models: vec!["local-model".to_string()],
                    max_structured_output_tokens: None,
                    supports_batch_processing: false,
                    batch_discount_percentage: None,
                    supports_parallel_function_calling: false,
                    supports_strict_function_calling: false,
                    supports_vision: true,
                    supported_image_formats: vec!["jpeg".to_string(), "png".to_string()],
                    supports_audio: false,
                    supported_audio_formats: vec![],
                    supports_streaming: true,
                    supports_function_calling: true,
                    supports_computer_use: false,
                    supports_thinking_tokens: false,
                    max_context_tokens: Some(128000),
                },
            },
        );

        // Ollama - Popular local model runner
        providers.insert(
            "ollama".to_string(),
            LLMProviderConfig {
                provider_type: LLMProviderType::Local,
                display_name: "Ollama".to_string(),
                enabled: false,
                default_model: "llama3.1".to_string(),
                available_models: vec![
                    "llama3.1".to_string(),
                    "llama3.1:70b".to_string(),
                    "qwen2.5:72b".to_string(),
                    "deepseek-r1:8b".to_string(),
                    "deepseek-r1:32b".to_string(),
                ],
                base_url: Some("http://localhost:11434/v1".to_string()),
                organization: None,
                max_retries: 3,
                timeout_seconds: 120, // Ollama can be slower
                rate_limit: RateLimitConfig {
                    requests_per_minute: 1000,
                    tokens_per_minute: None,
                    exponential_backoff: false,
                    initial_backoff_ms: 0,
                    max_backoff_ms: 0,
                },
                cost_config: CostConfig {
                    input_cost_per_1k: 0.0,
                    output_cost_per_1k: 0.0,
                    thinking_cost_per_1k: None,
                    currency: "USD".to_string(),
                },
                capabilities: LLMProviderCapabilities {
                    supports_structured_outputs: true,
                    structured_output_models: vec![
                        "llama3.1".to_string(),
                        "qwen2.5:72b".to_string(),
                    ],
                    max_structured_output_tokens: None,
                    supports_batch_processing: false,
                    batch_discount_percentage: None,
                    supports_parallel_function_calling: false,
                    supports_strict_function_calling: false,
                    supports_vision: true,
                    supported_image_formats: vec![
                        "jpeg".to_string(),
                        "png".to_string(),
                        "webp".to_string(),
                    ],
                    supports_audio: false,
                    supported_audio_formats: vec![],
                    supports_streaming: true,
                    supports_function_calling: true,
                    supports_computer_use: false,
                    supports_thinking_tokens: true, // DeepSeek R1 supports reasoning
                    max_context_tokens: Some(128000),
                },
            },
        );

        // GPT4All - Free local models
        providers.insert(
            "gpt4all".to_string(),
            LLMProviderConfig {
                provider_type: LLMProviderType::Local,
                display_name: "GPT4All".to_string(),
                enabled: false,
                default_model: "gpt4all-falcon".to_string(),
                available_models: vec![
                    "gpt4all-falcon".to_string(),
                    "llama-3-8b-instruct".to_string(),
                    "phi-3-mini".to_string(),
                ],
                base_url: Some("http://localhost:4891/v1".to_string()),
                organization: None,
                max_retries: 3,
                timeout_seconds: 60,
                rate_limit: RateLimitConfig {
                    requests_per_minute: 1000,
                    tokens_per_minute: None,
                    exponential_backoff: false,
                    initial_backoff_ms: 0,
                    max_backoff_ms: 0,
                },
                cost_config: CostConfig {
                    input_cost_per_1k: 0.0,
                    output_cost_per_1k: 0.0,
                    thinking_cost_per_1k: None,
                    currency: "USD".to_string(),
                },
                capabilities: LLMProviderCapabilities {
                    supports_structured_outputs: false,
                    structured_output_models: vec![],
                    max_structured_output_tokens: None,
                    supports_batch_processing: false,
                    batch_discount_percentage: None,
                    supports_parallel_function_calling: false,
                    supports_strict_function_calling: false,
                    supports_vision: false,
                    supported_image_formats: vec![],
                    supports_audio: false,
                    supported_audio_formats: vec![],
                    supports_streaming: true,
                    supports_function_calling: false,
                    supports_computer_use: false,
                    supports_thinking_tokens: false,
                    max_context_tokens: Some(4096),
                },
            },
        );

        // Jan - Open-source ChatGPT alternative
        providers.insert(
            "jan".to_string(),
            LLMProviderConfig {
                provider_type: LLMProviderType::Local,
                display_name: "Jan".to_string(),
                enabled: false,
                default_model: "trinity-v1".to_string(),
                available_models: vec!["trinity-v1".to_string(), "llama-3.1-8b".to_string()],
                base_url: Some("http://localhost:1337/v1".to_string()),
                organization: None,
                max_retries: 3,
                timeout_seconds: 60,
                rate_limit: RateLimitConfig {
                    requests_per_minute: 1000,
                    tokens_per_minute: None,
                    exponential_backoff: false,
                    initial_backoff_ms: 0,
                    max_backoff_ms: 0,
                },
                cost_config: CostConfig {
                    input_cost_per_1k: 0.0,
                    output_cost_per_1k: 0.0,
                    thinking_cost_per_1k: None,
                    currency: "USD".to_string(),
                },
                capabilities: LLMProviderCapabilities {
                    supports_structured_outputs: true,
                    structured_output_models: vec!["trinity-v1".to_string()],
                    max_structured_output_tokens: None,
                    supports_batch_processing: false,
                    batch_discount_percentage: None,
                    supports_parallel_function_calling: false,
                    supports_strict_function_calling: false,
                    supports_vision: false,
                    supported_image_formats: vec![],
                    supports_audio: false,
                    supported_audio_formats: vec![],
                    supports_streaming: true,
                    supports_function_calling: true,
                    supports_computer_use: false,
                    supports_thinking_tokens: false,
                    max_context_tokens: Some(32768),
                },
            },
        );

        // Custom - Any OpenAI-compatible API
        providers.insert(
            "custom".to_string(),
            LLMProviderConfig {
                provider_type: LLMProviderType::Local,
                display_name: "Custom".to_string(),
                enabled: false,
                default_model: "custom-model".to_string(),
                available_models: vec!["custom-model".to_string()],
                base_url: Some("http://localhost:8000/v1".to_string()),
                organization: None,
                max_retries: 3,
                timeout_seconds: 60,
                rate_limit: RateLimitConfig {
                    requests_per_minute: 1000,
                    tokens_per_minute: None,
                    exponential_backoff: false,
                    initial_backoff_ms: 0,
                    max_backoff_ms: 0,
                },
                cost_config: CostConfig {
                    input_cost_per_1k: 0.0,
                    output_cost_per_1k: 0.0,
                    thinking_cost_per_1k: None,
                    currency: "USD".to_string(),
                },
                capabilities: LLMProviderCapabilities {
                    supports_structured_outputs: true,
                    structured_output_models: vec!["custom-model".to_string()],
                    max_structured_output_tokens: None,
                    supports_batch_processing: false,
                    batch_discount_percentage: None,
                    supports_parallel_function_calling: false,
                    supports_strict_function_calling: false,
                    supports_vision: false,
                    supported_image_formats: vec![],
                    supports_audio: false,
                    supported_audio_formats: vec![],
                    supports_streaming: true,
                    supports_function_calling: true,
                    supports_computer_use: false,
                    supports_thinking_tokens: false,
                    max_context_tokens: None,
                },
            },
        );

        Self {
            active_provider: None,
            providers,
            global_config: GlobalLLMConfig {
                enable_usage_tracking: true,
                daily_spending_limit: Some(10.0),
                monthly_spending_limit: Some(100.0),
                enable_cost_warnings: true,
                cost_warning_threshold: 80.0,
                default_temperature: 0.7,
                default_max_tokens: None,
            },
        }
    }
}

impl LLMProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LLMProviderType::OpenAI => "openai",
            LLMProviderType::Anthropic => "anthropic",
            LLMProviderType::Local => "local",
        }
    }
}
