use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use turbomcp_client::sampling::{LLMServerClient, ServerInfo};
use turbomcp_protocol::types::{CreateMessageRequest, CreateMessageResult};

use super::shared::{HttpClientBuilder, LLMProviderError, MessageConverter};
use super::stop_reason_mapping::{map_stop_reason, LLMProvider};

/// Generic OpenAI-compatible client for local providers (Ollama, LMStudio, etc.)
///
/// This client uses the OpenAI chat completions API format, which is widely supported
/// by local LLM providers.
#[derive(Debug, Clone)]
pub struct OpenAICompatibleClient {
    client: Arc<Client>,
    base_url: String,
    default_model: String,
    provider_name: String,
    provider_type: LLMProvider,
}

/// OpenAI-compatible chat completion request
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI-compatible chat completion response
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    model: String,
    choices: Vec<Choice>,
    /// Usage metadata (deserialized by serde, not directly accessed)
    #[allow(dead_code)]
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    /// Role field (deserialized by serde, not directly accessed)
    #[allow(dead_code)]
    role: String,
    content: String,
}

/// OpenAI-compatible token usage (future metrics feature)
/// TODO(metrics): Expose token counts in CreateMessageResult
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

impl OpenAICompatibleClient {
    /// Create new client for Ollama
    pub fn new_ollama(
        base_url: String,
        default_model: String,
        timeout_seconds: u64,
    ) -> Result<Self, LLMProviderError> {
        Self::new(
            base_url,
            default_model,
            "Ollama".to_string(),
            LLMProvider::Ollama,
            timeout_seconds,
        )
    }

    /// Create new client for LMStudio
    pub fn new_lmstudio(
        base_url: String,
        default_model: String,
        timeout_seconds: u64,
    ) -> Result<Self, LLMProviderError> {
        Self::new(
            base_url,
            default_model,
            "LMStudio".to_string(),
            LLMProvider::LMStudio,
            timeout_seconds,
        )
    }

    /// Create new generic OpenAI-compatible client
    fn new(
        base_url: String,
        default_model: String,
        provider_name: String,
        provider_type: LLMProvider,
        timeout_seconds: u64,
    ) -> Result<Self, LLMProviderError> {
        info!(
            "Creating {} client with model: {} at {}",
            provider_name, default_model, base_url
        );

        let client = HttpClientBuilder::build(timeout_seconds)?;

        Ok(Self {
            client: Arc::new(client),
            base_url,
            default_model,
            provider_name,
            provider_type,
        })
    }

    /// Convert MCP messages to OpenAI format
    fn to_chat_messages(
        messages: &[turbomcp_protocol::types::SamplingMessage],
    ) -> Vec<ChatMessage> {
        messages
            .iter()
            .map(|msg| {
                // Note: MCP Role enum only has User and Assistant (no System)
                let role = match msg.role {
                    turbomcp_protocol::types::Role::User => "user",
                    turbomcp_protocol::types::Role::Assistant => "assistant",
                };

                let content = match &msg.content {
                    turbomcp_protocol::types::ContentBlock::Text(text) => text.text.clone(),
                    turbomcp_protocol::types::ContentBlock::Image(img) => {
                        format!("[Image: {}]", img.mime_type)
                    }
                    turbomcp_protocol::types::ContentBlock::Audio(audio) => {
                        format!("[Audio: {}]", audio.mime_type)
                    }
                    turbomcp_protocol::types::ContentBlock::ResourceLink(_) => {
                        "[Resource link]".to_string()
                    }
                    turbomcp_protocol::types::ContentBlock::Resource(_) => {
                        "[Embedded resource]".to_string()
                    }
                };

                ChatMessage {
                    role: role.to_string(),
                    content,
                }
            })
            .collect()
    }
}

#[async_trait]
impl LLMServerClient for OpenAICompatibleClient {
    async fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<CreateMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            "{} create_message called with {} messages",
            self.provider_name,
            request.messages.len()
        );

        // Convert messages to OpenAI format
        let messages = Self::to_chat_messages(&request.messages);

        // Extract model name from MCP 2025-06-18 model preferences structure
        let model = request
            .model_preferences
            .as_ref()
            .and_then(|prefs| prefs.hints.as_ref())
            .and_then(|hints| hints.first())
            .and_then(|hint| hint.name.as_ref())
            .cloned()
            .unwrap_or_else(|| self.default_model.clone());

        debug!("Using {} model: {}", self.provider_name, model);

        // Build request
        let chat_request = ChatCompletionRequest {
            model,
            messages,
            max_tokens: Some(request.max_tokens), // Wrap in Option for OpenAI-compatible API
            temperature: request.temperature,
            stop: request.stop_sequences,
        };

        // Call API
        debug!("Sending request to {} API...", self.provider_name);
        let endpoint = format!("{}/chat/completions", self.base_url);

        let http_response = self
            .client
            .post(&endpoint)
            .header("content-type", "application/json")
            .json(&chat_request)
            .send()
            .await?;

        // Check for HTTP errors
        let status = http_response.status();
        if !status.is_success() {
            let error_body = http_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!(
                "{} API error ({}): {}",
                self.provider_name, status, error_body
            );

            return Err(match status.as_u16() {
                404 => LLMProviderError::ModelNotFound(format!("Model not found: {}", error_body)),
                _ => LLMProviderError::ApiError(format!("{}: {}", status, error_body)),
            }
            .into());
        }

        // Parse response
        let response: ChatCompletionResponse = http_response.json().await.map_err(|e| {
            error!("Failed to parse {} response: {}", self.provider_name, e);
            LLMProviderError::InvalidResponse(e.to_string())
        })?;

        debug!("Received response from {} API", self.provider_name);

        // Extract first choice
        let choice = response.choices.first().ok_or_else(|| {
            error!("{} response contained no choices", self.provider_name);
            LLMProviderError::InvalidResponse("No choices in response".to_string())
        })?;

        let content = choice.message.content.clone();

        // Map finish_reason to MCP StopReason enum
        let stop_reason = if let Some(finish_reason) = &choice.finish_reason {
            debug!("{} finish_reason: {}", self.provider_name, finish_reason);
            map_stop_reason(self.provider_type, finish_reason)
        } else {
            warn!(
                "No finish_reason in {} response, defaulting to EndTurn",
                self.provider_name
            );
            turbomcp_protocol::types::StopReason::EndTurn
        };

        info!(
            "{} request completed - model: {}, stop_reason: {:?}",
            self.provider_name, response.model, stop_reason
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
            name: self.provider_name.clone(),
            models: vec![self.default_model.clone()],
            capabilities: vec!["streaming".to_string(), "function_calling".to_string()],
        })
    }
}
