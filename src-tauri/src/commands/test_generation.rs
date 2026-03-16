//! Test Generation & Execution Commands
//!
//! Tauri commands for AI-powered test generation, execution, and management.

use crate::testing::{TestDatabase, TestExecutor, TestGenerator};
use crate::types::{Test, TestResult, TestRun, TestSuite};
use crate::AppState;
use tauri::State;

/// Generate tests for a server using AI
#[tauri::command]
pub async fn generate_test_suite(
    server_id: String,
    provider_id: Option<String>,
    model_id: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    tracing::info!(
        "Generating test suite for server: {} with provider: {:?}, model: {:?}",
        server_id,
        provider_id,
        model_id
    );

    // Get database
    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    // Get server info
    let server_uuid = uuid::Uuid::parse_str(&server_id).map_err(|e| e.to_string())?;
    let server = app_state
        .mcp_manager
        .get_server_info(server_uuid)
        .await
        .map_err(|e| format!("Server not found: {}", e))?;

    // Create test generator
    let test_db = TestDatabase::new(db.pool().clone());
    let generator = TestGenerator::new(
        app_state.llm_config.clone(),
        test_db,
        app_state.mcp_manager.clone(),
    );

    // Generate tests with specified provider and model
    let suite_id = generator
        .generate_for_server(&server, provider_id, model_id)
        .await
        .map_err(|e| format!("Test generation failed: {}", e))?;

    Ok(suite_id)
}

/// Get all test suites for a server
#[tauri::command]
pub async fn get_test_suites(
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<TestSuite>, String> {
    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    let test_db = TestDatabase::new(db.pool().clone());
    test_db
        .list_suites(&server_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get a single test suite by ID
#[tauri::command]
pub async fn get_test_suite(
    suite_id: String,
    app_state: State<'_, AppState>,
) -> Result<TestSuite, String> {
    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    let test_db = TestDatabase::new(db.pool().clone());
    test_db
        .get_suite(&suite_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all tests in a suite
#[tauri::command]
pub async fn get_tests(
    suite_id: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<Test>, String> {
    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    let test_db = TestDatabase::new(db.pool().clone());
    test_db
        .list_tests(&suite_id)
        .await
        .map_err(|e| e.to_string())
}

/// Update a test
#[tauri::command]
pub async fn update_test(test: Test, app_state: State<'_, AppState>) -> Result<(), String> {
    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    let test_db = TestDatabase::new(db.pool().clone());
    test_db.update_test(&test).await.map_err(|e| e.to_string())
}

/// Delete a test
#[tauri::command]
pub async fn delete_test(test_id: String, app_state: State<'_, AppState>) -> Result<(), String> {
    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    let test_db = TestDatabase::new(db.pool().clone());
    test_db
        .delete_test(&test_id)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a test suite
#[tauri::command]
pub async fn delete_test_suite(
    suite_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    let test_db = TestDatabase::new(db.pool().clone());
    test_db
        .delete_suite(&suite_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get test run history for a suite
#[tauri::command]
pub async fn get_test_runs(
    suite_id: String,
    limit: Option<i32>,
    app_state: State<'_, AppState>,
) -> Result<Vec<TestRun>, String> {
    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    let test_db = TestDatabase::new(db.pool().clone());
    test_db
        .list_runs(&suite_id, limit.unwrap_or(10))
        .await
        .map_err(|e| e.to_string())
}

/// Delete a test run
#[tauri::command]
pub async fn delete_test_run(run_id: String, app_state: State<'_, AppState>) -> Result<(), String> {
    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    let test_db = TestDatabase::new(db.pool().clone());
    test_db.delete_run(&run_id).await.map_err(|e| e.to_string())
}

// =============================================================================
// Test Execution Commands
// =============================================================================

/// Run all tests in a suite
#[tauri::command]
pub async fn run_test_suite(
    suite_id: String,
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    tracing::info!("Running test suite: {} on server: {}", suite_id, server_id);

    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    // Parse server ID
    let server_uuid = uuid::Uuid::parse_str(&server_id).map_err(|e| e.to_string())?;

    // Create executor
    let test_db = TestDatabase::new(db.pool().clone());
    let executor = TestExecutor::new(app_state.mcp_manager.clone(), test_db);

    // Run tests
    let run_id = executor
        .run_suite(&suite_id, server_uuid)
        .await
        .map_err(|e| format!("Test execution failed: {}", e))?;

    Ok(run_id)
}

/// Run a single test
#[tauri::command]
pub async fn run_single_test(
    test_id: String,
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<TestResult, String> {
    tracing::info!("Running single test: {} on server: {}", test_id, server_id);

    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    // Parse server ID
    let server_uuid = uuid::Uuid::parse_str(&server_id).map_err(|e| e.to_string())?;

    // Create executor
    let test_db = TestDatabase::new(db.pool().clone());
    let executor = TestExecutor::new(app_state.mcp_manager.clone(), test_db);

    // Run test
    executor
        .run_single_test(&test_id, server_uuid)
        .await
        .map_err(|e| format!("Test execution failed: {}", e))
}

/// Get results for a test run
#[tauri::command]
pub async fn get_test_results(
    run_id: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<TestResult>, String> {
    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    let test_db = TestDatabase::new(db.pool().clone());
    test_db
        .get_results_for_run(&run_id)
        .await
        .map_err(|e| e.to_string())
}

/// Compare two test runs
#[tauri::command]
pub async fn compare_test_runs(
    run_id_1: String,
    run_id_2: String,
    app_state: State<'_, AppState>,
) -> Result<crate::types::RunComparison, String> {
    let db_lock = app_state.database.read().await;
    let Some(db) = db_lock.as_ref() else {
        return Err("Database not initialized".to_string());
    };

    let test_db = TestDatabase::new(db.pool().clone());
    test_db
        .compare_runs(&run_id_1, &run_id_2)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_test_generation_commands_compile() {
        // Smoke test - ensures module compiles
    }
}
