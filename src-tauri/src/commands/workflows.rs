//! Workflow Commands
//!
//! Tauri commands for workflow execution and management.
//! - Execute workflows with collections
//! - Manage workflow secrets
//! - Track workflow execution status
//! - Stop running workflows

use crate::types::collections::{Collection, WorkflowExecution};
use crate::workflow_engine::WorkflowEngine;
use crate::AppState;
use serde_json::Value;
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

/// Execute a workflow collection with variable substitution
#[tauri::command]
pub async fn execute_workflow(
    collection: Collection,
    user_variables: HashMap<String, Value>,
    app_state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<WorkflowExecution, String> {
    // Create workflow engine with MCP manager and app handle for real-time events
    let workflow_engine = WorkflowEngine::new(
        app_state.mcp_manager.clone(),
        app_state.llm_config.clone(),
        app_handle,
    );

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
    app_state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Option<WorkflowExecution>, String> {
    let uuid =
        Uuid::parse_str(&execution_id).map_err(|e| format!("Invalid execution ID: {}", e))?;

    // First check if the execution is currently active
    let workflow_engine = WorkflowEngine::new(
        app_state.mcp_manager.clone(),
        app_state.llm_config.clone(),
        app_handle,
    );

    if let Ok(Some(active_execution)) = workflow_engine.get_execution(uuid).await {
        // Return real-time status for active execution
        return Ok(Some(active_execution));
    }

    // Fall back to database for completed/persisted executions
    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        let execution = database
            .get_workflow_execution(uuid)
            .await
            .map_err(|e| format!("Failed to get workflow execution: {}", e))?;
        Ok(execution)
    } else {
        Err("Database not yet initialized".to_string())
    }
}

/// Stop a running workflow execution
#[tauri::command]
pub async fn stop_workflow_execution(
    execution_id: String,
    app_state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let uuid =
        Uuid::parse_str(&execution_id).map_err(|e| format!("Invalid execution ID: {}", e))?;

    // Create workflow engine and stop execution
    let workflow_engine = WorkflowEngine::new(
        app_state.mcp_manager.clone(),
        app_state.llm_config.clone(),
        app_handle,
    );
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
    app_state: State<'_, AppState>,
) -> Result<Vec<WorkflowExecution>, String> {
    let uuid =
        Uuid::parse_str(&collection_id).map_err(|e| format!("Invalid collection ID: {}", e))?;

    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        let executions = database
            .list_workflow_executions(uuid)
            .await
            .map_err(|e| format!("Failed to list workflow executions: {}", e))?;
        Ok(executions)
    } else {
        Ok(vec![])
    }
}

// Dead code removed - workflow variable/secret commands not registered in lib.rs
// If needed, implement WorkflowEngine::get_variable_store() methods
