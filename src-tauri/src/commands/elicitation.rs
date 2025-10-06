//! Elicitation Commands
//!
//! Tauri commands for MCP elicitation operations.
//! - Send elicitation responses
//! - Get pending elicitation requests

use crate::AppState;
use tauri::State;

/// Send an elicitation response (respond to server-initiated user input request)
#[tauri::command]
pub async fn send_elicitation_response(
    _server_id: String,
    request_id: String,
    response_data: serde_json::Value,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // Parse the response data into ElicitationResponse
    let action_str = response_data
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'action' field".to_string())?;

    let action = match action_str {
        "accept" => turbomcp_client::handlers::ElicitationAction::Accept,
        "decline" => turbomcp_client::handlers::ElicitationAction::Decline,
        "cancel" => turbomcp_client::handlers::ElicitationAction::Cancel,
        _ => return Err(format!("Invalid action: {}", action_str)),
    };

    let content = response_data.get("content").cloned();

    let response = turbomcp_client::handlers::ElicitationResponse { action, content };

    // Submit response through the elicitation handler
    app_state
        .mcp_manager
        .submit_elicitation_response(request_id, response)
        .map_err(|e| format!("Failed to submit elicitation response: {}", e))?;

    Ok(serde_json::json!({"status": "success"}))
}

/// Get pending elicitation requests for a server
/// DEPRECATED: Elicitation is now event-driven via 'elicitation_requested' events
/// This command returns empty array for backward compatibility
#[tauri::command]
pub async fn get_elicitation_requests(
    _server_id: String,
    _app_state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    // Elicitation is now event-driven - listen to 'elicitation_requested' events instead
    Ok(Vec::new())
}
