//! Stop Reason Mapping for LLM Provider Integration
//!
//! Maps provider-specific stop reason strings to MCP-compliant StopReason enum values.
//!
//! ## Problem
//!
//! - MCP 2025-06-18 spec defines `stopReason` as `type: "string"` (schema.json:540-542)
//! - TurboMCP 2.0.1 uses enum for type safety: `StopReason` with 5 variants
//! - Each LLM provider returns different stop reason formats
//!
//! ## Solution
//!
//! This module provides bidirectional mapping between provider formats and MCP enum values,
//! with comprehensive logging for debugging and extensibility for future providers.

use tracing::{debug, warn};
use turbomcp_protocol::types::StopReason;

/// LLM provider identifier for context-aware mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
    Ollama,
    LMStudio,
    Custom(&'static str),
}

impl LLMProvider {
    pub fn name(&self) -> &str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Ollama => "ollama",
            Self::LMStudio => "lmstudio",
            Self::Custom(name) => name,
        }
    }
}

/// Map provider-specific stop reason to MCP StopReason enum
///
/// # Arguments
///
/// * `provider` - The LLM provider that generated the response
/// * `reason` - The provider-specific stop reason string
///
/// # Returns
///
/// MCP-compliant `StopReason` enum value, with logging for unmapped cases
///
/// # Examples
///
/// ```
/// use turbomcp_protocol::types::StopReason;
///
/// // OpenAI format
/// let reason = map_stop_reason(LLMProvider::OpenAI, "stop");
/// assert_eq!(reason, StopReason::EndTurn);
///
/// // Anthropic format
/// let reason = map_stop_reason(LLMProvider::Anthropic, "end_turn");
/// assert_eq!(reason, StopReason::EndTurn);
///
/// // Unknown value (with warning logged)
/// let reason = map_stop_reason(LLMProvider::OpenAI, "unknown_reason");
/// assert_eq!(reason, StopReason::EndTurn); // Default fallback
/// ```
pub fn map_stop_reason(provider: LLMProvider, reason: &str) -> StopReason {
    debug!(
        provider = provider.name(),
        reason = reason,
        "Mapping provider stop reason to MCP StopReason"
    );

    let result = match provider {
        LLMProvider::OpenAI => map_openai_stop_reason(reason),
        LLMProvider::Anthropic => map_anthropic_stop_reason(reason),
        LLMProvider::Ollama => map_ollama_stop_reason(reason),
        LLMProvider::LMStudio => map_lmstudio_stop_reason(reason),
        LLMProvider::Custom(name) => {
            warn!(
                provider = name,
                reason = reason,
                "Unknown provider, using generic mapping"
            );
            map_generic_stop_reason(reason)
        }
    };

    debug!(
        provider = provider.name(),
        input = reason,
        output = ?result,
        "Stop reason mapped"
    );

    result
}

/// Map OpenAI finish_reason values to MCP StopReason
///
/// OpenAI API Reference:
/// https://platform.openai.com/docs/api-reference/chat/object
///
/// Possible values:
/// - "stop" - Natural completion (model stopped naturally)
/// - "length" - Max tokens reached
/// - "content_filter" - Content policy triggered
/// - "tool_calls" - Tool/function calling triggered (GPT-4+)
/// - "function_call" - Deprecated function calling
fn map_openai_stop_reason(reason: &str) -> StopReason {
    match reason {
        // Natural completion
        "stop" => StopReason::EndTurn,

        // Token limits
        "length" => StopReason::MaxTokens,

        // Content moderation
        "content_filter" => StopReason::ContentFilter,

        // Tool/function calling
        "tool_calls" | "function_call" => StopReason::ToolUse,

        // Unknown/undocumented values
        unknown => {
            warn!(
                provider = "openai",
                reason = unknown,
                mapped_to = ?StopReason::EndTurn,
                "Unknown OpenAI finish_reason, using default mapping"
            );
            StopReason::EndTurn
        }
    }
}

/// Map Anthropic stop_reason values to MCP StopReason
///
/// Anthropic API Reference:
/// https://docs.anthropic.com/en/api/messages
///
/// Possible values:
/// - "end_turn" - Natural completion
/// - "max_tokens" - Token limit reached
/// - "stop_sequence" - Stop sequence encountered
/// - "tool_use" - Tool invocation
fn map_anthropic_stop_reason(reason: &str) -> StopReason {
    match reason {
        // Natural completion
        "end_turn" => StopReason::EndTurn,

        // Token limits
        "max_tokens" => StopReason::MaxTokens,

        // Stop sequences
        "stop_sequence" => StopReason::StopSequence,

        // Tool usage
        "tool_use" => StopReason::ToolUse,

        // Unknown/undocumented values
        unknown => {
            warn!(
                provider = "anthropic",
                reason = unknown,
                mapped_to = ?StopReason::EndTurn,
                "Unknown Anthropic stop_reason, using default mapping"
            );
            StopReason::EndTurn
        }
    }
}

/// Map Ollama stop_reason values to MCP StopReason
///
/// Ollama typically follows OpenAI-compatible format
fn map_ollama_stop_reason(reason: &str) -> StopReason {
    match reason {
        "stop" | "end" => StopReason::EndTurn,
        "length" | "max_tokens" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::StopSequence,
        unknown => {
            warn!(
                provider = "ollama",
                reason = unknown,
                mapped_to = ?StopReason::EndTurn,
                "Unknown Ollama stop_reason, using default mapping"
            );
            StopReason::EndTurn
        }
    }
}

/// Map LM Studio stop_reason values to MCP StopReason
///
/// LM Studio follows OpenAI-compatible format
fn map_lmstudio_stop_reason(reason: &str) -> StopReason {
    // LM Studio uses OpenAI format
    map_openai_stop_reason(reason)
}

/// Generic mapping for unknown providers
///
/// Uses heuristic matching of common patterns across providers
fn map_generic_stop_reason(reason: &str) -> StopReason {
    let reason_lower = reason.to_lowercase();

    if reason_lower.contains("stop") || reason_lower.contains("end") || reason_lower.contains("complete") {
        StopReason::EndTurn
    } else if reason_lower.contains("length") || reason_lower.contains("token") || reason_lower.contains("max") {
        StopReason::MaxTokens
    } else if reason_lower.contains("sequence") {
        StopReason::StopSequence
    } else if reason_lower.contains("filter") || reason_lower.contains("content") || reason_lower.contains("safety") {
        StopReason::ContentFilter
    } else if reason_lower.contains("tool") || reason_lower.contains("function") {
        StopReason::ToolUse
    } else {
        warn!(
            reason = reason,
            mapped_to = ?StopReason::EndTurn,
            "Could not match stop reason pattern, using default"
        );
        StopReason::EndTurn
    }
}

/// Reverse mapping: Convert MCP StopReason to provider-specific string
///
/// Useful for debugging and when forwarding to other services
///
/// # Arguments
///
/// * `provider` - Target provider format
/// * `stop_reason` - MCP StopReason enum value
///
/// # Returns
///
/// Provider-specific stop reason string
pub fn to_provider_format(provider: LLMProvider, stop_reason: StopReason) -> &'static str {
    match provider {
        LLMProvider::OpenAI | LLMProvider::LMStudio => match stop_reason {
            StopReason::EndTurn => "stop",
            StopReason::MaxTokens => "length",
            StopReason::StopSequence => "stop", // OpenAI doesn't distinguish
            StopReason::ContentFilter => "content_filter",
            StopReason::ToolUse => "tool_calls",
        },
        LLMProvider::Anthropic => match stop_reason {
            StopReason::EndTurn => "end_turn",
            StopReason::MaxTokens => "max_tokens",
            StopReason::StopSequence => "stop_sequence",
            StopReason::ContentFilter => "end_turn", // Anthropic doesn't have this
            StopReason::ToolUse => "tool_use",
        },
        LLMProvider::Ollama => match stop_reason {
            StopReason::EndTurn => "stop",
            StopReason::MaxTokens => "length",
            StopReason::StopSequence => "stop_sequence",
            StopReason::ContentFilter => "stop",
            StopReason::ToolUse => "stop",
        },
        LLMProvider::Custom(_) => {
            // Default to generic lowercase names
            match stop_reason {
                StopReason::EndTurn => "end_turn",
                StopReason::MaxTokens => "max_tokens",
                StopReason::StopSequence => "stop_sequence",
                StopReason::ContentFilter => "content_filter",
                StopReason::ToolUse => "tool_use",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_stop_reasons() {
        assert_eq!(
            map_stop_reason(LLMProvider::OpenAI, "stop"),
            StopReason::EndTurn
        );
        assert_eq!(
            map_stop_reason(LLMProvider::OpenAI, "length"),
            StopReason::MaxTokens
        );
        assert_eq!(
            map_stop_reason(LLMProvider::OpenAI, "content_filter"),
            StopReason::ContentFilter
        );
        assert_eq!(
            map_stop_reason(LLMProvider::OpenAI, "tool_calls"),
            StopReason::ToolUse
        );
        assert_eq!(
            map_stop_reason(LLMProvider::OpenAI, "function_call"),
            StopReason::ToolUse
        );
    }

    #[test]
    fn test_anthropic_stop_reasons() {
        assert_eq!(
            map_stop_reason(LLMProvider::Anthropic, "end_turn"),
            StopReason::EndTurn
        );
        assert_eq!(
            map_stop_reason(LLMProvider::Anthropic, "max_tokens"),
            StopReason::MaxTokens
        );
        assert_eq!(
            map_stop_reason(LLMProvider::Anthropic, "stop_sequence"),
            StopReason::StopSequence
        );
        assert_eq!(
            map_stop_reason(LLMProvider::Anthropic, "tool_use"),
            StopReason::ToolUse
        );
    }

    #[test]
    fn test_unknown_values_default_to_end_turn() {
        assert_eq!(
            map_stop_reason(LLMProvider::OpenAI, "unknown_value"),
            StopReason::EndTurn
        );
        assert_eq!(
            map_stop_reason(LLMProvider::Anthropic, "weird_reason"),
            StopReason::EndTurn
        );
    }

    #[test]
    fn test_generic_mapping_heuristics() {
        assert_eq!(
            map_generic_stop_reason("completed"),
            StopReason::EndTurn
        );
        assert_eq!(
            map_generic_stop_reason("max_length_reached"),
            StopReason::MaxTokens
        );
        assert_eq!(
            map_generic_stop_reason("stop_word_found"),
            StopReason::StopSequence
        );
        assert_eq!(
            map_generic_stop_reason("safety_filter"),
            StopReason::ContentFilter
        );
        assert_eq!(
            map_generic_stop_reason("function_called"),
            StopReason::ToolUse
        );
    }

    #[test]
    fn test_reverse_mapping_openai() {
        assert_eq!(
            to_provider_format(LLMProvider::OpenAI, StopReason::EndTurn),
            "stop"
        );
        assert_eq!(
            to_provider_format(LLMProvider::OpenAI, StopReason::MaxTokens),
            "length"
        );
        assert_eq!(
            to_provider_format(LLMProvider::OpenAI, StopReason::ToolUse),
            "tool_calls"
        );
    }

    #[test]
    fn test_reverse_mapping_anthropic() {
        assert_eq!(
            to_provider_format(LLMProvider::Anthropic, StopReason::EndTurn),
            "end_turn"
        );
        assert_eq!(
            to_provider_format(LLMProvider::Anthropic, StopReason::MaxTokens),
            "max_tokens"
        );
        assert_eq!(
            to_provider_format(LLMProvider::Anthropic, StopReason::ToolUse),
            "tool_use"
        );
    }

    #[test]
    fn test_bidirectional_consistency() {
        // OpenAI round-trip
        let original = "stop";
        let mapped = map_stop_reason(LLMProvider::OpenAI, original);
        let back = to_provider_format(LLMProvider::OpenAI, mapped);
        assert_eq!(back, original);

        // Anthropic round-trip
        let original = "end_turn";
        let mapped = map_stop_reason(LLMProvider::Anthropic, original);
        let back = to_provider_format(LLMProvider::Anthropic, mapped);
        assert_eq!(back, original);
    }
}
