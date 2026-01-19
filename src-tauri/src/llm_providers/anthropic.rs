use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use tracing::{debug, error, info};
use turbomcp_client::sampling::{LLMServerClient, ServerInfo};
use turbomcp_protocol::types::{CreateMessageRequest, CreateMessageResult};

use super::shared::{
    AnthropicRequest, AnthropicResponse, HttpClientBuilder, LLMProviderError, MessageConverter,
};
use super::stop_reason_mapping::{map_stop_reason, LLMProvider};

/// Anthropic Claude client using direct REST API
#[derive(Debug, Clone)]
pub struct AnthropicLLMClient {
    client: Arc<Client>,
    api_key: String,
    default_model: String,
    base_url: String,
}

impl AnthropicLLMClient {
    /// Create new Anthropic client with API key
    pub fn new(
        api_key: String,
        default_model: String,
        timeout_seconds: u64,
    ) -> Result<Self, LLMProviderError> {
        info!("Creating Anthropic client with model: {}", default_model);

        let client = HttpClientBuilder::build(timeout_seconds)?;

        Ok(Self {
            client: Arc::new(client),
            api_key,
            default_model,
            base_url: "https://api.anthropic.com/v1".to_string(),
        })
    }

    /// Create client with custom base URL (for proxy/testing)
    /// TODO(enterprise): Support self-hosted Anthropic-compatible endpoints
    #[allow(dead_code)]
    pub fn with_base_url(
        api_key: String,
        default_model: String,
        timeout_seconds: u64,
        base_url: String,
    ) -> Result<Self, LLMProviderError> {
        info!(
            "Creating Anthropic client with custom base URL: {}",
            base_url
        );

        let client = HttpClientBuilder::build(timeout_seconds)?;

        Ok(Self {
            client: Arc::new(client),
            api_key,
            default_model,
            base_url,
        })
    }
}

#[async_trait]
impl LLMServerClient for AnthropicLLMClient {
    async fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<CreateMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            "Anthropic create_message called with {} messages",
            request.messages.len()
        );

        // Convert MCP messages to Anthropic format
        let messages = MessageConverter::to_anthropic_messages(&request.messages);

        // Extract model name from MCP 2025-06-18 model preferences structure
        let model = request
            .model_preferences
            .as_ref()
            .and_then(|prefs| prefs.hints.as_ref())
            .and_then(|hints| hints.first())
            .and_then(|hint| hint.name.as_ref())
            .cloned()
            .unwrap_or_else(|| self.default_model.clone());

        debug!("Using Anthropic model: {}", model);

        // Build Anthropic request
        let anthropic_request = AnthropicRequest {
            model,
            messages,
            max_tokens: request.max_tokens, // MCP 2025-06-18: Required field
            temperature: request.temperature,
            stop_sequences: request.stop_sequences,
        };

        // Call Anthropic API
        debug!("Sending request to Anthropic API...");
        let http_response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&anthropic_request)
            .send()
            .await?;

        // Check for HTTP errors
        let status = http_response.status();
        if !status.is_success() {
            let error_body = http_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Anthropic API error ({}): {}", status, error_body);

            return Err(match status.as_u16() {
                401 => LLMProviderError::AuthError(format!("Invalid API key: {}", error_body)),
                429 => LLMProviderError::RateLimitError(error_body),
                404 => LLMProviderError::ModelNotFound(error_body),
                _ => LLMProviderError::ApiError(format!("{}: {}", status, error_body)),
            }
            .into());
        }

        // Parse response
        let response: AnthropicResponse = http_response.json().await.map_err(|e| {
            error!("Failed to parse Anthropic response: {}", e);
            LLMProviderError::InvalidResponse(e.to_string())
        })?;

        debug!("Received response from Anthropic API");

        // Extract text from content array
        let text = response
            .content
            .first()
            .ok_or_else(|| {
                error!("Anthropic response contained no content");
                LLMProviderError::InvalidResponse("No content in response".to_string())
            })?
            .text
            .clone();

        // Map Anthropic stop_reason to MCP StopReason enum
        debug!("Anthropic stop_reason: {}", response.stop_reason);
        let stop_reason = map_stop_reason(LLMProvider::Anthropic, &response.stop_reason);

        info!(
            "Anthropic request completed - model: {}, stop_reason: {:?}, tokens: {}/{}",
            response.model, stop_reason, response.usage.input_tokens, response.usage.output_tokens
        );

        // Create MCP result
        Ok(MessageConverter::create_text_result(
            text,
            response.model,
            stop_reason,
        ))
    }

    async fn get_server_info(
        &self,
    ) -> Result<ServerInfo, Box<dyn std::error::Error + Send + Sync>> {
        // Model list updated January 2026 - Claude Opus 4.5 is latest flagship
        Ok(ServerInfo {
            name: "Anthropic".to_string(),
            models: vec![
                // Claude 4.5 family (November 2025 - latest)
                "claude-opus-4-5-20251101".to_string(),   // Latest flagship
                "claude-sonnet-4-5-20251101".to_string(), // Best balance of intelligence/speed/cost
                // Claude 4.x family (GA)
                "claude-sonnet-4-20250514".to_string(),
                "claude-opus-4-1-20250414".to_string(),
                // Aliases for latest versions
                "claude-opus-4-5".to_string(),
                "claude-sonnet-4-5".to_string(),
            ],
            capabilities: vec![
                "function_calling".to_string(),
                "vision".to_string(),
                "computer_use".to_string(),
                "thinking_tokens".to_string(),
                "1m_context".to_string(),        // Sonnet 4/4.5 expanded context
                "agent_skills".to_string(),      // Skills-2025-10-02 beta
                "code_execution".to_string(),    // Code-execution-2025-05-22 beta
            ],
        })
    }
}
