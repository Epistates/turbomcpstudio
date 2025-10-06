//! MCP Tools Commands
//!
//! Tauri commands for MCP tool operations.
//! - Call tools with parameters
//! - List available tools
//! - Get completion suggestions

use crate::types::ToolDefinition;
use crate::AppState;
use serde_json;
use tauri::State;
use uuid::Uuid;

/// Call a tool on a connected MCP server
#[tauri::command]
pub async fn call_tool(
    server_id: String,
    tool_name: String,
    parameters: serde_json::Value,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Create request data for message history
    let request_data = serde_json::json!({
        "method": "tools/call",
        "tool": tool_name,
        "arguments": parameters
    });

    // Track request with message history
    let db_lock = app_state.database.read().await;
    let request_tracking = if let Some(database) = db_lock.as_ref() {
        app_state
            .mcp_manager
            .track_request_with_history(uuid, "tools/call", &request_data, database)
            .await
            .ok()
    } else {
        // Fallback to basic tracking if database not ready
        app_state
            .mcp_manager
            .track_request_start(uuid, "tools/call", request_data.to_string().len() as u64)
            .await
            .ok()
    };

    // Call the tool using the actual manager with TurboMCP integration
    let result = app_state
        .mcp_manager
        .call_tool(uuid, &tool_name, parameters.clone())
        .await;

    // Handle response with message history tracking
    match result {
        Ok(response) => {
            tracing::info!("Tool call successful: {}", tool_name);

            // Save response to message history
            if let Some((start_time, _)) = request_tracking {
                let db_lock = app_state.database.read().await;
                if let Some(database) = db_lock.as_ref() {
                    if let Err(e) = app_state
                        .mcp_manager
                        .track_response_with_history(uuid, start_time, &response, database)
                        .await
                    {
                        tracing::warn!("Failed to track response history: {}", e);
                    }
                }
            }

            Ok(response)
        }
        Err(e) => {
            tracing::error!("Tool call failed: {}", e);

            // Track error and save error message to history
            if let Err(track_err) = app_state
                .mcp_manager
                .track_request_error(uuid, &format!("Tool call failed: {}", e))
                .await
            {
                tracing::warn!("Failed to track request error: {}", track_err);
            }

            // Save error response to history
            let error_json = serde_json::json!({
                "error": e.to_string(),
                "method": "tools/call",
                "tool": tool_name,
                "arguments": parameters
            });

            let db_lock = app_state.database.read().await;
            if let Some(database) = db_lock.as_ref() {
                if let Err(history_err) = app_state
                    .mcp_manager
                    .save_message_to_history(
                        uuid,
                        error_json,
                        crate::types::MessageDirection::ServerToClient,
                        None,
                        database,
                    )
                    .await
                {
                    tracing::warn!("Failed to save error message to history: {}", history_err);
                }
            }

            Err(format!("Failed to call tool '{}': {}", tool_name, e))
        }
    }
}

/// List available tools on a connected MCP server
#[tauri::command]
pub async fn list_tools(
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<ToolDefinition>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // List tools using the actual manager with TurboMCP integration
    let tools = app_state
        .mcp_manager
        .list_tools(uuid)
        .await
        .map_err(|e| format!("Failed to list tools: {}", e))?;

    Ok(tools)
}

/// Get argument completions from an MCP server (TurboMCP 1.0.10)
#[tauri::command]
pub async fn get_completions(
    server_id: String,
    completion_name: String,
    partial_input: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Get completions using the TurboMCP manager
    let completions = app_state
        .mcp_manager
        .get_completions(uuid, completion_name, partial_input)
        .await
        .map_err(|e| format!("Failed to get completions: {}", e))?;

    Ok(completions)
}
