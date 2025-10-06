//! Sampling and LLM Configuration Commands
//!
//! Tauri commands for LLM sampling configuration and HITL operations.
//! - Configure LLM providers and API keys
//! - Manage sampling modes (auto, HITL, disabled)
//! - Handle sampling requests and approvals
//! - Test sampling configurations

use crate::hitl_sampling::{PendingSamplingRequest, SamplingMode, SamplingResult};
use crate::AppState;
use serde_json::Value;
use tauri::State;
use turbomcp_protocol::CreateMessageRequest;
use uuid::Uuid;

/// Get current LLM configuration
#[tauri::command]
pub async fn get_llm_config(app_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = app_state.llm_config.get_config().await;
    serde_json::to_value(config).map_err(|e| format!("Serialization error: {}", e))
}

/// Get LLM provider statuses
#[tauri::command]
pub async fn get_llm_provider_statuses(
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let statuses = app_state.llm_config.get_provider_statuses().await;
    serde_json::to_value(statuses).map_err(|e| format!("Serialization error: {}", e))
}

/// Set API key for a provider
#[tauri::command]
pub async fn set_llm_api_key(
    provider_id: String,
    api_key: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::types::SetAPIKeyRequest;

    let request = SetAPIKeyRequest {
        provider_id: provider_id.clone(),
        api_key,
    };

    app_state
        .llm_config
        .set_api_key(request)
        .await
        .map_err(|e| format!("Failed to set API key: {}", e))?;

    // Update sampling handler with new configuration
    if let Err(e) = app_state
        .mcp_manager
        .update_sampling_handler(&app_state.llm_config)
        .await
    {
        tracing::warn!("Failed to update sampling handler: {}", e);
    }

    Ok(())
}

/// Remove API key for a provider
#[tauri::command]
pub async fn remove_llm_api_key(
    provider_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    app_state
        .llm_config
        .remove_api_key(&provider_id)
        .await
        .map_err(|e| format!("Failed to remove API key: {}", e))?;

    // Update sampling handler
    if let Err(e) = app_state
        .mcp_manager
        .update_sampling_handler(&app_state.llm_config)
        .await
    {
        tracing::warn!("Failed to update sampling handler: {}", e);
    }

    Ok(())
}

/// Set the active LLM provider
#[tauri::command]
pub async fn set_active_llm_provider(
    provider_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    app_state
        .llm_config
        .set_active_provider(provider_id)
        .await
        .map_err(|e| format!("Failed to set active provider: {}", e))?;

    // Update sampling handler
    if let Err(e) = app_state
        .mcp_manager
        .update_sampling_handler(&app_state.llm_config)
        .await
    {
        tracing::warn!("Failed to update sampling handler: {}", e);
    }

    Ok(())
}

/// Update LLM provider configuration
#[tauri::command]
pub async fn update_llm_provider_config(
    config: serde_json::Value,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::types::UpdateLLMConfigRequest;

    let request: UpdateLLMConfigRequest =
        serde_json::from_value(config).map_err(|e| format!("Invalid configuration: {}", e))?;

    app_state
        .llm_config
        .update_provider_config(request)
        .await
        .map_err(|e| format!("Failed to update provider config: {}", e))?;

    // Update sampling handler
    if let Err(e) = app_state
        .mcp_manager
        .update_sampling_handler(&app_state.llm_config)
        .await
    {
        tracing::warn!("Failed to update sampling handler: {}", e);
    }

    Ok(())
}

/// Check if sampling is available
#[tauri::command]
pub async fn is_sampling_available(app_state: State<'_, AppState>) -> Result<bool, String> {
    Ok(app_state
        .mcp_manager
        .is_sampling_available(&app_state.llm_config)
        .await)
}

/// Validate LLM configuration
#[tauri::command]
pub async fn validate_llm_config(app_state: State<'_, AppState>) -> Result<Vec<String>, String> {
    app_state
        .llm_config
        .validate_configuration()
        .await
        .map_err(|e| format!("Validation failed: {}", e))
}

/// Get the current HITL sampling mode
#[tauri::command]
pub async fn get_hitl_sampling_mode(
    app_state: State<'_, AppState>,
) -> Result<SamplingMode, String> {
    let mode = app_state.hitl_sampling.get_mode().await;
    Ok(mode)
}

/// Set the HITL sampling mode
#[tauri::command]
pub async fn set_hitl_sampling_mode(
    mode: SamplingMode,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    app_state
        .hitl_sampling
        .set_mode(mode)
        .await
        .map_err(|e| e.to_string())
}

/// Get pending sampling requests awaiting human approval
#[tauri::command]
pub async fn get_pending_sampling_requests(
    app_state: State<'_, AppState>,
) -> Result<Vec<PendingSamplingRequest>, String> {
    let pending = app_state.hitl_sampling.get_pending_requests();
    Ok(pending)
}

/// Get completed sampling results for analysis
#[tauri::command]
pub async fn get_completed_sampling_requests(
    app_state: State<'_, AppState>,
) -> Result<Vec<SamplingResult>, String> {
    let completed = app_state.hitl_sampling.get_completed_requests();
    Ok(completed)
}

/// Approve a pending sampling request
#[tauri::command]
pub async fn approve_sampling_request(
    request_id: String,
    approved_by: String,
    modified_request: Option<CreateMessageRequest>,
    app_state: State<'_, AppState>,
) -> Result<SamplingResult, String> {
    app_state
        .hitl_sampling
        .approve_request(&request_id, approved_by, modified_request)
        .await
        .map_err(|e| e.to_string())
}

/// Reject a pending sampling request
#[tauri::command]
pub async fn reject_sampling_request(
    request_id: String,
    reason: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    app_state
        .hitl_sampling
        .reject_request(&request_id, reason)
        .await
        .map_err(|e| e.to_string())
}

/// Process a sampling request through the HITL system
#[tauri::command]
pub async fn process_hitl_sampling_request(
    server_id: String,
    server_name: String,
    request: CreateMessageRequest,
    app_state: State<'_, AppState>,
) -> Result<SamplingResult, String> {
    app_state
        .hitl_sampling
        .process_sampling_request(server_id, server_name, request)
        .await
        .map_err(|e| e.to_string())
}

/// Test sampling request for debugging (doesn't actually send to LLM)
#[tauri::command]
pub async fn test_sampling_request(
    server_id: String,
    server_name: String,
    messages: Vec<serde_json::Value>,
    _app_state: State<'_, AppState>,
) -> Result<Value, String> {
    // Convert JSON messages to MCP protocol format
    use turbomcp_protocol::types::{Content, IncludeContext, Role, SamplingMessage, TextContent};

    let sampling_messages: Result<Vec<SamplingMessage>, String> = messages
        .into_iter()
        .map(|msg| {
            let role = match msg.get("role").and_then(|r| r.as_str()) {
                Some("user") => Role::User,
                Some("assistant") => Role::Assistant,
                _ => return Err("Invalid role in message".to_string()),
            };

            let content = match msg.get("content").and_then(|c| c.as_str()) {
                Some(text) => Content::Text(TextContent {
                    text: text.to_string(),
                    annotations: None,
                    meta: None,
                }),
                None => return Err("Missing content in message".to_string()),
            };

            Ok(SamplingMessage {
                role,
                content,
                metadata: None,
            })
        })
        .collect();

    let sampling_messages = sampling_messages?;

    let request = CreateMessageRequest {
        messages: sampling_messages,
        model_preferences: None,
        system_prompt: None,
        include_context: Some(IncludeContext::ThisServer),
        max_tokens: Some(1000),
        temperature: Some(0.7),
        stop_sequences: None,
        _meta: None,
    };

    // Get conversation analysis without actually processing
    let analysis = serde_json::json!({
        "request_id": uuid::Uuid::new_v4().to_string(),
        "server_id": server_id,
        "server_name": server_name,
        "message_count": request.messages.len(),
        "estimated_tokens": request.messages.len() * 50, // Rough estimate
        "estimated_cost": 0.001, // Rough estimate
        "conversation_context": {
            "thread_length": request.messages.len(),
            "has_system_prompt": request.system_prompt.is_some(),
            "model_preferences": request.model_preferences,
            "parameters": {
                "max_tokens": request.max_tokens,
                "temperature": request.temperature,
                "temperature": request.temperature,
                "include_context": request.include_context
            }
        },
        "status": "test_mode"
    });

    Ok(analysis)
}

/// Create a sampling request (client-initiated request to a capable MCP server)
#[tauri::command]
pub async fn create_sampling_request(
    server_id: String,
    messages: Vec<serde_json::Value>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Create sampling request using runtime LLM configuration
    let result = app_state
        .mcp_manager
        .create_sampling_request_with_config(
            uuid,
            messages,
            max_tokens,
            temperature,
            &app_state.llm_config,
        )
        .await
        .map_err(|e| format!("Failed to create sampling request: {}", e))?;

    Ok(result)
}
