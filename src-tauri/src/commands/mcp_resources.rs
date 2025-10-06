//! MCP Resources Commands
//!
//! Tauri commands for MCP resource operations.
//! - List available resources
//! - Read resource contents
//! - List filesystem roots
//! - Check handler status

use crate::AppState;
use tauri::State;
use uuid::Uuid;

/// List available resources on a connected MCP server
#[tauri::command]
pub async fn list_resources(
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // List resources using the actual manager with TurboMCP integration
    let resources = app_state
        .mcp_manager
        .list_resources(uuid)
        .await
        .map_err(|e| format!("Failed to list resources: {}", e))?;

    Ok(resources)
}

/// Read a specific resource from a connected MCP server
#[tauri::command]
pub async fn read_resource(
    server_id: String,
    resource_uri: String,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Read resource using the actual manager with TurboMCP integration
    let resource = app_state
        .mcp_manager
        .read_resource(uuid, resource_uri.clone())
        .await
        .map_err(|e| format!("Failed to read resource '{}': {}", resource_uri, e))?;

    Ok(resource)
}

/// List filesystem roots available to the server (TurboMCP 1.0.10)
#[tauri::command]
pub async fn list_filesystem_roots(
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // List filesystem roots using the TurboMCP manager
    let roots = app_state
        .mcp_manager
        .list_filesystem_roots(uuid)
        .await
        .map_err(|e| format!("Failed to list filesystem roots: {}", e))?;

    Ok(roots)
}

/// Check handler registration status for a server (TurboMCP 1.0.10)
#[tauri::command]
pub async fn get_handler_status(
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Get handler status using the TurboMCP manager
    let status = app_state
        .mcp_manager
        .get_handler_status(uuid)
        .await
        .map_err(|e| format!("Failed to get handler status: {}", e))?;

    Ok(status)
}
