//! Server Lifecycle Commands
//!
//! Tauri commands for managing MCP server connections and lifecycle.
//! - Connect/disconnect to servers
//! - Query server status and information
//! - List available servers

use crate::types::{ConnectionMetrics, ConnectionStatus, ServerConfig, ServerInfo};
use crate::AppState;
use tauri::{Manager, State};
use uuid::Uuid;

/// Connect to an MCP server (using stored configuration)
#[tauri::command]
pub async fn connect_server(
    server_config: ServerConfig,
    app_state: State<'_, AppState>,
) -> Result<ServerInfo, String> {
    // Connect to the server using the actual manager with TurboMCP integration
    let result = app_state
        .mcp_manager
        .connect_server(server_config)
        .await
        .map_err(|e| format!("Failed to connect to server: {}", e))?;

    Ok(result)
}

/// Disconnect from an MCP server
#[tauri::command]
pub async fn disconnect_server(
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Disconnect from the server using the actual manager
    app_state
        .mcp_manager
        .disconnect_server(uuid)
        .await
        .map_err(|e| format!("Failed to disconnect from server: {}", e))?;

    Ok(())
}

/// Get information about a specific server
#[tauri::command]
pub async fn get_server_info(
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<ServerInfo, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Get server info using the actual manager
    let server_info = app_state
        .mcp_manager
        .get_server_info(uuid)
        .await
        .map_err(|e| format!("Failed to get server info: {}", e))?;

    Ok(server_info)
}

/// List all configured servers
#[tauri::command]
pub async fn list_servers(app_handle: tauri::AppHandle) -> Result<Vec<ServerInfo>, String> {
    // Get all saved server configurations from database
    let app_state = app_handle
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not yet initialized".to_string())?;

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    let server_configs = database
        .list_server_configs()
        .await
        .map_err(|e| format!("Failed to load server configs: {}", e))?;

    let mut servers = Vec::new();

    // For each saved config, check if it's actively connected and get its status
    for config in server_configs {
        let server_id = config.id;

        // Try to get connection status from MCP manager
        match app_state.mcp_manager.get_server_info(server_id).await {
            Ok(active_server_info) => {
                // Server is actively connected - use the live info
                servers.push(active_server_info);
            }
            Err(_e) => {
                // Server is not connected - create ServerInfo with disconnected status
                servers.push(ServerInfo {
                    id: server_id,
                    config,
                    status: ConnectionStatus::Disconnected,
                    capabilities: None,
                    process_info: None,
                    metrics: ConnectionMetrics::default(),
                    last_seen: chrono::Utc::now(),
                });
            }
        }
    }

    Ok(servers)
}
