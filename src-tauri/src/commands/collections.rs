//! Collections Commands
//!
//! Tauri commands for managing collections.
//! - Save and load collections
//! - Import and export collections
//! - Manage collection templates
//! - Delete collections

use crate::types::collections::Collection;
use crate::AppState;
use tauri::State;
use uuid::Uuid;

/// Save a collection to the database
#[tauri::command]
pub async fn save_collection(
    collection: Collection,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        database
            .save_collection(&collection)
            .await
            .map_err(|e| format!("Failed to save collection: {}", e))?;
        Ok(())
    } else {
        Err("Database not yet initialized".to_string())
    }
}

/// Load a collection from the database
#[tauri::command]
pub async fn load_collection(
    collection_id: String,
    app_state: State<'_, AppState>,
) -> Result<Option<Collection>, String> {
    let uuid =
        Uuid::parse_str(&collection_id).map_err(|e| format!("Invalid collection ID: {}", e))?;

    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        let collection = database
            .load_collection(uuid)
            .await
            .map_err(|e| format!("Failed to load collection: {}", e))?;
        Ok(collection)
    } else {
        Err("Database not yet initialized".to_string())
    }
}

/// List all collections from the database
#[tauri::command]
pub async fn list_collections(app_state: State<'_, AppState>) -> Result<Vec<Collection>, String> {
    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        let collections = database
            .list_collections()
            .await
            .map_err(|e| format!("Failed to list collections: {}", e))?;
        Ok(collections)
    } else {
        Err("Database not yet initialized".to_string())
    }
}

/// Delete a collection from the database
#[tauri::command]
pub async fn delete_collection(
    collection_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid =
        Uuid::parse_str(&collection_id).map_err(|e| format!("Invalid collection ID: {}", e))?;

    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        database
            .delete_collection(uuid)
            .await
            .map_err(|e| format!("Failed to delete collection: {}", e))?;
        Ok(())
    } else {
        Err("Database not yet initialized".to_string())
    }
}

/// Export a collection to JSON format for sharing
#[tauri::command]
pub async fn export_collection(
    collection_id: String,
    include_execution_history: Option<bool>,
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    let uuid =
        Uuid::parse_str(&collection_id).map_err(|e| format!("Invalid collection ID: {}", e))?;
    let include_history = include_execution_history.unwrap_or(false);

    let db_lock = app_state.database.read().await;
    let database = db_lock.as_ref().ok_or("Database not yet initialized")?;

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
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    let overwrite = overwrite_existing.unwrap_or(false);

    // Parse the JSON
    let import_data: serde_json::Value =
        serde_json::from_str(&json_data).map_err(|e| format!("Invalid JSON format: {}", e))?;

    // Validate format version
    let format_version = import_data
        .get("format_version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    if format_version != "1.0.0" {
        return Err(format!("Unsupported format version: {}", format_version));
    }

    // Extract collection data
    let collection_data = import_data
        .get("collection")
        .ok_or_else(|| "Missing collection data in import".to_string())?;

    // Parse the collection
    let mut collection: Collection = serde_json::from_value(collection_data.clone())
        .map_err(|e| format!("Failed to parse collection: {}", e))?;

    let db_lock = app_state.database.read().await;
    let database = db_lock.as_ref().ok_or("Database not yet initialized")?;

    // Check if collection already exists
    let existing = database
        .load_collection(collection.id)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    if existing.is_some() && !overwrite {
        return Err(format!(
            "Collection '{}' already exists. Use overwrite option to replace.",
            collection.name
        ));
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
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    // Get collection data
    let export_json =
        export_collection(collection_id, include_execution_history, app_state).await?;

    // Write to file
    std::fs::write(&file_path, export_json).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(file_path)
}

/// Import collection from file using specified path
#[tauri::command]
pub async fn import_collection_from_file(
    file_path: String,
    overwrite_existing: Option<bool>,
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    // Read file
    let json_data =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Import the collection
    import_collection(json_data, overwrite_existing, app_state).await
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
        }),
    ];

    Ok(templates)
}

/// Create a collection from a template
#[tauri::command]
pub async fn create_collection_from_template(
    template_id: String,
    collection_name: String,
    variable_values: std::collections::HashMap<String, serde_json::Value>,
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    // Get templates
    let templates = get_collection_templates().await?;

    // Find the requested template
    let template = templates
        .iter()
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
    let mut variables: std::collections::HashMap<
        String,
        crate::types::collections::CollectionVariable,
    > = template
        .get("variables")
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
        description: template
            .get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string()),
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
    let db_lock = app_state.database.read().await;
    let database = db_lock.as_ref().ok_or("Database not yet initialized")?;

    database
        .save_collection(&collection)
        .await
        .map_err(|e| format!("Failed to save collection from template: {}", e))?;

    Ok(collection_id.to_string())
}
