use async_openai::config::OpenAIConfig;
use async_openai::types::CreateChatCompletionRequestArgs;
use async_openai::Client as OpenAISDKClient;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use turbomcp_client::sampling::{LLMServerClient, ServerInfo};
use turbomcp_protocol::types::{CreateMessageRequest, CreateMessageResult};

use super::shared::{LLMProviderError, MessageConverter};
use super::stop_reason_mapping::{map_stop_reason, LLMProvider};

/// OpenAI LLM client using official async-openai SDK
#[derive(Debug, Clone)]
pub struct OpenAILLMClient {
    client: Arc<OpenAISDKClient<OpenAIConfig>>,
    default_model: String,
    provider_id: String,
}

impl OpenAILLMClient {
    /// Create new OpenAI client with API key
    pub fn new(api_key: String, default_model: String, base_url: Option<String>) -> Self {
        info!(
            "Creating OpenAI client with model: {}",
            default_model
        );

        let mut config = OpenAIConfig::new().with_api_key(api_key);

        // Support custom base URLs (for Azure OpenAI, etc.)
        if let Some(url) = base_url {
            info!("Using custom OpenAI base URL: {}", url);
            config = config.with_api_base(url);
        }

        Self {
            client: Arc::new(OpenAISDKClient::with_config(config)),
            default_model,
            provider_id: "openai".to_string(),
        }
    }
}

#[async_trait]
impl LLMServerClient for OpenAILLMClient {
    async fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<CreateMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        debug!("OpenAI create_message called with {} messages", request.messages.len());

        // Convert MCP messages to OpenAI format
        let messages = MessageConverter::to_openai_messages(&request.messages);

        // Extract model name from MCP 2025-06-18 model preferences structure
        let model = request
            .model_preferences
            .as_ref()
            .and_then(|prefs| prefs.hints.as_ref())
            .and_then(|hints| hints.first())
            .and_then(|hint| hint.name.as_ref()).cloned()
            .unwrap_or_else(|| self.default_model.clone());

        debug!("Using OpenAI model: {}", model);

        // Build request
        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder
            .model(&model)
            .messages(messages)
            .max_tokens(request.max_tokens);  // MCP 2025-06-18: Always required

        if let Some(temperature) = request.temperature {
            // OpenAI SDK expects f32, MCP uses f64
            request_builder.temperature(temperature as f32);
        }

        if let Some(stop_sequences) = request.stop_sequences {
            if !stop_sequences.is_empty() {
                request_builder.stop(stop_sequences);
            }
        }

        let chat_request = request_builder
            .build()
            .map_err(|e| LLMProviderError::ConfigError(e.to_string()))?;

        // Call OpenAI API
        debug!("Sending request to OpenAI API...");
        let response = self
            .client
            .chat()
            .create(chat_request)
            .await
            .map_err(|e| {
                error!("OpenAI API error: {}", e);
                LLMProviderError::ApiError(e.to_string())
            })?;

        debug!("Received response from OpenAI API");

        // Extract first choice
        let choice = response.choices.first().ok_or_else(|| {
            error!("OpenAI response contained no choices");
            LLMProviderError::InvalidResponse("No choices in response".to_string())
        })?;

        // Extract message content
        let content = choice
            .message
            .content
            .as_ref()
            .ok_or_else(|| {
                error!("OpenAI choice message has no content");
                LLMProviderError::InvalidResponse("No content in message".to_string())
            })?
            .clone();

        // Map OpenAI finish_reason to MCP StopReason enum
        let stop_reason = if let Some(finish_reason) = &choice.finish_reason {
            // Convert finish_reason to string for mapping
            let reason_str = format!("{:?}", finish_reason);
            debug!("OpenAI finish_reason: {}", reason_str);
            map_stop_reason(LLMProvider::OpenAI, &reason_str.to_lowercase())
        } else {
            warn!("No finish_reason in OpenAI response, defaulting to EndTurn");
            turbomcp_protocol::types::StopReason::EndTurn
        };

        info!(
            "OpenAI request completed - model: {}, stop_reason: {:?}",
            response.model, stop_reason
        );

        // Create MCP result
        Ok(MessageConverter::create_text_result(
            content,
            response.model,
            stop_reason,
        ))
    }

    async fn get_server_info(
        &self,
    ) -> Result<ServerInfo, Box<dyn std::error::Error + Send + Sync>> {
        Ok(ServerInfo {
            name: "OpenAI".to_string(),
            models: vec![
                "gpt-5".to_string(),
                "gpt-5-mini".to_string(),
                "gpt-5-nano".to_string(),
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
            ],
            capabilities: vec![
                "structured_outputs".to_string(),
                "function_calling".to_string(),
                "vision".to_string(),
                "audio".to_string(),
            ],
        })
    }
}
