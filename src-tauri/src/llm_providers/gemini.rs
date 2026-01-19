use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use turbomcp_client::sampling::{LLMServerClient, ServerInfo};
use turbomcp_protocol::types::{CreateMessageRequest, CreateMessageResult};

use super::shared::{HttpClientBuilder, LLMProviderError, MessageConverter};

/// Google Gemini client using direct REST API
#[derive(Debug, Clone)]
pub struct GeminiLLMClient {
    client: Arc<Client>,
    api_key: String,
    default_model: String,
    base_url: String,
}

/// Gemini request format
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

/// Gemini response format
#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    /// Usage metadata (deserialized by serde, not directly accessed)
    #[allow(dead_code)]
    #[serde(default)]
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
    /// Role field (deserialized by serde, not directly accessed)
    #[allow(dead_code)]
    role: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    text: String,
}

/// Gemini token usage (future metrics feature)
/// TODO(metrics): Expose token counts in CreateMessageResult
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GeminiUsage {
    prompt_token_count: u64,
    candidates_token_count: u64,
    total_token_count: u64,
}

impl GeminiLLMClient {
    /// Create new Gemini client with API key
    pub fn new(
        api_key: String,
        default_model: String,
        timeout_seconds: u64,
    ) -> Result<Self, LLMProviderError> {
        info!("Creating Gemini client with model: {}", default_model);

        let client = HttpClientBuilder::build(timeout_seconds)?;

        Ok(Self {
            client: Arc::new(client),
            api_key,
            default_model,
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        })
    }

    /// Convert MCP messages to Gemini format
    fn to_gemini_contents(
        messages: &[turbomcp_protocol::types::SamplingMessage],
    ) -> Vec<GeminiContent> {
        messages
            .iter()
            .map(|msg| {
                // Note: MCP Role enum only has User and Assistant (no System)
                let role = match msg.role {
                    turbomcp_protocol::types::Role::User => "user",
                    turbomcp_protocol::types::Role::Assistant => "model",
                };

                let text = match &msg.content {
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

                GeminiContent {
                    role: role.to_string(),
                    parts: vec![GeminiPart { text }],
                }
            })
            .collect()
    }
}

#[async_trait]
impl LLMServerClient for GeminiLLMClient {
    async fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<CreateMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            "Gemini create_message called with {} messages",
            request.messages.len()
        );

        // Convert MCP messages to Gemini format
        let contents = Self::to_gemini_contents(&request.messages);

        // Extract model name from MCP 2025-06-18 model preferences structure
        let model = request
            .model_preferences
            .as_ref()
            .and_then(|prefs| prefs.hints.as_ref())
            .and_then(|hints| hints.first())
            .and_then(|hint| hint.name.as_ref())
            .cloned()
            .unwrap_or_else(|| self.default_model.clone());

        debug!("Using Gemini model: {}", model);

        // Build generation config (max_tokens always required per MCP 2025-06-18)
        let generation_config = Some(GenerationConfig {
            temperature: request.temperature,
            max_output_tokens: Some(request.max_tokens), // Always present, wrap in Option for Gemini API
            stop_sequences: request.stop_sequences,
        });

        // Build Gemini request
        let gemini_request = GeminiRequest {
            contents,
            generation_config,
        };

        // Call Gemini API
        debug!("Sending request to Gemini API...");
        let endpoint = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url, model, self.api_key
        );

        let http_response = self
            .client
            .post(&endpoint)
            .header("content-type", "application/json")
            .json(&gemini_request)
            .send()
            .await?;

        // Check for HTTP errors
        let status = http_response.status();
        if !status.is_success() {
            let error_body = http_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Gemini API error ({}): {}", status, error_body);

            return Err(match status.as_u16() {
                401 => LLMProviderError::AuthError(format!("Invalid API key: {}", error_body)),
                429 => LLMProviderError::RateLimitError(error_body),
                404 => LLMProviderError::ModelNotFound(error_body),
                _ => LLMProviderError::ApiError(format!("{}: {}", status, error_body)),
            }
            .into());
        }

        // Parse response
        let response: GeminiResponse = http_response.json().await.map_err(|e| {
            error!("Failed to parse Gemini response: {}", e);
            LLMProviderError::InvalidResponse(e.to_string())
        })?;

        debug!("Received response from Gemini API");

        // Extract first candidate
        let candidate = response.candidates.first().ok_or_else(|| {
            error!("Gemini response contained no candidates");
            LLMProviderError::InvalidResponse("No candidates in response".to_string())
        })?;

        // Extract text from parts
        let text = candidate
            .content
            .parts
            .first()
            .ok_or_else(|| {
                error!("Gemini candidate has no parts");
                LLMProviderError::InvalidResponse("No parts in candidate".to_string())
            })?
            .text
            .clone();

        // Map Gemini finish_reason to MCP StopReason enum
        let stop_reason = if let Some(finish_reason) = &candidate.finish_reason {
            debug!("Gemini finish_reason: {}", finish_reason);
            // Gemini uses values like "STOP", "MAX_TOKENS", etc.
            match finish_reason.as_str() {
                "STOP" => turbomcp_protocol::types::StopReason::EndTurn,
                "MAX_TOKENS" => turbomcp_protocol::types::StopReason::MaxTokens,
                "SAFETY" | "RECITATION" => turbomcp_protocol::types::StopReason::ContentFilter,
                _ => {
                    warn!("Unknown Gemini finish_reason: {}", finish_reason);
                    turbomcp_protocol::types::StopReason::EndTurn
                }
            }
        } else {
            warn!("No finish_reason in Gemini response, defaulting to EndTurn");
            turbomcp_protocol::types::StopReason::EndTurn
        };

        info!(
            "Gemini request completed - model: {}, stop_reason: {:?}",
            model, stop_reason
        );

        // Create MCP result
        Ok(MessageConverter::create_text_result(
            text,
            model,
            stop_reason,
        ))
    }

    async fn get_server_info(
        &self,
    ) -> Result<ServerInfo, Box<dyn std::error::Error + Send + Sync>> {
        // Model list updated January 2026 - Gemini 2.5 and 3.0 families
        Ok(ServerInfo {
            name: "Google Gemini".to_string(),
            models: vec![
                // Gemini 3 family (GA in AI Studio)
                "gemini-3-pro".to_string(),
                "gemini-3-flash".to_string(),
                // Gemini 2.5 family (Production recommended)
                "gemini-2.5-pro".to_string(),       // High-capability, 1M context
                "gemini-2.5-flash".to_string(),     // Fast, controllable thinking budgets
                "gemini-2.5-flash-lite".to_string(), // Cost-optimized for scale
                // Gemini Live API
                "gemini-2.5-flash-live".to_string(), // Real-time bidirectional streaming
                // Legacy aliases (may point to preview)
                "gemini-pro".to_string(),
            ],
            capabilities: vec![
                "vision".to_string(),
                "function_calling".to_string(),
                "1m_context".to_string(),           // Gemini 2.5 Pro
                "adaptive_thinking".to_string(),    // Controllable thinking budgets
                "live_streaming".to_string(),       // Gemini Live API
                "multimodal".to_string(),
            ],
        })
    }
}
