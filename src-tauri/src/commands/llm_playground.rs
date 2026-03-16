//! LLM Playground Commands
//!
//! Tauri commands for interacting with LLM providers directly from the UI.
//! Exposes the existing LLM infrastructure (LLMConfigManager) to the frontend
//! for building a world-class chat playground experience.
//!
//! Note: API key management commands (set_llm_api_key, remove_llm_api_key, set_active_llm_provider)
//! are in sampling.rs since they're used by both sampling and playground features.

use crate::types::LLMProviderStatus;
use crate::AppState;
use tauri::State;
use turbomcp_protocol::types::{CreateMessageRequest, CreateMessageResult};

/// Get list of all LLM providers with their status
///
/// Returns provider information including:
/// - ID, display name, and type
/// - Enabled/disabled status
/// - Whether API key is configured
/// - Default model
/// - Base URL (for local providers)
#[tauri::command]
pub async fn list_llm_providers(
    app_state: State<'_, AppState>,
) -> Result<Vec<LLMProviderStatus>, String> {
    app_state
        .llm_config
        .get_provider_statuses()
        .await
        .into_iter()
        .map(Ok)
        .collect()
}

/// Send a message to an LLM provider
///
/// # Arguments
/// * `request` - CreateMessageRequest with messages, model, max_tokens, etc.
/// * `provider_id` - Optional provider ID (if None, uses active provider)
///
/// # Returns
/// CreateMessageResult with assistant's response and usage statistics
#[tauri::command]
pub async fn send_llm_message(
    request: CreateMessageRequest,
    provider_id: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<CreateMessageResult, String> {
    app_state
        .llm_config
        .invoke_llm_directly(request, provider_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get the current active provider ID
///
/// Returns the ID of the currently active provider, if any.
#[tauri::command]
pub async fn get_active_llm_provider(
    app_state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let config = app_state.llm_config.get_config().await;
    Ok(config.active_provider)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_llm_playground_commands_compile() {
        // Smoke test - ensure module compiles
    }

    // TODO: Add integration tests with mock LLM providers
    // - Test list_llm_providers returns expected providers
    // - Test send_llm_message with mock responses
    // - Test API key storage/retrieval
    // - Test provider switching
}
