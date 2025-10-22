//! Server Configuration Commands
//!
//! Tauri commands for managing server configurations.
//! - Create and update server configurations
//! - Test server connectivity
//! - Manage server templates
//! - Load and delete configurations

use crate::database::Database;
use crate::types::{ServerConfig, TransportConfig};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{Manager, State};
use uuid::Uuid;

/// Request structure for creating a server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub description: Option<String>,
    pub transport: TransportConfig,
    pub environment_variables: HashMap<String, String>,
}

/// Request structure for updating a server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateServerRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub transport: TransportConfig,
    pub environment_variables: HashMap<String, String>,
}

/// Response structure for server templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerTemplate {
    pub name: String,
    pub description: String,
    pub transport: TransportConfig,
    pub environment_variables: HashMap<String, String>,
    pub category: String,
}

/// Create a new server configuration
#[tauri::command]
pub async fn create_server_config(
    request: CreateServerRequest,
    app_state: State<'_, AppState>,
) -> Result<ServerConfig, String> {
    let server_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let config = ServerConfig {
        id: server_id,
        name: request.name,
        description: request.description,
        transport_config: request.transport,
        environment_variables: request.environment_variables,
        created_at: now,
        updated_at: now,
    };

    // Save to database
    let db_lock = app_state.database.read().await;
    if let Some(ref database) = *db_lock {
        database
            .save_server_config(&config)
            .await
            .map_err(|e| format!("Failed to save server config: {}", e))?;
    } else {
        return Err(
            "Database not available. Please wait for the application to finish initializing."
                .to_string(),
        );
    }

    Ok(config)
}

/// Test a server configuration without persisting it
#[tauri::command]
pub async fn test_server_config(
    request: CreateServerRequest,
    _app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    use std::path::Path;
    use std::process::Command;

    // Implement actual connection test with executable validation
    match request.transport {
        TransportConfig::Stdio {
            command,
            args: _,
            working_directory,
        } => {
            if command.is_empty() {
                return Err("Command cannot be empty for STDIO transport".to_string());
            }

            // Determine the actual executable path based on working directory
            let (actual_command, actual_working_dir) = if let Some(ref wd) = working_directory {
                // If working directory is provided and command is relative, check in working directory
                let wd_path = Path::new(wd);
                if !wd_path.exists() {
                    return Err(format!("Working directory does not exist: {}", wd));
                }
                if !wd_path.is_dir() {
                    return Err(format!("Working directory is not a directory: {}", wd));
                }

                // If command is relative, construct full path from working directory
                if !Path::new(&command).is_absolute() {
                    let full_command_path = wd_path.join(&command);
                    if full_command_path.exists() {
                        // Use the full path for command execution to avoid path resolution issues
                        (
                            full_command_path.to_string_lossy().to_string(),
                            Some(wd.clone()),
                        )
                    } else {
                        return Err(format!(
                            "Executable not found: '{}' in working directory '{}'.\n\
                            Please check:\n\
                            • The executable name is correct\n\
                            • The working directory path is correct\n\
                            • The executable has been built (try 'cargo build --release' if it's a Rust project)",
                            command, wd
                        ));
                    }
                } else {
                    // Absolute path provided, check if it exists
                    if !Path::new(&command).exists() {
                        return Err(format!("Executable not found: {}", command));
                    }
                    (command.clone(), Some(wd.clone()))
                }
            } else {
                // No working directory, check if command exists in PATH or as absolute path
                if Path::new(&command).is_absolute() {
                    if !Path::new(&command).exists() {
                        return Err(format!("Executable not found: {}", command));
                    }
                } else {
                    // Try to find in PATH using platform-specific command
                    #[cfg(target_os = "windows")]
                    let which_cmd = "where";
                    #[cfg(not(target_os = "windows"))]
                    let which_cmd = "which";

                    match Command::new(which_cmd).arg(&command).output() {
                        Ok(output) if output.status.success() => {}
                        _ => {
                            return Err(format!(
                                "Executable '{}' not found in PATH.\n\
                            Please provide either:\n\
                            • The full absolute path to the executable\n\
                            • A working directory where the executable can be found",
                                command
                            ))
                        }
                    }
                }
                (command.clone(), None)
            };

            // Test if the executable is actually executable by trying to run it with --help
            let mut test_cmd = Command::new(&actual_command);
            test_cmd.arg("--help");

            if let Some(ref wd) = actual_working_dir {
                test_cmd.current_dir(wd);
            }

            match test_cmd.output() {
                Ok(_) => {
                    // Executable runs successfully
                    Ok(true)
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        Err(format!(
                            "Failed to execute '{}': Executable not found.\n\
                            Working directory: {:?}\n\
                            Please verify:\n\
                            • The executable path is correct\n\
                            • The working directory is correct\n\
                            • You have permission to execute the file",
                            actual_command, actual_working_dir
                        ))
                    } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                        #[cfg(target_os = "windows")]
                        let permission_hint = format!(
                            "Permission denied executing '{}'.\n\
                            Please check:\n\
                            • File permissions in Windows Explorer\n\
                            • Antivirus or security software may be blocking execution",
                            actual_command
                        );
                        #[cfg(not(target_os = "windows"))]
                        let permission_hint = format!(
                            "Permission denied executing '{}'.\n\
                            Try: chmod +x {}",
                            actual_command, actual_command
                        );
                        Err(permission_hint)
                    } else {
                        Err(format!(
                            "Failed to test executable '{}': {}\n\
                            Working directory: {:?}",
                            actual_command, e, actual_working_dir
                        ))
                    }
                }
            }
        }
        TransportConfig::Http { url, .. } => {
            if url.is_empty() {
                return Err("URL cannot be empty for HTTP transport".to_string());
            }
            // Basic URL validation
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err("HTTP URL must start with http:// or https://".to_string());
            }
            Ok(true)
        }
        TransportConfig::WebSocket { url, .. } => {
            if url.is_empty() {
                return Err("URL cannot be empty for WebSocket transport".to_string());
            }
            // Basic URL validation
            if !url.starts_with("ws://") && !url.starts_with("wss://") {
                return Err("WebSocket URL must start with ws:// or wss://".to_string());
            }
            Ok(true)
        }
        TransportConfig::Tcp { host, port } => {
            if host.is_empty() {
                return Err("Host cannot be empty for TCP transport".to_string());
            }
            if port == 0 {
                return Err("Port must be greater than 0 for TCP transport".to_string());
            }
            Ok(true)
        }
        TransportConfig::Unix { path } => {
            if path.is_empty() {
                return Err("Path cannot be empty for Unix transport".to_string());
            }
            // Check if the path exists or if its parent directory exists
            let unix_path = Path::new(&path);
            if let Some(parent) = unix_path.parent() {
                if !parent.exists() {
                    return Err(format!(
                        "Parent directory does not exist: {}",
                        parent.display()
                    ));
                }
            }
            Ok(true)
        }
    }
}

/// Get predefined server templates
#[tauri::command]
pub async fn get_server_templates() -> Result<Vec<ServerTemplate>, String> {
    Ok(vec![
        ServerTemplate {
            name: "TurboMCP Demo Server (Hello World)".to_string(),
            description:
                "TurboMCP hello world example server with production-grade transport implementation"
                    .to_string(),
            transport: TransportConfig::Stdio {
                command: "/Users/nickpaterno/work/turbomcp/target/release/examples/01_hello_world"
                    .to_string(),
                args: vec![],
                working_directory: None,
            },
            environment_variables: {
                let mut env = HashMap::new();
                env.insert("RUST_LOG".to_string(), "".to_string()); // Suppress logs for clean JSON-RPC
                env
            },
            category: "TurboMCP Examples".to_string(),
        },
        ServerTemplate {
            name: "Python MCP Server".to_string(),
            description: "A Python-based MCP server using STDIO transport".to_string(),
            transport: TransportConfig::Stdio {
                command: "python".to_string(),
                args: vec!["-m".to_string(), "your_mcp_server".to_string()],
                working_directory: None,
            },
            environment_variables: {
                let mut env = HashMap::new();
                env.insert("MCP_LOG_LEVEL".to_string(), "info".to_string());
                env
            },
            category: "Development".to_string(),
        },
        ServerTemplate {
            name: "Node.js MCP Server".to_string(),
            description: "A Node.js-based MCP server using STDIO transport".to_string(),
            transport: TransportConfig::Stdio {
                command: "node".to_string(),
                args: vec!["server.js".to_string()],
                working_directory: None,
            },
            environment_variables: HashMap::new(),
            category: "Development".to_string(),
        },
        ServerTemplate {
            name: "HTTP MCP Server".to_string(),
            description: "Connect to an MCP server via HTTP".to_string(),
            transport: TransportConfig::Http {
                url: "http://localhost:8000/mcp".to_string(),
                headers: {
                    let mut headers = HashMap::new();
                    headers.insert("Content-Type".to_string(), "application/json".to_string());
                    headers
                },
            },
            environment_variables: HashMap::new(),
            category: "Network".to_string(),
        },
        ServerTemplate {
            name: "WebSocket MCP Server".to_string(),
            description: "Connect to an MCP server via WebSocket".to_string(),
            transport: TransportConfig::WebSocket {
                url: "ws://localhost:8080/mcp".to_string(),
                headers: HashMap::new(),
            },
            environment_variables: HashMap::new(),
            category: "Network".to_string(),
        },
    ])
}

/// Save a server configuration to the database
#[tauri::command]
pub async fn save_server_config(
    config: ServerConfig,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Try to get database, return graceful error if not ready
    let app_state = app_handle
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not yet initialized. Please try again in a moment.".to_string())?;
    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized. Please try again in a moment.".to_string())?;

    database
        .save_server_config(&config)
        .await
        .map_err(|e| format!("Failed to save server config: {}", e))?;

    Ok(())
}

/// Update an existing server configuration
#[tauri::command]
pub async fn update_server_config(
    request: UpdateServerRequest,
    app_handle: tauri::AppHandle,
) -> Result<ServerConfig, String> {
    let server_id =
        Uuid::parse_str(&request.id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Get AppState
    let app_state = app_handle.try_state::<AppState>().ok_or_else(|| {
        "Application state not yet initialized. Please try again in a moment.".to_string()
    })?;

    // Get the existing config to preserve created_at timestamp
    let existing_config = {
        let db_lock = app_state.database.read().await;
        let database = db_lock.as_ref().ok_or_else(|| {
            "Database not yet initialized. Please try again in a moment.".to_string()
        })?;

        database
            .load_server_config(server_id)
            .await
            .map_err(|e| format!("Failed to find existing server config: {}", e))?
            .ok_or_else(|| "Server configuration not found".to_string())?
    };

    // Check if the server is currently connected and if connection details changed
    let is_connected = match app_state.mcp_manager.get_server_info(server_id).await {
        Ok(server_info) => matches!(
            server_info.status,
            crate::types::ConnectionStatus::Connected
        ),
        Err(_) => false, // Server not found or error, treat as not connected
    };
    let connection_changed = existing_config.transport_config != request.transport;

    // If server is connected and connection details changed, disconnect first
    if is_connected && connection_changed {
        let _ = app_state.mcp_manager.disconnect_server(server_id).await;
    }

    // Create updated config with new timestamp
    let now = chrono::Utc::now();
    let updated_config = ServerConfig {
        id: server_id,
        name: request.name,
        description: request.description,
        transport_config: request.transport,
        environment_variables: request.environment_variables,
        created_at: existing_config.created_at, // Preserve original creation time
        updated_at: now,
    };

    // Save the updated configuration
    {
        let db_lock = app_state.database.read().await;
        let database = db_lock.as_ref().ok_or_else(|| {
            "Database not yet initialized. Please try again in a moment.".to_string()
        })?;

        database
            .save_server_config(&updated_config)
            .await
            .map_err(|e| format!("Failed to save updated server config: {}", e))?;
    }

    Ok(updated_config)
}

/// Load all server configurations from the database
#[tauri::command]
pub async fn load_server_configs(
    app_handle: tauri::AppHandle,
) -> Result<Vec<ServerConfig>, String> {
    // Try to get database, return empty list if not ready
    let database = app_handle
        .try_state::<Database>()
        .ok_or_else(|| "Database not yet initialized. Please try again in a moment.".to_string())?;

    let configs = database
        .list_server_configs()
        .await
        .map_err(|e| format!("Failed to load server configs: {}", e))?;

    Ok(configs)
}

/// Delete a server configuration from the database
#[tauri::command]
pub async fn delete_server_config(
    server_id: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    log::debug!(
        "delete_server_config called with server_id: {}",
        server_id
    );

    // Try to get database, return graceful error if not ready
    let app_state = app_handle.try_state::<AppState>().ok_or_else(|| {
        log::error!("AppState not initialized");
        "AppState not yet initialized. Please try again in a moment.".to_string()
    })?;

    let db_lock = app_state.database.read().await;
    let database = db_lock.as_ref().ok_or_else(|| {
        log::error!("Database not initialized");
        "Database not yet initialized. Please try again in a moment.".to_string()
    })?;

    log::debug!("Database connection acquired");

    let uuid = Uuid::parse_str(&server_id).map_err(|e| {
        log::error!("Invalid UUID format: {}", e);
        format!("Invalid server ID: {}", e)
    })?;

    log::debug!("Parsed UUID: {}", uuid);
    log::debug!("Calling database.delete_server_config...");

    database.delete_server_config(uuid).await.map_err(|e| {
        log::error!("Database deletion failed: {}", e);
        format!("Failed to delete server config: {}", e)
    })?;

    log::info!(
        "Successfully deleted server configuration: {}",
        server_id
    );
    Ok(())
}
