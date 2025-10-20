//! Registry-related Tauri commands

use crate::registry::*;
use crate::types::{ServerConfig, TransportConfig};
use crate::AppState;
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

/// Partial server config from registry (without id, timestamps)
#[derive(Debug, serde::Deserialize)]
pub struct RegistryServerConfig {
    pub name: String,
    pub description: Option<String>,
    pub transport_config: TransportConfig,
    pub environment_variables: HashMap<String, String>,
}

/// Fetches the Docker MCP registry catalog
#[tauri::command]
pub async fn fetch_registry_catalog() -> Result<HashMap<String, RegistryServer>, String> {
    fetch_registry()
        .await
        .map_err(|e| format!("Failed to fetch registry: {}", e))
}

/// Refreshes the registry catalog (clears cache)
#[tauri::command]
pub async fn refresh_registry_catalog() -> Result<HashMap<String, RegistryServer>, String> {
    refresh_registry()
        .await
        .map_err(|e| format!("Failed to refresh registry: {}", e))
}

/// Searches servers in the registry
#[tauri::command]
pub fn search_registry_servers(
    servers: HashMap<String, RegistryServer>,
    query: String,
) -> Result<Vec<ServerDisplayInfo>, String> {
    Ok(search_servers(&servers, &query))
}

/// Filters servers by category
#[tauri::command]
pub fn filter_registry_by_category(
    servers: HashMap<String, RegistryServer>,
    category: String,
) -> Result<Vec<ServerDisplayInfo>, String> {
    Ok(filter_by_category(&servers, &category))
}

/// Gets all available categories
#[tauri::command]
pub fn get_registry_categories(
    servers: HashMap<String, RegistryServer>,
) -> Result<Vec<String>, String> {
    Ok(get_categories(&servers))
}

/// Generates configuration for a specific client
#[tauri::command]
pub fn generate_client_config(
    server: RegistryServer,
    user_config: UserConfig,
    client_type: String,
) -> Result<GeneratedConfig, String> {
    let client = match client_type.as_str() {
        "turbomcp" => ClientType::TurboMCP,
        "claude-desktop" => ClientType::ClaudeDesktop,
        "claude-code" => ClientType::ClaudeCode,
        "lmstudio" => ClientType::LMStudio,
        "cursor" => ClientType::Cursor,
        "cline" => ClientType::Cline,
        _ => return Err(format!("Unknown client type: {}", client_type)),
    };

    generate_config(&server, &user_config, client)
        .map_err(|e| format!("Failed to generate config: {}", e))
}

/// Gets detailed information about a server
#[tauri::command]
pub fn get_server_details(
    servers: HashMap<String, RegistryServer>,
    server_name: String,
) -> Result<Option<RegistryServer>, String> {
    Ok(servers.get(&server_name).cloned())
}

/// Adds a server from the registry to the local server manager
#[tauri::command]
pub async fn add_server_from_registry(
    config: RegistryServerConfig,
    app_state: State<'_, AppState>,
) -> Result<ServerConfig, String> {
    // Generate missing fields
    let now = chrono::Utc::now();
    let full_config = ServerConfig {
        id: Uuid::new_v4(),
        name: config.name,
        description: config.description,
        transport_config: config.transport_config,
        environment_variables: config.environment_variables,
        created_at: now,
        updated_at: now,
    };

    // Save to database
    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        database
            .save_server_config(&full_config)
            .await
            .map_err(|e| format!("Failed to save server config: {}", e))?;
    } else {
        return Err(
            "Database not available. Please wait for the application to finish initializing."
                .to_string(),
        );
    }

    Ok(full_config)
}

/// Checks if Docker is available on the system
#[tauri::command]
pub async fn check_docker_available() -> Result<bool, String> {
    crate::registry::check_docker_available()
        .await
        .map_err(|e| format!("Failed to check Docker availability: {}", e))
}
