use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
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
    /// Request JSON response format (2025 best practice for structured output)
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

/// Response format configuration for JSON output (OpenAI API 2025 standard)
#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI-compatible chat completion response
/// Allows extra fields to handle variations between OpenAI, LM Studio, and other compatible APIs
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    model: String,
    choices: Vec<Choice>,
    /// Usage metadata (deserialized by serde, not directly accessed)
    #[allow(dead_code)]
    #[serde(default)]
    usage: Option<Usage>,
    // Other fields like id, object, created, stats, system_fingerprint are allowed but ignored by serde default
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatResponseMessage,
    #[serde(default)]
    finish_reason: Option<String>,
    // Other fields like index, logprobs are allowed but ignored by serde default
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    /// Role field (deserialized by serde, not directly accessed)
    #[allow(dead_code)]
    role: String,
    content: String,
    /// Extended reasoning (used by LM Studio with OpenAI extended reasoning format)
    #[serde(default)]
    reasoning: Option<String>,
    // Other fields like tool_calls are allowed but ignored by serde default
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

        // Ensure minimum timeout for large prompts and slow models
        // Extended reasoning models need more time - 600 seconds (10 minutes) minimum
        let effective_timeout = std::cmp::max(timeout_seconds, 600);
        info!("Creating HTTP client with timeout: {} seconds (input was: {} seconds)", effective_timeout, timeout_seconds);

        let client = HttpClientBuilder::build(effective_timeout)?;

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
                    // v3 new variants for tool calling in sampling
                    turbomcp_protocol::types::ContentBlock::ToolUse(tool_use) => {
                        format!("[Tool call: {}]", tool_use.name)
                    }
                    turbomcp_protocol::types::ContentBlock::ToolResult(tool_result) => {
                        format!("[Tool result: {}]", tool_result.tool_use_id)
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
        let mut messages = Self::to_chat_messages(&request.messages);
        info!("Messages before prepending system prompt: {}", messages.len());

        // Prepend system prompt if provided (MCP protocol has separate system_prompt field)
        // OpenAI-compatible APIs expect system messages in the messages array
        if let Some(system_prompt) = &request.system_prompt {
            info!("SYSTEM PROMPT FOUND - prepending to messages array");
            info!("System prompt length: {}", system_prompt.len());
            messages.insert(0, ChatMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
            info!("Messages after prepending: {}", messages.len());
        } else {
            error!("WARNING: No system prompt provided in request!");
        }

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
        // Note: LM Studio and Ollama don't support response_format, so we only add it for cloud providers
        // The user must select a non-thinking model for proper JSON output
        let chat_request = ChatCompletionRequest {
            model,
            messages,
            max_tokens: Some(request.max_tokens), // Wrap in Option for OpenAI-compatible API
            temperature: request.temperature,
            stop: request.stop_sequences,
            response_format: None, // Local providers don't support structured output format
        };

        // Call API
        debug!("Sending request to {} API...", self.provider_name);
        let endpoint = format!("{}/chat/completions", self.base_url);
        info!("Endpoint: {}", endpoint);

        // Log the actual request body for debugging
        let request_body = serde_json::to_string(&chat_request)
            .unwrap_or_else(|_| "Failed to serialize request".to_string());
        debug!("Request body (first 500 chars): {}",
            if request_body.len() > 500 { format!("{}...", &request_body[..500]) } else { request_body.clone() });

        let http_response = self
            .client
            .post(&endpoint)
            .header("content-type", "application/json")
            .json(&chat_request)
            .send()
            .await
            .map_err(|e| {
                error!("HTTP request failed for {}: {}", self.provider_name, e);
                error!("Endpoint was: {}", endpoint);
                error!("Error source: {}", e.source().map(|s| s.to_string()).unwrap_or_default());
                e
            })?;

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
        info!("Response status: {}", http_response.status());
        let response_text = http_response.text().await.map_err(|e| {
            error!("Failed to read response body as text: {}", e);
            e
        })?;

        info!("Response body length: {} bytes", response_text.len());
        debug!("Response body (first 300 chars): {}",
            if response_text.len() > 300 { format!("{}...", &response_text[..300]) } else { response_text.clone() });

        let response: ChatCompletionResponse = serde_json::from_str(&response_text).map_err(|e| {
            error!("Failed to parse {} JSON response", self.provider_name);
            error!("Parse error: {}", e);
            error!("Response body was: {}", response_text);
            LLMProviderError::InvalidResponse(format!("JSON parse error: {} - Body: {}", e,
                if response_text.len() > 300 { format!("{}...", &response_text[..300]) } else { response_text.clone() }
            ))
        })?;

        debug!("Received response from {} API", self.provider_name);

        // Extract first choice
        let choice = response.choices.first().ok_or_else(|| {
            error!("{} response contained no choices", self.provider_name);
            LLMProviderError::InvalidResponse("No choices in response".to_string())
        })?;

        // Log what we received in the response
        info!("Response message content length: {} bytes", choice.message.content.len());
        info!("Response message has reasoning: {}", choice.message.reasoning.is_some());
        if let Some(reasoning) = &choice.message.reasoning {
            info!("Response reasoning length: {} bytes", reasoning.len());
        }
        debug!("Response content (first 200 chars): {}",
            if choice.message.content.len() > 200 { format!("{}...", &choice.message.content[..200]) } else { choice.message.content.clone() });

        // Use content field, but fall back to reasoning field if content is empty
        // (handles LM Studio extended reasoning format where content is empty and reasoning contains the actual response)
        let content = if choice.message.content.is_empty() {
            info!("Content field is empty, using reasoning field");
            choice.message.reasoning.as_ref().cloned().unwrap_or_default()
        } else {
            choice.message.content.clone()
        };

        if content.is_empty() {
            error!("{} response has no content and no reasoning", self.provider_name);
            return Err(LLMProviderError::InvalidResponse("Empty response from LLM".to_string()).into());
        }

        // Detect if response contains thinking tags (indicates thinking model)
        // Log as warning but don't fail - let JSON extraction handle it
        if content.contains("<think>") || content.contains("</think>") {
            warn!("{} returned thinking model output with <think> tags. Attempting to extract JSON from response.", self.provider_name);
            info!("Note: Thinking models include their reasoning process in responses. Our JSON extraction will attempt to recover the structured output after the </think> closing tag.");
        }

        // Check for completely empty responses (token limit exceeded or model failure)
        if content.trim().is_empty() {
            let error_msg = "LLM returned empty response. This typically indicates:\n1. Token limit was exceeded\n2. Model failed to generate output\n\nTry increasing max_tokens or switching to a different model.".to_string();
            error!("{}", error_msg);
            return Err(error_msg.into());
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lmstudio_response() {
        // Exact response from LM Studio curl
        let response_json = r#"{
  "id": "chatcmpl-xeo2sep964mtq9cw3ixj5o",
  "object": "chat.completion",
  "created": 1761700234,
  "model": "openai/gpt-oss-20b",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "{\"suite_name\":\"YouTube Transcript Retrieval\",\"description\":\"Tests for retrieving transcripts from YouTube URLs using the youtube_transcript server.\",\"tests\":[{\"name\":\"Retrieve transcript for valid YouTube URL\",\"description\":\"Ensures that the server returns a successful response with transcript data for a known video.\",\"category\":\"happy_path\",\"complexity\":\"simple\",\"kind\":{\"tool_call\":{\"tool_name\":\"youtube_transcript\",\"arguments\":{\"url\":\"https://www.youtube.com/watch?v=dQw4w9WgXcQ\"}}},\"test_data\":{\"url\":\"https://www.youtube.com/watch?v=dQw4w9WgXcQ\"},\"assertions\":[{\"status_equals\":{\"expected\":\"success\"}}]},{\"name\":\"Handle invalid YouTube URL\",\"description\":\"Verifies that the server returns an error status when provided with a non-existent or invalid video URL.\",\"category\":\"error_handling\",\"complexity\":\"simple\",\"kind\":{\"tool_call\":{\"tool_name\":\"youtube_transcript\",\"arguments\":{\"url\":\"https://www.youtube.com/watch?v=invalidvideo\"}}},\"test_data\":{\"url\":\"https://www.youtube.com/watch?v=invalidvideo\"},\"assertions\":[{\"status_equals\":{\"expected\":\"error\"}}]}]}",
        "reasoning": "We need to produce JSON object matching expected structure..."
      },
      "logprobs": null,
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 323,
    "completion_tokens": 1564,
    "total_tokens": 1887
  },
  "stats": {},
  "system_fingerprint": "openai/gpt-oss-20b"
}"#;

        // Try to deserialize
        let result: Result<ChatCompletionResponse, serde_json::Error> =
            serde_json::from_str(response_json);

        match result {
            Ok(response) => {
                println!("✅ Successfully parsed LM Studio response");
                println!("Model: {}", response.model);
                println!("Choices count: {}", response.choices.len());
                if let Some(choice) = response.choices.first() {
                    println!("Message content length: {}", choice.message.content.len());
                    println!("Has reasoning: {}", choice.message.reasoning.is_some());
                    println!("Finish reason: {:?}", choice.finish_reason);
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to parse LM Studio response");
                eprintln!("Error: {}", e);
                eprintln!("Error type: {:?}", e.classify());
                eprintln!("Error line: {}", e.line());
                eprintln!("Error column: {}", e.column());
                panic!("Deserialization failed: {}", e);
            }
        }
    }
}
