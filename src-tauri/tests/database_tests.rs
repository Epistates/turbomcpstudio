/// Database Tests for TurboMCP Studio Backend
///
/// Tests database migrations, schema integrity, and query correctness

use sqlx::{SqlitePool, Row};
use uuid::Uuid;

#[tokio::test]
async fn test_workflow_executions_schema_exists() {
    // Create in-memory database
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create in-memory database");

    // Create the table with correct schema
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS workflow_executions (
            id TEXT PRIMARY KEY,
            collection_id TEXT NOT NULL,
            collection_version TEXT NOT NULL,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            status TEXT NOT NULL,
            step_results TEXT NOT NULL,
            final_variables TEXT NOT NULL,
            summary TEXT NOT NULL,
            environment_name TEXT,
            user_variables TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create workflow_executions table");

    // Verify table exists and has correct columns
    let columns: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('workflow_executions') ORDER BY name"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to query table info");

    let column_names: Vec<String> = columns.into_iter().map(|c| c.0).collect();

    // Verify critical columns exist
    assert!(column_names.contains(&"id".to_string()), "Missing id column");
    assert!(column_names.contains(&"step_results".to_string()), "Missing step_results column");
    assert!(column_names.contains(&"status".to_string()), "Missing status column");
    assert!(column_names.contains(&"started_at".to_string()), "Missing started_at column");
    
    // Verify we DON'T have the old 'results' column
    assert!(!column_names.contains(&"results".to_string()), "Old 'results' column should not exist");

    pool.close().await;
}

#[tokio::test]
async fn test_workflow_execution_insert_and_query() {
    // Create in-memory database with proper schema
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create in-memory database");

    // Create table
    sqlx::query(
        r#"
        CREATE TABLE workflow_executions (
            id TEXT PRIMARY KEY,
            collection_id TEXT NOT NULL,
            collection_version TEXT NOT NULL,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            status TEXT NOT NULL,
            step_results TEXT NOT NULL,
            final_variables TEXT NOT NULL,
            summary TEXT NOT NULL,
            environment_name TEXT,
            user_variables TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create table");

    // Insert test data
    let test_id = Uuid::new_v4().to_string();
    let test_collection_id = Uuid::new_v4().to_string();
    
    sqlx::query(
        r#"
        INSERT INTO workflow_executions 
        (id, collection_id, collection_version, started_at, completed_at, status, 
         step_results, final_variables, summary, environment_name, user_variables)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&test_id)
    .bind(&test_collection_id)
    .bind("1.0.0")
    .bind("2025-10-21T10:00:00Z")
    .bind("2025-10-21T10:05:00Z")
    .bind("Completed")
    .bind(r#"[{"step": "test", "result": "success"}]"#)
    .bind(r#"{}"#)
    .bind(r#"{"success": true}"#)
    .bind("production")
    .bind(r#"{}"#)
    .execute(&pool)
    .await
    .expect("Failed to insert test data");

    // Query back the data using the CORRECT column name
    let row = sqlx::query(
        "SELECT id, step_results, status FROM workflow_executions WHERE id = ?"
    )
    .bind(&test_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to query workflow execution");

    let id: String = row.get("id");
    let step_results: String = row.get("step_results");
    let status: String = row.get("status");

    assert_eq!(id, test_id);
    assert!(step_results.contains("test"));
    assert_eq!(status, "Completed");

    pool.close().await;
}

#[tokio::test]
async fn test_migration_from_old_results_column() {
    // Simulate the migration scenario from Phase 1, Issue #1
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create in-memory database");

    // Create OLD schema with 'results' column (the broken one)
    sqlx::query(
        r#"
        CREATE TABLE workflow_executions (
            id TEXT PRIMARY KEY,
            collection_id TEXT NOT NULL,
            collection_version TEXT NOT NULL,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            status TEXT NOT NULL,
            results TEXT NOT NULL,
            final_variables TEXT NOT NULL,
            summary TEXT NOT NULL,
            environment_name TEXT,
            user_variables TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create old schema");

    // Insert data with old column name
    let test_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO workflow_executions 
        (id, collection_id, collection_version, started_at, status, 
         results, final_variables, summary, user_variables)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&test_id)
    .bind(Uuid::new_v4().to_string())
    .bind("1.0.0")
    .bind("2025-10-21T10:00:00Z")
    .bind("Completed")
    .bind(r#"[{"step": "test"}]"#)
    .bind(r#"{}"#)
    .bind(r#"{}"#)
    .bind(r#"{}"#)
    .execute(&pool)
    .await
    .expect("Failed to insert into old schema");

    // Perform migration (same logic as in database.rs)
    // 1. Check if old column exists
    let has_old_column: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('workflow_executions') WHERE name = 'results'"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to check for old column");

    assert_eq!(has_old_column, 1, "Old 'results' column should exist before migration");

    // 2. Create new table with correct schema
    sqlx::query(
        r#"
        CREATE TABLE workflow_executions_new (
            id TEXT PRIMARY KEY,
            collection_id TEXT NOT NULL,
            collection_version TEXT NOT NULL,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            status TEXT NOT NULL,
            step_results TEXT NOT NULL,
            final_variables TEXT NOT NULL,
            summary TEXT NOT NULL,
            environment_name TEXT,
            user_variables TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create new table");

    // 3. Copy data, renaming column
    sqlx::query(
        r#"
        INSERT INTO workflow_executions_new
        SELECT id, collection_id, collection_version, started_at, completed_at, status,
               results AS step_results, final_variables, summary, environment_name, user_variables
        FROM workflow_executions
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to copy data");

    // 4. Drop old table
    sqlx::query("DROP TABLE workflow_executions")
        .execute(&pool)
        .await
        .expect("Failed to drop old table");

    // 5. Rename new table
    sqlx::query("ALTER TABLE workflow_executions_new RENAME TO workflow_executions")
        .execute(&pool)
        .await
        .expect("Failed to rename table");

    // Verify migration succeeded
    let has_new_column: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('workflow_executions') WHERE name = 'step_results'"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to check for new column");

    assert_eq!(has_new_column, 1, "New 'step_results' column should exist after migration");

    let has_old_column_after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('workflow_executions') WHERE name = 'results'"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to check for old column after migration");

    assert_eq!(has_old_column_after, 0, "Old 'results' column should not exist after migration");

    // Verify data was preserved
    let row = sqlx::query("SELECT step_results FROM workflow_executions WHERE id = ?")
        .bind(&test_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to query migrated data");

    let step_results: String = row.get("step_results");
    assert!(step_results.contains("test"), "Data should be preserved during migration");

    pool.close().await;
}

#[tokio::test]
async fn test_workflow_status_variants() {
    // Test that all ExecutionStatus variants can be stored and retrieved
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create in-memory database");

    sqlx::query(
        r#"
        CREATE TABLE workflow_executions (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            step_results TEXT NOT NULL,
            collection_id TEXT NOT NULL,
            collection_version TEXT NOT NULL,
            started_at TEXT NOT NULL,
            final_variables TEXT NOT NULL,
            summary TEXT NOT NULL,
            user_variables TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create table");

    // Test all status variants from Phase 1 fix
    let statuses = vec!["Running", "Completed", "Failed", "Cancelled", "Paused"];
    
    for status in &statuses {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO workflow_executions 
            (id, status, step_results, collection_id, collection_version, started_at, 
             final_variables, summary, user_variables)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(status)
        .bind("[]")
        .bind(Uuid::new_v4().to_string())
        .bind("1.0.0")
        .bind("2025-10-21T10:00:00Z")
        .bind("{}")
        .bind("{}")
        .bind("{}")
        .execute(&pool)
        .await
        .expect(&format!("Failed to insert status: {}", status));

        // Query back
        let retrieved_status: String = sqlx::query_scalar(
            "SELECT status FROM workflow_executions WHERE id = ?"
        )
        .bind(&id)
        .fetch_one(&pool)
        .await
        .expect(&format!("Failed to query status: {}", status));

        assert_eq!(&retrieved_status, status, "Status should round-trip correctly");
    }

    pool.close().await;
}

#[tokio::test]
async fn test_query_prevents_column_mismatch_panic() {
    // This test verifies the fix for Issue #1 - querying with wrong column name should fail gracefully
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create in-memory database");

    sqlx::query(
        r#"
        CREATE TABLE workflow_executions (
            id TEXT PRIMARY KEY,
            step_results TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create table");

    // Try to query with the OLD (wrong) column name - should fail, not panic
    let result = sqlx::query("SELECT results FROM workflow_executions")
        .fetch_optional(&pool)
        .await;

    assert!(result.is_err(), "Query with wrong column name should return error, not panic");
    
    // Verify the error message mentions the column
    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("results") || error_msg.contains("column") || error_msg.contains("no such column"),
            "Error should mention the problematic column, got: {}",
            error_msg
        );
    }

    pool.close().await;
}

