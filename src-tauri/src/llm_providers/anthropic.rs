use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use tracing::{debug, error, info};
use turbomcp_client::sampling::{LLMServerClient, ServerInfo};
use turbomcp_protocol::types::{CreateMessageRequest, CreateMessageResult};

use super::shared::{
    AnthropicRequest, AnthropicResponse, HttpClientBuilder, LLMProviderError,
    MessageConverter,
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
            .and_then(|hint| hint.name.as_ref()).cloned()
            .unwrap_or_else(|| self.default_model.clone());

        debug!("Using Anthropic model: {}", model);

        // Build Anthropic request
        let anthropic_request = AnthropicRequest {
            model,
            messages,
            max_tokens: request.max_tokens,  // MCP 2025-06-18: Required field
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
            let error_body = http_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
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
        Ok(ServerInfo {
            name: "Anthropic".to_string(),
            models: vec![
                "claude-4-sonnet".to_string(),
                "claude-4-opus".to_string(),
                "claude-4.1-opus".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
            ],
            capabilities: vec![
                "function_calling".to_string(),
                "vision".to_string(),
                "computer_use".to_string(),
                "thinking_tokens".to_string(),
            ],
        })
    }
}
