//! LLM Provider Integrations
//!
//! This module provides integrations with various LLM providers (OpenAI, Anthropic, Ollama, etc.)
//! and utilities for converting between provider-specific formats and MCP protocol types.
//!
//! ## Architecture
//!
//! - **stop_reason_mapping**: Bidirectional mapping between provider formats and MCP StopReason enum
//! - **openai**: OpenAI API client (Phase 2)
//! - **anthropic**: Anthropic API client (Phase 2)
//! - **ollama**: Ollama local model client (Phase 2)
//! - **lmstudio**: LM Studio local model client (Phase 2)

pub mod shared;
pub mod stop_reason_mapping;

// LLM Provider Clients - Phase 2 Implementation
pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod openai_compatible; // Used by Ollama, LMStudio, and other OpenAI-compatible providers

// Re-export commonly used types (only what's actually used)
// Note: map_stop_reason, to_provider_format, and LLMProvider are used internally
// but don't need to be re-exported at the module level

// Re-export client types
pub use anthropic::AnthropicLLMClient;
pub use gemini::GeminiLLMClient;
pub use openai::OpenAILLMClient;
pub use openai_compatible::OpenAICompatibleClient;
