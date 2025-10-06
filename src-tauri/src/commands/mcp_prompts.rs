//! MCP Prompts Commands
//!
//! Tauri commands for MCP prompt operations.
//! - List available prompts
//! - Get prompt details with arguments

use crate::AppState;
use tauri::State;
use uuid::Uuid;

/// List available prompts on a connected MCP server
#[tauri::command]
pub async fn list_prompts(
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // List prompts using the actual manager with TurboMCP integration
    let prompts = app_state
        .mcp_manager
        .list_prompts(uuid)
        .await
        .map_err(|e| format!("Failed to list prompts: {}", e))?;

    Ok(prompts)
}

/// Get a specific prompt from a connected MCP server
#[tauri::command]
pub async fn get_prompt(
    server_id: String,
    prompt_name: String,
    parameters: serde_json::Value,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Get prompt using the actual manager with TurboMCP integration
    let prompt = app_state
        .mcp_manager
        .get_prompt(uuid, prompt_name.clone(), Some(parameters))
        .await
        .map_err(|e| format!("Failed to get prompt '{}': {}", prompt_name, e))?;

    Ok(prompt)
}
