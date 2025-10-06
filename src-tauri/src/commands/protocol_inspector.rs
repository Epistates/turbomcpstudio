//! Protocol Inspector Commands
//!
//! Tauri commands for protocol inspection and debugging.
//! - Get message history for servers
//! - Clear message history
//! - Get connection metrics
//! - Query server information

use crate::AppState;
use tauri::State;
use uuid::Uuid;

/// Get message history for a server with optional pagination
#[tauri::command]
pub async fn get_message_history(
    server_id: String,
    limit: Option<i32>,
    offset: Option<i32>,
    app_state: State<'_, AppState>,
) -> Result<Vec<crate::types::MessageHistory>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized. Please try again in a moment.".to_string())?;

    let messages = database
        .get_message_history(uuid, limit, offset)
        .await
        .map_err(|e| format!("Failed to get message history: {}", e))?;

    Ok(messages)
}

/// Clear message history for a specific server
#[tauri::command]
pub async fn clear_message_history(
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized. Please try again in a moment.".to_string())?;

    database
        .clear_message_history(uuid)
        .await
        .map_err(|e| format!("Failed to clear message history: {}", e))?;

    tracing::info!("Cleared message history for server: {}", server_id);
    Ok(())
}

/// Get connection metrics for a specific server
#[tauri::command]
pub async fn get_connection_metrics(
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<crate::types::ConnectionMetrics, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    let metrics = app_state
        .mcp_manager
        .get_connection_metrics(uuid)
        .await
        .map_err(|e| format!("Failed to get connection metrics: {}", e))?;

    Ok(metrics)
}

/// Get comprehensive metrics for all connections
#[tauri::command]
pub async fn get_all_connection_metrics(
    app_state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, crate::types::ConnectionMetrics>, String> {
    let metrics_map = app_state.mcp_manager.get_all_connection_metrics().await;

    // Convert UUID keys to string keys for JSON serialization
    let string_map: std::collections::HashMap<String, crate::types::ConnectionMetrics> =
        metrics_map
            .into_iter()
            .map(|(uuid, metrics)| (uuid.to_string(), metrics))
            .collect();

    Ok(string_map)
}

/// Get all server information for dashboard display
#[tauri::command]
pub async fn get_all_server_info(
    app_state: State<'_, AppState>,
) -> Result<Vec<crate::types::ServerInfo>, String> {
    let server_infos = app_state.mcp_manager.get_all_server_info().await;

    Ok(server_infos)
}
