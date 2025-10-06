use crate::database::Database;
use crate::types::{ServerConfig, ServerInfo, ToolDefinition, TransportConfig, ConnectionStatus, ConnectionMetrics};
use crate::types::collections::{Collection, WorkflowExecution};
use crate::workflow_engine::WorkflowEngine;
use crate::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
pub async fn list_servers(
    app_state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<ServerInfo>, String> {
    // Debug: Uncomment for debugging
    // println!("🔍 list_servers called");

    // Get all saved server configurations from database
    let database = app_handle
        .try_state::<Database>()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    let server_configs = database
        .list_server_configs()
        .await
        .map_err(|e| format!("Failed to load server configs: {}", e))?;

    // Debug: Found {} server configs in database

    let mut servers = Vec::new();

    // For each saved config, check if it's actively connected and get its status
    for config in server_configs {
        let server_id = config.id;
        // Debug: Processing server

        // Try to get connection status from MCP manager
        match app_state.mcp_manager.get_server_info(server_id).await {
            Ok(active_server_info) => {
                // Server is actively connected - use the live info
                // Debug: Server is connected
                servers.push(active_server_info);
            }
            Err(e) => {
                // Server is not connected - create ServerInfo with disconnected status
                // Debug: Server is disconnected
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

    // Debug: Returning servers to frontend
    Ok(servers)
}

/// Create a new server configuration
#[tauri::command]
pub async fn create_server_config(
    request: CreateServerRequest,
    database: State<'_, Database>,
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
    database
        .save_server_config(&config)
        .await
        .map_err(|e| format!("Failed to save server config: {}", e))?;

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
                    // Try to find in PATH using 'which' command
                    match Command::new("which").arg(&command).output() {
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
                        Err(format!(
                            "Permission denied executing '{}'. Try: chmod +x {}",
                            actual_command, actual_command
                        ))
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

/// Call a tool on a connected MCP server
#[tauri::command]
pub async fn call_tool(
    server_id: String,
    tool_name: String,
    parameters: serde_json::Value,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Call the tool using the actual manager with TurboMCP integration
    let result = app_state
        .mcp_manager
        .call_tool(uuid, &tool_name, parameters)
        .await
        .map_err(|e| format!("Failed to call tool '{}': {}", tool_name, e))?;

    Ok(result)
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

/// Save a server configuration to the database
#[tauri::command]
pub async fn save_server_config(
    config: ServerConfig,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Try to get database, return graceful error if not ready
    let database = app_handle
        .try_state::<Database>()
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

    // Try to get database
    let database = app_handle
        .try_state::<Database>()
        .ok_or_else(|| "Database not yet initialized. Please try again in a moment.".to_string())?;

    // Get the existing config to preserve created_at timestamp
    let existing_config = database
        .load_server_config(server_id)
        .await
        .map_err(|e| format!("Failed to find existing server config: {}", e))?
        .ok_or_else(|| "Server configuration not found".to_string())?;

    // Check if the server is currently connected and if connection details changed
    let app_state = app_handle.try_state::<AppState>().ok_or_else(|| {
        "Application state not yet initialized. Please try again in a moment.".to_string()
    })?;

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
    database
        .save_server_config(&updated_config)
        .await
        .map_err(|e| format!("Failed to save updated server config: {}", e))?;

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
    // Try to get database, return graceful error if not ready
    let database = app_handle
        .try_state::<Database>()
        .ok_or_else(|| "Database not yet initialized. Please try again in a moment.".to_string())?;

    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    database
        .delete_server_config(uuid)
        .await
        .map_err(|e| format!("Failed to delete server config: {}", e))?;

    Ok(())
}

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
        .create_sampling_request_with_config(uuid, messages, max_tokens, temperature, &app_state.llm_config)
        .await
        .map_err(|e| format!("Failed to create sampling request: {}", e))?;

    Ok(result)
}

// ================================================================================================
// LLM Configuration Commands
// ================================================================================================

/// Get current LLM configuration
#[tauri::command]
pub async fn get_llm_config(app_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = app_state.llm_config.get_config().await;
    serde_json::to_value(config).map_err(|e| format!("Serialization error: {}", e))
}

/// Get LLM provider statuses
#[tauri::command]
pub async fn get_llm_provider_statuses(app_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
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
    if let Err(e) = app_state.mcp_manager.update_sampling_handler(&app_state.llm_config).await {
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
    if let Err(e) = app_state.mcp_manager.update_sampling_handler(&app_state.llm_config).await {
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
    if let Err(e) = app_state.mcp_manager.update_sampling_handler(&app_state.llm_config).await {
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

    let request: UpdateLLMConfigRequest = serde_json::from_value(config)
        .map_err(|e| format!("Invalid configuration: {}", e))?;

    app_state
        .llm_config
        .update_provider_config(request)
        .await
        .map_err(|e| format!("Failed to update provider config: {}", e))?;

    // Update sampling handler
    if let Err(e) = app_state.mcp_manager.update_sampling_handler(&app_state.llm_config).await {
        tracing::warn!("Failed to update sampling handler: {}", e);
    }

    Ok(())
}

/// Check if sampling is available
#[tauri::command]
pub async fn is_sampling_available(app_state: State<'_, AppState>) -> Result<bool, String> {
    Ok(app_state.mcp_manager.is_sampling_available(&app_state.llm_config).await)
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

/// Send an elicitation response (respond to server-initiated user input request)
#[tauri::command]
pub async fn send_elicitation_response(
    server_id: String,
    request_id: String,
    response_data: serde_json::Value,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Send elicitation response using the actual manager with TurboMCP integration
    let result = app_state
        .mcp_manager
        .send_elicitation_response(uuid, request_id, response_data)
        .await
        .map_err(|e| format!("Failed to send elicitation response: {}", e))?;

    Ok(result)
}

/// Get pending elicitation requests for a server
#[tauri::command]
pub async fn get_elicitation_requests(
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| format!("Invalid server ID: {}", e))?;

    // Get elicitation requests using the actual manager with TurboMCP integration
    let requests = app_state
        .mcp_manager
        .get_elicitation_requests(uuid)
        .await
        .map_err(|e| format!("Failed to get elicitation requests: {}", e))?;

    Ok(requests)
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

// ============================================================================
// COLLECTIONS & WORKFLOW EXECUTION COMMANDS
// ============================================================================

/// Execute a workflow collection with variable substitution
#[tauri::command]
pub async fn execute_workflow(
    collection: Collection,
    user_variables: HashMap<String, Value>,
    app_state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<WorkflowExecution, String> {
    // Create workflow engine with MCP manager and app handle for real-time events
    let workflow_engine = WorkflowEngine::new(app_state.mcp_manager.clone(), app_state.llm_config.clone(), app_handle);

    // Execute the workflow with environment selection (None = default)
    let execution = workflow_engine
        .execute_workflow(collection, user_variables, None)
        .await
        .map_err(|e| format!("Failed to execute workflow: {}", e))?;

    Ok(execution)
}

/// Get the status and results of a workflow execution
#[tauri::command]
pub async fn get_workflow_execution(
    execution_id: String,
    database: State<'_, Database>,
) -> Result<Option<WorkflowExecution>, String> {
    let uuid = Uuid::parse_str(&execution_id).map_err(|e| format!("Invalid execution ID: {}", e))?;

    let execution = database
        .get_workflow_execution(uuid)
        .await
        .map_err(|e| format!("Failed to get workflow execution: {}", e))?;

    Ok(execution)
}

/// Stop a running workflow execution
#[tauri::command]
pub async fn stop_workflow_execution(
    execution_id: String,
    app_state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&execution_id).map_err(|e| format!("Invalid execution ID: {}", e))?;

    // Create workflow engine and stop execution
    let workflow_engine = WorkflowEngine::new(app_state.mcp_manager.clone(), app_state.llm_config.clone(), app_handle);
    workflow_engine
        .stop_execution(uuid)
        .await
        .map_err(|e| format!("Failed to stop workflow execution: {}", e))?;

    Ok(())
}

/// List all workflow executions for a collection
#[tauri::command]
pub async fn list_workflow_executions(
    collection_id: String,
    database: State<'_, Database>,
) -> Result<Vec<WorkflowExecution>, String> {
    let uuid = Uuid::parse_str(&collection_id).map_err(|e| format!("Invalid collection ID: {}", e))?;

    let executions = database
        .list_workflow_executions(uuid)
        .await
        .map_err(|e| format!("Failed to list workflow executions: {}", e))?;

    Ok(executions)
}

/// Save a collection to the database
#[tauri::command]
pub async fn save_collection(
    collection: Collection,
    database: State<'_, Database>,
) -> Result<(), String> {
    database
        .save_collection(&collection)
        .await
        .map_err(|e| format!("Failed to save collection: {}", e))?;

    Ok(())
}

/// Load a collection from the database
#[tauri::command]
pub async fn load_collection(
    collection_id: String,
    database: State<'_, Database>,
) -> Result<Option<Collection>, String> {
    let uuid = Uuid::parse_str(&collection_id).map_err(|e| format!("Invalid collection ID: {}", e))?;

    let collection = database
        .load_collection(uuid)
        .await
        .map_err(|e| format!("Failed to load collection: {}", e))?;

    Ok(collection)
}

/// List all collections from the database
#[tauri::command]
pub async fn list_collections(
    database: State<'_, Database>,
) -> Result<Vec<Collection>, String> {
    let collections = database
        .list_collections()
        .await
        .map_err(|e| format!("Failed to list collections: {}", e))?;

    Ok(collections)
}

/// Delete a collection from the database
#[tauri::command]
pub async fn delete_collection(
    collection_id: String,
    database: State<'_, Database>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&collection_id).map_err(|e| format!("Invalid collection ID: {}", e))?;

    database
        .delete_collection(uuid)
        .await
        .map_err(|e| format!("Failed to delete collection: {}", e))?;

    Ok(())
}

// =============================================================================
// Collection Import/Export Commands
// =============================================================================

/// Export a collection to JSON format for sharing
#[tauri::command]
pub async fn export_collection(
    collection_id: String,
    include_execution_history: Option<bool>,
    database: State<'_, Database>,
) -> Result<String, String> {
    let uuid = Uuid::parse_str(&collection_id).map_err(|e| format!("Invalid collection ID: {}", e))?;
    let include_history = include_execution_history.unwrap_or(false);

    // Load the collection
    let collection = database
        .load_collection(uuid)
        .await
        .map_err(|e| format!("Failed to load collection: {}", e))?
        .ok_or_else(|| "Collection not found".to_string())?;

    // Create export format
    let mut export_data = serde_json::json!({
        "format_version": "1.0.0",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "collection": collection,
        "metadata": {
            "exported_by": "MCP Studio",
            "version": env!("CARGO_PKG_VERSION")
        }
    });

    // Optionally include execution history
    if include_history {
        let executions = database
            .list_workflow_executions(uuid)
            .await
            .map_err(|e| format!("Failed to load execution history: {}", e))?;

        export_data["execution_history"] = serde_json::to_value(executions)
            .map_err(|e| format!("Failed to serialize execution history: {}", e))?;
    }

    // Serialize to pretty JSON
    serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize collection: {}", e))
}

/// Import a collection from JSON format with validation
#[tauri::command]
pub async fn import_collection(
    json_data: String,
    overwrite_existing: Option<bool>,
    database: State<'_, Database>,
) -> Result<String, String> {
    let overwrite = overwrite_existing.unwrap_or(false);

    // Parse the JSON
    let import_data: serde_json::Value = serde_json::from_str(&json_data)
        .map_err(|e| format!("Invalid JSON format: {}", e))?;

    // Validate format version
    let format_version = import_data.get("format_version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    if format_version != "1.0.0" {
        return Err(format!("Unsupported format version: {}", format_version));
    }

    // Extract collection data
    let collection_data = import_data.get("collection")
        .ok_or_else(|| "Missing collection data in import".to_string())?;

    // Parse the collection
    let mut collection: Collection = serde_json::from_value(collection_data.clone())
        .map_err(|e| format!("Failed to parse collection: {}", e))?;

    // Check if collection already exists
    let existing = database.load_collection(collection.id).await
        .map_err(|e| format!("Database error: {}", e))?;

    if existing.is_some() && !overwrite {
        return Err(format!("Collection '{}' already exists. Use overwrite option to replace.", collection.name));
    }

    // Generate new ID if overwriting is disabled and collection exists
    if existing.is_some() && overwrite {
        tracing::info!("Overwriting existing collection: {}", collection.name);
    } else if existing.is_some() {
        collection.id = Uuid::new_v4();
        collection.name = format!("{} (Imported)", collection.name);
    }

    // Update metadata
    collection.created_at = chrono::Utc::now();
    collection.updated_at = chrono::Utc::now();
    collection.run_count = 0;
    collection.last_run = None;

    // Save to database
    database
        .save_collection(&collection)
        .await
        .map_err(|e| format!("Failed to save imported collection: {}", e))?;

    Ok(collection.id.to_string())
}

/// Export collection to file using specified path
#[tauri::command]
pub async fn export_collection_to_file(
    collection_id: String,
    file_path: String,
    include_execution_history: Option<bool>,
    database: State<'_, Database>,
) -> Result<String, String> {
    // Get collection data
    let export_json = export_collection(collection_id, include_execution_history, database).await?;

    // Write to file
    std::fs::write(&file_path, export_json)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(file_path)
}

/// Import collection from file using specified path
#[tauri::command]
pub async fn import_collection_from_file(
    file_path: String,
    overwrite_existing: Option<bool>,
    database: State<'_, Database>,
) -> Result<String, String> {
    // Read file
    let json_data = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Import the collection
    import_collection(json_data, overwrite_existing, database).await
}

/// Get built-in collection templates for common MCP workflows
#[tauri::command]
pub async fn get_collection_templates() -> Result<Vec<serde_json::Value>, String> {
    let templates = vec![
        serde_json::json!({
            "name": "File System Explorer",
            "description": "Template for exploring and managing files through MCP file system servers",
            "template_id": "filesystem_explorer",
            "workflow": [
                {
                    "id": "list_roots",
                    "name": "List File System Roots",
                    "operation": {
                        "type": "resource",
                        "server_alias": "filesystem",
                        "resource_uri": "file:///"
                    },
                    "variable_extracts": [
                        {
                            "variable_name": "root_paths",
                            "path": "$.response.data",
                            "description": "Available root directories"
                        }
                    ]
                },
                {
                    "id": "explore_directory",
                    "name": "Explore Directory",
                    "operation": {
                        "type": "resource",
                        "server_alias": "filesystem",
                        "resource_uri": "file://${target_directory}"
                    },
                    "depends_on": ["list_roots"]
                }
            ],
            "variables": {
                "target_directory": {
                    "name": "target_directory",
                    "type": "string",
                    "default_value": "/",
                    "description": "Directory to explore",
                    "required": true
                }
            }
        }),
        serde_json::json!({
            "name": "API Testing Suite",
            "description": "Template for testing REST APIs and web services through MCP servers",
            "template_id": "api_testing",
            "workflow": [
                {
                    "id": "health_check",
                    "name": "API Health Check",
                    "operation": {
                        "type": "tool",
                        "server_alias": "api_client",
                        "tool_name": "get_request",
                        "parameters": {
                            "url": "${base_url}/health",
                            "headers": {}
                        }
                    },
                    "assertions": [
                        {
                            "name": "Status Code 200",
                            "condition": {
                                "operator": "equals",
                                "path": "$.status",
                                "expected_value": 200
                            },
                            "severity": "error"
                        }
                    ]
                },
                {
                    "id": "get_data",
                    "name": "Fetch Data",
                    "operation": {
                        "type": "tool",
                        "server_alias": "api_client",
                        "tool_name": "get_request",
                        "parameters": {
                            "url": "${base_url}/api/data",
                            "headers": {
                                "Authorization": "Bearer ${auth_token}"
                            }
                        }
                    },
                    "depends_on": ["health_check"],
                    "variable_extracts": [
                        {
                            "variable_name": "response_data",
                            "path": "$.response.data",
                            "description": "API response data"
                        }
                    ]
                }
            ],
            "variables": {
                "base_url": {
                    "name": "base_url",
                    "type": "string",
                    "default_value": "https://api.example.com",
                    "description": "Base API URL",
                    "required": true
                },
                "auth_token": {
                    "name": "auth_token",
                    "type": "string",
                    "description": "Authentication token",
                    "required": true
                }
            }
        }),
        serde_json::json!({
            "name": "Database Query Chain",
            "description": "Template for chaining database queries and data transformations",
            "template_id": "database_queries",
            "workflow": [
                {
                    "id": "fetch_users",
                    "name": "Fetch Users",
                    "operation": {
                        "type": "tool",
                        "server_alias": "database",
                        "tool_name": "execute_query",
                        "parameters": {
                            "query": "SELECT id, name, email FROM users WHERE active = true",
                            "parameters": []
                        }
                    },
                    "variable_extracts": [
                        {
                            "variable_name": "user_ids",
                            "path": "$.response.data[*].id",
                            "description": "List of active user IDs"
                        }
                    ]
                },
                {
                    "id": "get_user_orders",
                    "name": "Get User Orders",
                    "operation": {
                        "type": "tool",
                        "server_alias": "database",
                        "tool_name": "execute_query",
                        "parameters": {
                            "query": "SELECT * FROM orders WHERE user_id IN (${user_ids})",
                            "parameters": []
                        }
                    },
                    "depends_on": ["fetch_users"]
                }
            ]
        })
    ];

    Ok(templates)
}

/// Create a collection from a template
#[tauri::command]
pub async fn create_collection_from_template(
    template_id: String,
    collection_name: String,
    variable_values: std::collections::HashMap<String, serde_json::Value>,
    database: State<'_, Database>,
) -> Result<String, String> {
    // Get templates
    let templates = get_collection_templates().await?;

    // Find the requested template
    let template = templates.iter()
        .find(|t| t.get("template_id").and_then(|id| id.as_str()) == Some(&template_id))
        .ok_or_else(|| format!("Template '{}' not found", template_id))?;

    // Create new collection from template
    let collection_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // Parse template workflow
    let workflow_steps: Vec<crate::types::collections::WorkflowStep> = template
        .get("workflow")
        .and_then(|w| serde_json::from_value(w.clone()).ok())
        .unwrap_or_default();

    // Parse template variables and merge with provided values
    let mut variables: std::collections::HashMap<String, crate::types::collections::CollectionVariable> =
        template.get("variables")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // Update variables with provided values
    for (name, value) in variable_values {
        if let Some(var) = variables.get_mut(&name) {
            var.current_value = Some(value);
        }
    }

    let collection = Collection {
        id: collection_id,
        name: collection_name,
        description: template.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
        tags: vec!["template".to_string(), template_id],
        workflow: workflow_steps,
        variables,
        environment: crate::types::collections::CollectionEnvironment {
            name: "default".to_string(),
            description: Some("Default environment created from template".to_string()),
            servers: std::collections::HashMap::new(),
            variables: std::collections::HashMap::new(),
        },
        created_at: now,
        updated_at: now,
        created_by: Some("MCP Studio".to_string()),
        version: "1.0.0".to_string(),
        last_run: None,
        run_count: 0,
    };

    // Save to database
    database
        .save_collection(&collection)
        .await
        .map_err(|e| format!("Failed to save collection from template: {}", e))?;

    Ok(collection_id.to_string())
}

// =============================================================================
// HITL Sampling Commands
// =============================================================================

use crate::hitl_sampling::{SamplingMode, PendingSamplingRequest, SamplingResult};
use turbomcp_protocol::types::CreateMessageRequest;

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
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    // Convert JSON messages to MCP protocol format
    use turbomcp_protocol::types::{Content, Role, SamplingMessage, TextContent, IncludeContext};

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

            Ok(SamplingMessage { role, content })
        })
        .collect();

    let sampling_messages = sampling_messages?;

    let request = CreateMessageRequest {
        messages: sampling_messages,
        model_preferences: None,
        system_prompt: None,
        include_context: Some(IncludeContext::ThisServer),
        max_tokens: 1000,
        temperature: Some(0.7),
        stop_sequences: None,
        _meta: None,
        metadata: None,
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

/// Fetch available models from LLM API (avoids CORS issues)
#[tauri::command]
pub async fn fetch_llm_models(base_url: String) -> Result<Value, String> {
    let client = reqwest::Client::new();
    // Handle base URL properly - if it already ends with /v1, just add /models
    let base = base_url.trim_end_matches('/');
    let models_url = if base.ends_with("/v1") {
        format!("{}/models", base)
    } else {
        format!("{}/v1/models", base)
    };

    match client.get(&models_url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(json) => Ok(json),
                    Err(e) => Err(format!("Failed to parse models response: {}", e)),
                }
            } else {
                Err(format!("Models API returned status: {}", response.status()))
            }
        }
        Err(e) => Err(format!("Failed to fetch models: {}", e)),
    }
}

/// Make LLM completion request through Tauri (avoids CORS issues)
#[tauri::command]
pub async fn llm_completion_request(
    base_url: String,
    api_key: String,
    model: String,
    messages: Vec<Value>,
    max_tokens: Option<i32>,
    temperature: Option<f32>,
) -> Result<Value, String> {
    let client = reqwest::Client::new();
    // Handle baseUrl that may or may not already include /v1
    let base = base_url.trim_end_matches('/');
    let chat_url = if base.ends_with("/v1") {
        format!("{}/chat/completions", base)
    } else {
        format!("{}/v1/chat/completions", base)
    };

    let mut request_body = serde_json::json!({
        "model": model,
        "messages": messages
    });

    if let Some(max_tokens) = max_tokens {
        request_body["max_tokens"] = Value::Number(max_tokens.into());
    }

    if let Some(temperature) = temperature {
        request_body["temperature"] = Value::Number(serde_json::Number::from_f64(temperature as f64).unwrap_or(serde_json::Number::from(0)));
    }

    let mut request_builder = client.post(&chat_url).json(&request_body);

    // Add API key if provided
    if !api_key.is_empty() {
        request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
    }

    match request_builder.send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(json) => Ok(json),
                    Err(e) => Err(format!("Failed to parse completion response: {}", e)),
                }
            } else {
                let status = response.status();
                match response.text().await {
                    Ok(error_text) => Err(format!("LLM API error ({}): {}", status, error_text)),
                    Err(_) => Err(format!("LLM API returned status: {}", status)),
                }
            }
        }
        Err(e) => Err(format!("Failed to complete LLM request: {}", e)),
    }
}
