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

    // Use factory methods based on action
    let response = match action_str {
        "accept" => {
            // Extract content and convert to HashMap
            let content_map: std::collections::HashMap<String, serde_json::Value> = response_data
                .get("content")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            turbomcp_client::handlers::ElicitationResponse::accept(content_map)
        }
        "decline" => turbomcp_client::handlers::ElicitationResponse::decline(),
        "cancel" => turbomcp_client::handlers::ElicitationResponse::cancel(),
        _ => return Err(format!("Invalid action: {}", action_str)),
    };

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
