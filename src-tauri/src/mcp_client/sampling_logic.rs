//! Sampling Logic Module
//!
//! Handles MCP sampling requests with LLM provider integration.
//! This module provides stateless operations for:
//! - Converting JSON messages to MCP SamplingMessage format
//! - Processing sampling requests through configured LLM handlers
//! - Formatting responses back to JSON for frontend consumption

use crate::error::{McpResult, McpStudioError};
use crate::mcp_client::connection::ManagedConnection;
use std::sync::Arc;
use turbomcp_client::sampling::SamplingHandler; // Import trait for method availability
use turbomcp_protocol::types::{Content, CreateMessageRequest, Role, SamplingMessage, TextContent};

/// Sampling Logic Operations
///
/// Provides stateless operations for MCP sampling requests.
pub struct SamplingLogic;

impl SamplingLogic {
    /// Create and process sampling request with runtime LLM configuration
    ///
    /// This method:
    /// 1. Converts JSON messages to MCP protocol format
    /// 2. Routes through configured LLM handler (OpenAI, Anthropic, etc.)
    /// 3. Returns formatted response with completion or fallback message
    pub async fn create_sampling_request_with_config(
        connection: &Arc<ManagedConnection>,
        messages: Vec<serde_json::Value>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        llm_config: &crate::llm_config::LLMConfigManager,
    ) -> McpResult<serde_json::Value> {
        // Get the active sampling handler from runtime configuration
        if let Some(handler) = llm_config.get_active_sampling_handler().await {
            tracing::info!(
                "Processing sampling request for server: {} with {} messages",
                connection.config.name,
                messages.len()
            );

            // Convert JSON messages to proper MCP protocol format
            let sampling_messages = Self::convert_json_to_sampling_messages(messages)?;

            let request = CreateMessageRequest {
                messages: sampling_messages,
                max_tokens: max_tokens.unwrap_or(1000), // MCP 2025-06-18: Required field
                temperature: temperature.map(|t| t as f64),
                system_prompt: None,
                stop_sequences: None,
                model_preferences: None,
                include_context: None,
                _meta: None, // TurboMCP 2.0: removed metadata, use _meta
            };

            // Process the sampling request through TurboMCP
            match handler.handle_create_message(request).await {
                Ok(result) => {
                    tracing::info!(
                        "Sampling request completed successfully for server: {}",
                        connection.config.name
                    );

                    // Convert result back to JSON format
                    let response = serde_json::json!({
                        "status": "completed",
                        "role": match result.role {
                            Role::User => "user",
                            Role::Assistant => "assistant",
                        },
                        "content": match result.content {
                            Content::Text(text) => text.text,
                            _ => "Unsupported content type".to_string(),
                        },
                        "model": result.model,
                        "stop_reason": result.stop_reason,
                        "processed_messages": 1
                    });

                    Ok(response)
                }
                Err(e) => {
                    tracing::error!(
                        "Sampling request failed for server {}: {}",
                        connection.config.name,
                        e
                    );

                    let response = serde_json::json!({
                        "status": "error",
                        "message": format!("Sampling failed: {}", e),
                        "server": connection.config.name
                    });

                    Ok(response)
                }
            }
        } else {
            // Fallback response when no LLM provider is configured
            tracing::warn!(
                "No LLM provider configured - using fallback response for server: {}",
                connection.config.name
            );

            let response = serde_json::json!({
                "status": "no_provider",
                "message": "No LLM provider configured. Set OPENAI_API_KEY or ANTHROPIC_API_KEY environment variable.",
                "received_messages": messages.len(),
                "max_tokens": max_tokens,
                "temperature": temperature,
                "note": "Configure an LLM provider to enable real sampling functionality"
            });

            Ok(response)
        }
    }

    /// Convert JSON messages to MCP SamplingMessage format
    ///
    /// Handles role parsing and content extraction from frontend JSON format.
    fn convert_json_to_sampling_messages(
        messages: Vec<serde_json::Value>,
    ) -> McpResult<Vec<SamplingMessage>> {
        let sampling_messages: Result<Vec<SamplingMessage>, _> = messages
            .into_iter()
            .map(|msg| {
                let role = match msg.get("role").and_then(|r| r.as_str()) {
                    Some("user") => Role::User,
                    Some("assistant") => Role::Assistant,
                    _ => Role::User, // Default to user
                };

                let content = msg
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                Ok(SamplingMessage {
                    role,
                    content: Content::Text(TextContent {
                        text: content,
                        annotations: None,
                        meta: None,
                    }),
                    metadata: None, // TurboMCP 2.0: added optional metadata field
                })
            })
            .collect();

        sampling_messages.map_err(|e: &str| {
            McpStudioError::ConfigError(format!("Failed to parse messages: {}", e))
        })
    }
}
