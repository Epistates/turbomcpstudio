//! Database operations for test persistence
//!
//! Provides CRUD operations for test suites, tests, test runs, and results.

use crate::error::McpResult;
use crate::types::{
    NewTest, NewTestRun, NewTestResult, NewTestSuite, RunComparison, Test, TestResult, TestRun,
    TestRunSummary, TestSuite,
};
use sqlx::{Pool, Row, Sqlite};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Database operations for test management
#[derive(Clone)]
pub struct TestDatabase {
    pool: Pool<Sqlite>,
}

impl TestDatabase {
    /// Create a new TestDatabase instance
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Get a reference to the underlying connection pool
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    // =========================================================================
    // Test Suites
    // =========================================================================

    /// Create a new test suite
    pub async fn create_suite(&self, suite: NewTestSuite) -> McpResult<String> {
        let id = Uuid::new_v4().to_string();
        let now = SystemTime::now();

        sqlx::query(
            r#"
            INSERT INTO test_suites (
                id, server_id, name, description, version,
                created_at, updated_at, generated_at, schema_hash
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&suite.server_id)
        .bind(&suite.name)
        .bind(&suite.description)
        .bind(1) // version starts at 1
        .bind(Self::system_time_to_unix(now).to_string())
        .bind(Self::system_time_to_unix(now).to_string())
        .bind(suite.generated_at.map(|t| Self::system_time_to_unix(t).to_string()))
        .bind(&suite.schema_hash)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get a test suite by ID
    pub async fn get_suite(&self, suite_id: &str) -> McpResult<TestSuite> {
        let row = sqlx::query(
            r#"
            SELECT id, server_id, name, description, version,
                   created_at, updated_at, generated_at, schema_hash
            FROM test_suites WHERE id = ?
            "#,
        )
        .bind(suite_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(TestSuite {
            id: row.get("id"),
            server_id: row.get("server_id"),
            name: row.get("name"),
            description: row.get("description"),
            version: row.get("version"),
            created_at: Self::parse_system_time(row.get("created_at"))?,
            updated_at: Self::parse_system_time(row.get("updated_at"))?,
            generated_at: row
                .try_get::<Option<String>, _>("generated_at")
                .ok()
                .flatten()
                .and_then(|s| Self::parse_system_time(&s).ok()),
            schema_hash: row.get("schema_hash"),
        })
    }

    /// List all test suites for a server
    pub async fn list_suites(&self, server_id: &str) -> McpResult<Vec<TestSuite>> {
        let rows = sqlx::query(
            r#"
            SELECT id, server_id, name, description, version,
                   created_at, updated_at, generated_at, schema_hash
            FROM test_suites
            WHERE server_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await?;

        let mut suites = Vec::new();
        for row in rows {
            suites.push(TestSuite {
                id: row.get("id"),
                server_id: row.get("server_id"),
                name: row.get("name"),
                description: row.get("description"),
                version: row.get("version"),
                created_at: Self::parse_system_time(row.get("created_at"))?,
                updated_at: Self::parse_system_time(row.get("updated_at"))?,
                generated_at: row
                    .try_get::<Option<String>, _>("generated_at")
                    .ok()
                    .flatten()
                    .and_then(|s| Self::parse_system_time(&s).ok()),
                schema_hash: row.get("schema_hash"),
            });
        }

        Ok(suites)
    }

    /// Update a test suite
    pub async fn update_suite(&self, suite: &TestSuite) -> McpResult<()> {
        let now = SystemTime::now();

        sqlx::query(
            r#"
            UPDATE test_suites
            SET name = ?, description = ?, updated_at = ?, schema_hash = ?
            WHERE id = ?
            "#,
        )
        .bind(&suite.name)
        .bind(&suite.description)
        .bind(Self::system_time_to_unix(now).to_string())
        .bind(&suite.schema_hash)
        .bind(&suite.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a test suite and all its tests
    pub async fn delete_suite(&self, suite_id: &str) -> McpResult<()> {
        // Delete tests first (foreign key constraint)
        sqlx::query("DELETE FROM tests WHERE suite_id = ?")
            .bind(suite_id)
            .execute(&self.pool)
            .await?;

        // Delete suite
        sqlx::query("DELETE FROM test_suites WHERE id = ?")
            .bind(suite_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // =========================================================================
    // Tests
    // =========================================================================

    /// Create multiple tests at once (batch insert)
    pub async fn create_tests(&self, tests: Vec<NewTest>) -> McpResult<Vec<String>> {
        let now = SystemTime::now();
        let mut ids = Vec::new();

        for test in tests {
            let id = Uuid::new_v4().to_string();

            sqlx::query(
                r#"
                INSERT INTO tests (
                    id, suite_id, name, description, kind, test_data,
                    assertions, category, complexity, auto_generated, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&id)
            .bind(&test.suite_id)
            .bind(&test.name)
            .bind(&test.description)
            .bind(test.kind.as_str())
            .bind(serde_json::to_string(&test.test_data)?)
            .bind(serde_json::to_string(&test.assertions)?)
            .bind(test.category.as_str())
            .bind(test.complexity.as_str())
            .bind(test.auto_generated)
            .bind(Self::system_time_to_unix(now).to_string())
            .execute(&self.pool)
            .await?;

            ids.push(id);
        }

        Ok(ids)
    }

    /// Get a single test by ID
    pub async fn get_test(&self, test_id: &str) -> McpResult<Test> {
        let row = sqlx::query(
            r#"
            SELECT id, suite_id, name, description, kind, test_data,
                   assertions, category, complexity, auto_generated,
                   created_at, edited_at
            FROM tests WHERE id = ?
            "#,
        )
        .bind(test_id)
        .fetch_one(&self.pool)
        .await?;

        Self::row_to_test(row)
    }

    /// List all tests in a suite
    pub async fn list_tests(&self, suite_id: &str) -> McpResult<Vec<Test>> {
        let rows = sqlx::query(
            r#"
            SELECT id, suite_id, name, description, kind, test_data,
                   assertions, category, complexity, auto_generated,
                   created_at, edited_at
            FROM tests
            WHERE suite_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(suite_id)
        .fetch_all(&self.pool)
        .await?;

        let mut tests = Vec::new();
        for row in rows {
            tests.push(Self::row_to_test(row)?);
        }

        Ok(tests)
    }

    /// Update a test
    pub async fn update_test(&self, test: &Test) -> McpResult<()> {
        let now = SystemTime::now();

        sqlx::query(
            r#"
            UPDATE tests
            SET name = ?, description = ?, kind = ?, test_data = ?,
                assertions = ?, category = ?, complexity = ?, edited_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&test.name)
        .bind(&test.description)
        .bind(test.kind.as_str())
        .bind(serde_json::to_string(&test.test_data)?)
        .bind(serde_json::to_string(&test.assertions)?)
        .bind(test.category.as_str())
        .bind(test.complexity.as_str())
        .bind(Self::system_time_to_unix(now).to_string())
        .bind(&test.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a test
    pub async fn delete_test(&self, test_id: &str) -> McpResult<()> {
        sqlx::query("DELETE FROM tests WHERE id = ?")
            .bind(test_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // =========================================================================
    // Test Runs
    // =========================================================================

    /// Start a new test run
    pub async fn start_run(&self, run: NewTestRun) -> McpResult<String> {
        let id = Uuid::new_v4().to_string();
        let now = SystemTime::now();

        sqlx::query(
            r#"
            INSERT INTO test_runs (
                id, suite_id, started_at, total_tests, status, triggered_by,
                passed, failed, errors
            ) VALUES (?, ?, ?, ?, ?, ?, 0, 0, 0)
            "#,
        )
        .bind(&id)
        .bind(&run.suite_id)
        .bind(Self::system_time_to_unix(now).to_string())
        .bind(run.total_tests)
        .bind("running")
        .bind(&run.triggered_by)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Complete a test run with results
    pub async fn complete_run(&self, run_id: &str, summary: TestRunSummary) -> McpResult<()> {
        let now = SystemTime::now();

        sqlx::query(
            r#"
            UPDATE test_runs
            SET completed_at = ?, duration_ms = ?, passed = ?,
                failed = ?, errors = ?, status = ?
            WHERE id = ?
            "#,
        )
        .bind(Self::system_time_to_unix(now).to_string())
        .bind(summary.duration_ms)
        .bind(summary.passed)
        .bind(summary.failed)
        .bind(summary.errors)
        .bind(summary.status.as_str())
        .bind(run_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a test run by ID
    pub async fn get_run(&self, run_id: &str) -> McpResult<TestRun> {
        let row = sqlx::query(
            r#"
            SELECT id, suite_id, started_at, completed_at, duration_ms,
                   total_tests, passed, failed, errors, status, triggered_by
            FROM test_runs WHERE id = ?
            "#,
        )
        .bind(run_id)
        .fetch_one(&self.pool)
        .await?;

        Self::row_to_run(row)
    }

    /// List test runs for a suite
    pub async fn list_runs(&self, suite_id: &str, limit: i32) -> McpResult<Vec<TestRun>> {
        let rows = sqlx::query(
            r#"
            SELECT id, suite_id, started_at, completed_at, duration_ms,
                   total_tests, passed, failed, errors, status, triggered_by
            FROM test_runs
            WHERE suite_id = ?
            ORDER BY started_at DESC
            LIMIT ?
            "#,
        )
        .bind(suite_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut runs = Vec::new();
        for row in rows {
            runs.push(Self::row_to_run(row)?);
        }

        Ok(runs)
    }

    /// Delete a test run and its results
    pub async fn delete_run(&self, run_id: &str) -> McpResult<()> {
        // Delete test results first (foreign key constraint)
        sqlx::query("DELETE FROM test_results WHERE run_id = ?")
            .bind(run_id)
            .execute(&self.pool)
            .await?;

        // Delete the run
        sqlx::query("DELETE FROM test_runs WHERE id = ?")
            .bind(run_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // =========================================================================
    // Test Results
    // =========================================================================

    /// Save a test result
    pub async fn save_test_result(&self, result: NewTestResult) -> McpResult<String> {
        let id = Uuid::new_v4().to_string();
        let now = SystemTime::now();

        sqlx::query(
            r#"
            INSERT INTO test_results (
                id, run_id, test_id, passed, error_message,
                actual_result, duration_ms, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&result.run_id)
        .bind(&result.test_id)
        .bind(result.passed)
        .bind(&result.error_message)
        .bind(result.actual_result.as_ref().map(|v| v.to_string()))
        .bind(result.duration_ms)
        .bind(Self::system_time_to_unix(now).to_string())
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get all results for a test run
    pub async fn get_results_for_run(&self, run_id: &str) -> McpResult<Vec<TestResult>> {
        let rows = sqlx::query(
            r#"
            SELECT id, run_id, test_id, passed, error_message,
                   actual_result, duration_ms, timestamp
            FROM test_results
            WHERE run_id = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            results.push(Self::row_to_result(row)?);
        }

        Ok(results)
    }

    /// Compare two test runs
    pub async fn compare_runs(
        &self,
        run_id_1: &str,
        run_id_2: &str,
    ) -> McpResult<RunComparison> {
        // Get results for both runs
        let results1 = self.get_results_for_run(run_id_1).await?;
        let results2 = self.get_results_for_run(run_id_2).await?;

        // Build maps for comparison
        let map1: std::collections::HashMap<_, _> =
            results1.iter().map(|r| (r.test_id.clone(), r)).collect();
        let map2: std::collections::HashMap<_, _> =
            results2.iter().map(|r| (r.test_id.clone(), r)).collect();

        let mut newly_passing = Vec::new();
        let mut newly_failing = Vec::new();
        let mut still_passing = Vec::new();
        let mut still_failing = Vec::new();

        for (test_id, r2) in &map2 {
            if let Some(r1) = map1.get(test_id) {
                match (r1.passed, r2.passed) {
                    (false, true) => newly_passing.push(test_id.clone()),
                    (true, false) => newly_failing.push(test_id.clone()),
                    (true, true) => still_passing.push(test_id.clone()),
                    (false, false) => still_failing.push(test_id.clone()),
                }
            }
        }

        Ok(RunComparison {
            run1_id: run_id_1.to_string(),
            run2_id: run_id_2.to_string(),
            newly_passing,
            newly_failing,
            still_passing,
            still_failing,
            performance_changes: Vec::new(), // TODO: Calculate performance changes
        })
    }

    // =========================================================================
    // Helper Functions
    // =========================================================================

    /// Convert SystemTime to UNIX timestamp (seconds since epoch) for storage
    fn system_time_to_unix(time: SystemTime) -> i64 {
        time.duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// Parse UNIX timestamp (seconds since epoch) back to SystemTime
    fn parse_system_time(s: &str) -> McpResult<SystemTime> {
        // First try to parse as i64 (UNIX timestamp - new format)
        if let Ok(secs) = s.parse::<i64>() {
            return Ok(UNIX_EPOCH + Duration::from_secs(secs as u64));
        }

        // Fallback: try to parse legacy Debug format "SystemTime { tv_sec: X, tv_nsec: Y }"
        if s.starts_with("SystemTime") {
            // Extract tv_sec value from Debug format
            if let Some(start) = s.find("tv_sec: ") {
                let remainder = &s[start + 8..];
                if let Some(end) = remainder.find(|c: char| !c.is_ascii_digit()) {
                    if let Ok(secs) = remainder[..end].parse::<u64>() {
                        return Ok(UNIX_EPOCH + Duration::from_secs(secs));
                    }
                }
            }
        }

        // Fallback to current time if parsing fails
        tracing::warn!("Failed to parse timestamp '{}', using current time", s);
        Ok(SystemTime::now())
    }

    fn row_to_test(row: sqlx::sqlite::SqliteRow) -> McpResult<Test> {
        let kind_str: String = row.get("kind");
        let test_data: String = row.get("test_data");
        let assertions_str: String = row.get("assertions");
        let category_str: String = row.get("category");
        let complexity_str: String = row.get("complexity");

        Ok(Test {
            id: row.get("id"),
            suite_id: row.get("suite_id"),
            name: row.get("name"),
            description: row.get("description"),
            kind: serde_json::from_value(serde_json::json!({ "type": kind_str }))
                .unwrap_or_else(|_| {
                    crate::types::TestKind::ToolCall {
                        tool_name: "unknown".to_string(),
                        arguments: serde_json::json!({}),
                    }
                }),
            test_data: serde_json::from_str(&test_data)?,
            assertions: serde_json::from_str(&assertions_str)?,
            category: serde_json::from_value(serde_json::json!(category_str))?,
            complexity: serde_json::from_value(serde_json::json!(complexity_str))?,
            auto_generated: row.get("auto_generated"),
            created_at: Self::parse_system_time(row.get("created_at"))?,
            edited_at: row
                .try_get::<Option<String>, _>("edited_at")
                .ok()
                .flatten()
                .and_then(|s| Self::parse_system_time(&s).ok()),
        })
    }

    fn row_to_run(row: sqlx::sqlite::SqliteRow) -> McpResult<TestRun> {
        let status_str: String = row.get("status");

        Ok(TestRun {
            id: row.get("id"),
            suite_id: row.get("suite_id"),
            started_at: Self::parse_system_time(row.get("started_at"))?,
            completed_at: row
                .try_get::<Option<String>, _>("completed_at")
                .ok()
                .flatten()
                .and_then(|s| Self::parse_system_time(&s).ok()),
            duration_ms: row.get("duration_ms"),
            total_tests: row.get("total_tests"),
            passed: row.get("passed"),
            failed: row.get("failed"),
            errors: row.get("errors"),
            status: serde_json::from_value(serde_json::json!(status_str))?,
            triggered_by: row.get("triggered_by"),
        })
    }

    fn row_to_result(row: sqlx::sqlite::SqliteRow) -> McpResult<TestResult> {
        let actual_result_str: Option<String> = row.get("actual_result");

        Ok(TestResult {
            id: row.get("id"),
            run_id: row.get("run_id"),
            test_id: row.get("test_id"),
            passed: row.get("passed"),
            error_message: row.get("error_message"),
            actual_result: actual_result_str.and_then(|s| serde_json::from_str(&s).ok()),
            duration_ms: row.get("duration_ms"),
            timestamp: Self::parse_system_time(row.get("timestamp"))?,
        })
    }
}
