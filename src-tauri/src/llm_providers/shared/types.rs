use serde::{Deserialize, Serialize};
use turbomcp_protocol::types::{
    CreateMessageResult, Role, SamplingContent, SamplingContentBlock, SamplingMessage, StopReason,
    TextContent,
};

/// Token usage information
/// Note: May appear unused due to serde deriving Deserialize, but used in response parsing
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub thinking_tokens: Option<u64>,
}

/// A plain OpenAI-compatible chat message for use with the reqwest-based providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatMessage {
    pub role: String,
    pub content: String,
}

/// Message conversion utilities for MCP ↔ Provider formats
pub struct MessageConverter;

impl MessageConverter {
    /// Convert MCP SamplingMessage to OpenAI-compatible chat messages.
    ///
    /// Returns plain structs that can be serialised directly by the reqwest-based
    /// OpenAI and OpenAI-compatible providers without requiring async-openai types.
    pub fn to_openai_messages(messages: &[SamplingMessage]) -> Vec<OpenAIChatMessage> {
        messages
            .iter()
            .map(|msg| {
                let content = Self::extract_text_from_content(&msg.content);
                // Note: MCP Role enum only has User and Assistant (no System).
                // System-like messages should be prepended to user messages by the caller.
                let role = match msg.role {
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                };
                OpenAIChatMessage { role, content }
            })
            .collect()
    }

    /// Convert MCP SamplingMessage to Anthropic format
    pub fn to_anthropic_messages(messages: &[SamplingMessage]) -> Vec<AnthropicMessage> {
        messages
            .iter()
            .map(|msg| AnthropicMessage {
                role: match msg.role {
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                },
                content: Self::extract_text_from_content(&msg.content),
            })
            .collect()
    }

    /// Extract text from MCP sampling content blocks.
    fn extract_text_from_content(content: &SamplingContentBlock) -> String {
        content
            .to_vec()
            .into_iter()
            .map(|content| match content {
                SamplingContent::Text(text_content) => text_content.text.clone(),
                SamplingContent::Image(image_content) => {
                    // For image content, return description or placeholder
                    format!("[Image: {}]", image_content.mime_type)
                }
                SamplingContent::Audio(audio_content) => {
                    format!("[Audio: {}]", audio_content.mime_type)
                }
                SamplingContent::ToolUse(tool_use) => {
                    format!("[Tool call: {}]", tool_use.name)
                }
                SamplingContent::ToolResult(tool_result) => {
                    format!("[Tool result: {}]", tool_result.tool_use_id)
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Create MCP CreateMessageResult from text response
    pub fn create_text_result(
        text: String,
        model: String,
        stop_reason: StopReason,
    ) -> CreateMessageResult {
        CreateMessageResult {
            role: Role::Assistant,
            content: SamplingContent::Text(TextContent {
                text,
                annotations: None,
                meta: None,
            })
            .into(),
            model,
            stop_reason: Some(stop_reason.into()),
            meta: None,
        }
    }
}

/// Anthropic message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

/// Anthropic request format
#[derive(Debug, Clone, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Anthropic response format
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicResponse {
    pub content: Vec<AnthropicContent>,
    pub model: String,
    pub stop_reason: String,
    pub usage: AnthropicUsage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicContent {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

// Note: Gemini message types are defined locally in gemini.rs where they're used
