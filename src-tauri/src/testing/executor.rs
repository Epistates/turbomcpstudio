//! Test execution engine
//!
//! Executes tests in parallel with proper error handling,
//! timeout management, and result tracking.

use crate::error::{McpResult, McpStudioError};
use crate::mcp_client::McpClientManager;
use crate::testing::TestDatabase;
use crate::types::{
    Assertion, NewTestResult, NewTestRun, Test, TestKind, TestResult, TestRunStatus,
    TestRunSummary, WorkflowAction,
};
use futures::future::join_all;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use uuid::Uuid;

/// Test executor with parallel execution capabilities
pub struct TestExecutor {
    mcp_client: Arc<McpClientManager>,
    db: TestDatabase,
    concurrency: usize,
    timeout: Duration,
}

impl TestExecutor {
    /// Create a new test executor
    pub fn new(mcp_client: Arc<McpClientManager>, db: TestDatabase) -> Self {
        Self {
            mcp_client,
            db,
            concurrency: 5,                   // Run 5 tests concurrently
            timeout: Duration::from_secs(30), // 30s timeout per test
        }
    }

    /// Run all tests in a suite
    pub async fn run_suite(&self, suite_id: &str, server_id: Uuid) -> McpResult<String> {
        tracing::info!("Starting test run for suite: {}", suite_id);

        // Get all tests in suite
        let tests = self.db.list_tests(suite_id).await?;
        if tests.is_empty() {
            return Err(McpStudioError::ConfigError(
                "Test suite has no tests".to_string(),
            ));
        }

        tracing::info!("Found {} tests to execute", tests.len());

        // Create test run record
        let run_id = self
            .db
            .start_run(NewTestRun {
                suite_id: suite_id.to_string(),
                total_tests: tests.len() as i32,
                triggered_by: "user".to_string(),
            })
            .await?;

        tracing::info!("Created test run: {}", run_id);

        let start = Instant::now();

        // Execute tests in parallel
        let results = self
            .execute_tests_parallel(&tests, server_id, &run_id)
            .await;

        let duration = start.elapsed();

        // Calculate summary
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = results.len() - passed;

        let summary = TestRunSummary {
            duration_ms: duration.as_millis() as i64,
            passed: passed as i32,
            failed: failed as i32,
            errors: 0, // TODO: Track errors separately from failures
            status: TestRunStatus::Completed,
        };

        // Complete the run
        self.db.complete_run(&run_id, summary).await?;

        tracing::info!(
            "Test run completed: {} passed, {} failed, {}ms",
            passed,
            failed,
            duration.as_millis()
        );

        Ok(run_id)
    }

    /// Execute a single test
    pub async fn run_single_test(&self, test_id: &str, server_id: Uuid) -> McpResult<TestResult> {
        tracing::info!("Running single test: {}", test_id);

        let test = self.db.get_test(test_id).await?;

        // Create a temporary run for this test
        let run_id = self
            .db
            .start_run(NewTestRun {
                suite_id: test.suite_id.clone(),
                total_tests: 1,
                triggered_by: "user".to_string(),
            })
            .await?;

        let start = Instant::now();
        let result = self.execute_single_test(&test, server_id).await;
        let duration = start.elapsed();

        // Extract actual result and determine pass/fail
        let (passed, error_message, actual_result) = match result {
            Ok(actual) => (true, None, actual),
            Err(e) => (false, Some(e.to_string()), None),
        };

        // Save result
        let test_result = TestResult {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.clone(),
            test_id: test_id.to_string(),
            passed,
            error_message: error_message.clone(),
            actual_result: actual_result.clone(),
            duration_ms: duration.as_millis() as i64,
            timestamp: std::time::SystemTime::now(),
        };

        self.db
            .save_test_result(NewTestResult {
                run_id: run_id.clone(),
                test_id: test_id.to_string(),
                passed,
                error_message,
                actual_result,
                duration_ms: test_result.duration_ms,
            })
            .await?;

        // Complete the run
        let summary = TestRunSummary {
            duration_ms: duration.as_millis() as i64,
            passed: if test_result.passed { 1 } else { 0 },
            failed: if test_result.passed { 0 } else { 1 },
            errors: 0,
            status: TestRunStatus::Completed,
        };
        self.db.complete_run(&run_id, summary).await?;

        Ok(test_result)
    }

    /// Execute tests in parallel with concurrency control
    async fn execute_tests_parallel(
        &self,
        tests: &[Test],
        server_id: Uuid,
        run_id: &str,
    ) -> Vec<TestResult> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency));

        // Spawn tasks for each test
        let handles: Vec<_> = tests
            .iter()
            .map(|test| {
                let test = test.clone();
                let run_id = run_id.to_string();
                let sem = semaphore.clone();
                let timeout = self.timeout;
                let db = self.db.clone();
                let mcp_client = self.mcp_client.clone();

                tokio::spawn(async move {
                    // Acquire semaphore permit (limits concurrency)
                    let _permit = sem.acquire().await.unwrap();

                    let start = Instant::now();

                    // Execute with timeout
                    let result = tokio::time::timeout(
                        timeout,
                        Self::execute_test_static(&test, server_id, &mcp_client),
                    )
                    .await;

                    let duration = start.elapsed();

                    let (passed, error_message, actual_result) = match result {
                        Ok(Ok(actual)) => (true, None, actual),
                        Ok(Err(e)) => (false, Some(e.to_string()), None),
                        Err(_) => (
                            false,
                            Some(format!("Test timed out after {}s", timeout.as_secs())),
                            None,
                        ),
                    };

                    // Create result
                    let test_result = TestResult {
                        id: Uuid::new_v4().to_string(),
                        run_id: run_id.clone(),
                        test_id: test.id.clone(),
                        passed,
                        error_message: error_message.clone(),
                        actual_result: actual_result.clone(),
                        duration_ms: duration.as_millis() as i64,
                        timestamp: std::time::SystemTime::now(),
                    };

                    // Save result to database
                    let _ = db
                        .save_test_result(NewTestResult {
                            run_id: run_id.clone(),
                            test_id: test.id.clone(),
                            passed,
                            error_message,
                            actual_result,
                            duration_ms: duration.as_millis() as i64,
                        })
                        .await;

                    test_result
                })
            })
            .collect();

        // Wait for all tests to complete
        let results = join_all(handles).await;

        // Unwrap results (tasks shouldn't panic)
        results.into_iter().filter_map(|r| r.ok()).collect()
    }

    /// Execute a single test (static to allow spawning)
    /// Returns (success, actual_result) tuple for debugging and assertion checking
    async fn execute_test_static(
        test: &Test,
        server_id: Uuid,
        mcp_client: &McpClientManager,
    ) -> McpResult<Option<serde_json::Value>> {
        tracing::debug!("Executing test: {}", test.name);

        // Execute based on test kind
        let actual_result = match &test.kind {
            TestKind::ToolCall {
                tool_name,
                arguments,
            } => {
                // Call the tool
                let result = mcp_client
                    .call_tool(server_id, tool_name, arguments.clone())
                    .await?;

                serde_json::to_value(result).ok()
            }
            TestKind::ResourceRead { uri } => {
                // Read the resource
                let result = mcp_client.read_resource(server_id, uri.clone()).await?;

                serde_json::to_value(result).ok()
            }
            TestKind::PromptGet { name, arguments } => {
                // Get the prompt
                let result = mcp_client
                    .get_prompt(server_id, name.clone(), arguments.clone())
                    .await?;

                serde_json::to_value(result).ok()
            }
            TestKind::Workflow { steps } => {
                // Execute workflow steps in sequence, ordered by `order` field
                let mut sorted_steps = steps.clone();
                sorted_steps.sort_by_key(|s| s.order);

                let mut last_result = None;
                for step in sorted_steps {
                    tracing::debug!("Executing workflow step {}: {:?}", step.order, step.action);

                    let step_result = match &step.action {
                        WorkflowAction::CallTool {
                            tool_name,
                            arguments,
                        } => {
                            mcp_client
                                .call_tool(server_id, tool_name, arguments.clone())
                                .await?
                        }
                        WorkflowAction::ReadResource { uri } => {
                            mcp_client.read_resource(server_id, uri.clone()).await?
                        }
                        WorkflowAction::GetPrompt { name, arguments } => {
                            mcp_client
                                .get_prompt(server_id, name.clone(), arguments.clone())
                                .await?
                        }
                        WorkflowAction::Delay { milliseconds } => {
                            tokio::time::sleep(Duration::from_millis(*milliseconds)).await;
                            serde_json::json!({"status": "completed", "delay_ms": milliseconds})
                        }
                    };

                    // Verify expected outcome if specified
                    if let Some(expected) = &step.expected_outcome {
                        let result_str = serde_json::to_string(&step_result).unwrap_or_default();
                        if !result_str.contains(expected) {
                            return Err(McpStudioError::ConfigError(format!(
                                "Workflow step {} failed: expected '{}' not found in result",
                                step.order, expected
                            )));
                        }
                    }

                    last_result = Some(step_result);
                }

                last_result
            }
        };

        // Check assertions
        if let Some(ref result) = actual_result {
            Self::check_assertions(&test.assertions, result)?;
        }

        Ok(actual_result)
    }

    /// Execute a single test (instance method)
    async fn execute_single_test(
        &self,
        test: &Test,
        server_id: Uuid,
    ) -> McpResult<Option<serde_json::Value>> {
        Self::execute_test_static(test, server_id, &self.mcp_client).await
    }

    /// Check assertions against actual result
    fn check_assertions(assertions: &[Assertion], actual: &serde_json::Value) -> McpResult<()> {
        for assertion in assertions {
            match assertion {
                Assertion::StatusEquals { expected } => {
                    // Check if result has a status field
                    if let Some(status) = actual.get("status") {
                        if status.as_str() != Some(expected) {
                            return Err(McpStudioError::ConfigError(format!(
                                "Status assertion failed: expected '{}', got '{}'",
                                expected,
                                status.as_str().unwrap_or("null")
                            )));
                        }
                    }
                }
                Assertion::ContentContains { substring } => {
                    let content_str = serde_json::to_string(actual).unwrap_or_default();
                    if !content_str.contains(substring) {
                        return Err(McpStudioError::ConfigError(format!(
                            "Content assertion failed: '{}' not found in result",
                            substring
                        )));
                    }
                }
                Assertion::ContentMatches { regex } => {
                    let content_str = serde_json::to_string(actual).unwrap_or_default();
                    let re = regex::Regex::new(regex).map_err(|e| {
                        McpStudioError::ConfigError(format!("Invalid regex: {}", e))
                    })?;
                    if !re.is_match(&content_str) {
                        return Err(McpStudioError::ConfigError(format!(
                            "Regex assertion failed: pattern '{}' did not match",
                            regex
                        )));
                    }
                }
                Assertion::ContentEquals { expected } => {
                    if actual != expected {
                        return Err(McpStudioError::ConfigError(
                            "Content equality assertion failed".to_string(),
                        ));
                    }
                }
                Assertion::ResponseTimeUnder { milliseconds } => {
                    // This should be checked at the executor level, not here
                    // We'll skip for now
                    tracing::debug!("ResponseTimeUnder assertion: {}ms", milliseconds);
                }
                Assertion::ErrorCodeEquals { code } => {
                    if let Some(error_code) = actual.get("code") {
                        if error_code.as_i64() != Some(*code as i64) {
                            return Err(McpStudioError::ConfigError(format!(
                                "Error code assertion failed: expected {}, got {}",
                                code,
                                error_code.as_i64().unwrap_or(-1)
                            )));
                        }
                    }
                }
                Assertion::ErrorMessageContains { substring } => {
                    if let Some(error_msg) = actual.get("message") {
                        if let Some(msg_str) = error_msg.as_str() {
                            if !msg_str.contains(substring) {
                                return Err(McpStudioError::ConfigError(format!(
                                    "Error message assertion failed: '{}' not found",
                                    substring
                                )));
                            }
                        }
                    }
                }
                Assertion::FieldEquals {
                    field_path,
                    expected,
                } => {
                    // Navigate to field using JSON pointer syntax
                    let actual_value = actual.pointer(field_path);
                    if actual_value != Some(expected) {
                        return Err(McpStudioError::ConfigError(format!(
                            "Field assertion failed: {} != expected",
                            field_path
                        )));
                    }
                }
                Assertion::ArrayLengthEquals { expected } => {
                    if let Some(arr) = actual.as_array() {
                        if arr.len() != *expected {
                            return Err(McpStudioError::ConfigError(format!(
                                "Array length assertion failed: expected {}, got {}",
                                expected,
                                arr.len()
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_assertions_content_contains() {
        let assertions = vec![Assertion::ContentContains {
            substring: "success".to_string(),
        }];

        let result = serde_json::json!({"status": "success", "data": "test"});
        assert!(TestExecutor::check_assertions(&assertions, &result).is_ok());

        let result = serde_json::json!({"status": "error"});
        assert!(TestExecutor::check_assertions(&assertions, &result).is_err());
    }

    #[test]
    fn test_check_assertions_array_length() {
        let assertions = vec![Assertion::ArrayLengthEquals { expected: 3 }];

        let result = serde_json::json!([1, 2, 3]);
        assert!(TestExecutor::check_assertions(&assertions, &result).is_ok());

        let result = serde_json::json!([1, 2]);
        assert!(TestExecutor::check_assertions(&assertions, &result).is_err());
    }
}
