use crate::database::Database;
use crate::types::{ServerConfig, ServerInfo, ToolDefinition, TransportConfig};
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
pub async fn list_servers(app_state: State<'_, AppState>) -> Result<Vec<ServerInfo>, String> {
    // List servers using the actual manager
    let servers = app_state
        .mcp_manager
        .list_servers()
        .await
        .map_err(|e| format!("Failed to list servers: {}", e))?;

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
        transport: request.transport,
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
    let connection_changed = existing_config.transport != request.transport;

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
        transport: request.transport,
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

    // Create sampling request using the actual manager with TurboMCP integration
    let result = app_state
        .mcp_manager
        .create_sampling_request(uuid, messages, max_tokens, temperature)
        .await
        .map_err(|e| format!("Failed to create sampling request: {}", e))?;

    Ok(result)
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
