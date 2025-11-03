//! Test generation and execution types
//!
//! Supports AI-powered test generation, execution, and historical tracking.
//! Tests are organized per-server with full persistence.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// =============================================================================
// Test Suites
// =============================================================================

/// A collection of tests for a specific MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: i32,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub generated_at: Option<SystemTime>,
    pub schema_hash: Option<String>,
}

/// Create a new test suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTestSuite {
    pub server_id: String,
    pub name: String,
    pub description: Option<String>,
    pub generated_at: Option<SystemTime>,
    pub schema_hash: Option<String>,
}

// =============================================================================
// Tests
// =============================================================================

/// A single test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Test {
    pub id: String,
    pub suite_id: String,
    pub name: String,
    pub description: Option<String>,
    pub kind: TestKind,
    pub test_data: serde_json::Value,
    pub assertions: Vec<Assertion>,
    pub category: TestCategory,
    pub complexity: TestComplexity,
    pub auto_generated: bool,
    pub created_at: SystemTime,
    pub edited_at: Option<SystemTime>,
}

/// Create a new test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTest {
    pub suite_id: String,
    pub name: String,
    pub description: Option<String>,
    pub kind: TestKind,
    pub test_data: serde_json::Value,
    pub assertions: Vec<Assertion>,
    pub category: TestCategory,
    pub complexity: TestComplexity,
    pub auto_generated: bool,
}

/// Type of test
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TestKind {
    ToolCall {
        tool_name: String,
        arguments: serde_json::Value,
    },
    ResourceRead {
        uri: String,
    },
    PromptGet {
        name: String,
        arguments: Option<serde_json::Value>,
    },
    Workflow {
        steps: Vec<WorkflowStep>,
    },
}

impl TestKind {
    /// Convert to string for database storage
    pub fn as_str(&self) -> &str {
        match self {
            TestKind::ToolCall { .. } => "tool_call",
            TestKind::ResourceRead { .. } => "resource_read",
            TestKind::PromptGet { .. } => "prompt_get",
            TestKind::Workflow { .. } => "workflow",
        }
    }
}

/// A step in a workflow test
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowStep {
    pub order: i32,
    pub action: WorkflowAction,
    pub expected_outcome: Option<String>,
}

/// Workflow action types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowAction {
    CallTool {
        tool_name: String,
        arguments: serde_json::Value,
    },
    ReadResource {
        uri: String,
    },
    GetPrompt {
        name: String,
        arguments: Option<serde_json::Value>,
    },
    Delay {
        milliseconds: u64,
    },
}

/// Test assertion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Assertion {
    StatusEquals {
        expected: String,
    },
    ContentContains {
        substring: String,
    },
    ContentMatches {
        regex: String,
    },
    ContentEquals {
        expected: serde_json::Value,
    },
    ResponseTimeUnder {
        milliseconds: u64,
    },
    ErrorCodeEquals {
        code: i32,
    },
    ErrorMessageContains {
        substring: String,
    },
    FieldEquals {
        field_path: String,
        expected: serde_json::Value,
    },
    ArrayLengthEquals {
        expected: usize,
    },
}

/// Test category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TestCategory {
    HappyPath,
    EdgeCase,
    Error,
    Security,
    Workflow,
    Performance,
}

impl TestCategory {
    pub fn as_str(&self) -> &str {
        match self {
            TestCategory::HappyPath => "happy_path",
            TestCategory::EdgeCase => "edge_case",
            TestCategory::Error => "error",
            TestCategory::Security => "security",
            TestCategory::Workflow => "workflow",
            TestCategory::Performance => "performance",
        }
    }
}

/// Test complexity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TestComplexity {
    Simple,
    Medium,
    Complex,
}

impl TestComplexity {
    pub fn as_str(&self) -> &str {
        match self {
            TestComplexity::Simple => "simple",
            TestComplexity::Medium => "medium",
            TestComplexity::Complex => "complex",
        }
    }
}

// =============================================================================
// Test Runs (Execution History)
// =============================================================================

/// A test run (execution of a test suite)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRun {
    pub id: String,
    pub suite_id: String,
    pub started_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub duration_ms: Option<i64>,
    pub total_tests: i32,
    pub passed: i32,
    pub failed: i32,
    pub errors: i32,
    pub status: TestRunStatus,
    pub triggered_by: String,
}

/// Start a new test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTestRun {
    pub suite_id: String,
    pub total_tests: i32,
    pub triggered_by: String,
}

/// Test run status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TestRunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TestRunStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TestRunStatus::Running => "running",
            TestRunStatus::Completed => "completed",
            TestRunStatus::Failed => "failed",
            TestRunStatus::Cancelled => "cancelled",
        }
    }
}

/// Summary of test run results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunSummary {
    pub duration_ms: i64,
    pub passed: i32,
    pub failed: i32,
    pub errors: i32,
    pub status: TestRunStatus,
}

// =============================================================================
// Test Results (Individual Test Outcomes)
// =============================================================================

/// Result of running a single test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub id: String,
    pub run_id: String,
    pub test_id: String,
    pub passed: bool,
    pub error_message: Option<String>,
    pub actual_result: Option<serde_json::Value>,
    pub duration_ms: i64,
    pub timestamp: SystemTime,
}

/// Create a new test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTestResult {
    pub run_id: String,
    pub test_id: String,
    pub passed: bool,
    pub error_message: Option<String>,
    pub actual_result: Option<serde_json::Value>,
    pub duration_ms: i64,
}

// =============================================================================
// Test Generation
// =============================================================================

/// AI-generated test suite (from LLM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedTestSuite {
    #[serde(default)]
    pub suite_name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub tests: Vec<GeneratedTest>,
}

/// AI-generated test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedTest {
    pub name: String,
    pub description: Option<String>,
    pub category: TestCategory,
    pub complexity: TestComplexity,
    pub kind: TestKind,
    pub test_data: serde_json::Value,
    pub assertions: Vec<Assertion>,
}

// =============================================================================
// Schema Analysis
// =============================================================================

/// Server schema analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaAnalysis {
    pub patterns: Vec<Pattern>,
    pub complexity: ComplexityScore,
    pub coverage_areas: Vec<TestArea>,
    pub hash: String,
}

/// Detected patterns in server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Pattern {
    Crud,
    Search,
    Authentication,
    Pagination,
    Workflow,
    AsyncOperation,
    FileOperation,
    DataTransformation,
}

/// Complexity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityScore {
    pub tool_count: usize,
    pub resource_count: usize,
    pub prompt_count: usize,
    pub total_score: i32,
}

/// Test coverage area
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TestArea {
    HappyPath,
    EdgeCases,
    ErrorHandling,
    Security,
    Workflows,
    Performance,
}

// =============================================================================
// Run Comparison
// =============================================================================

/// Comparison between two test runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunComparison {
    pub run1_id: String,
    pub run2_id: String,
    pub newly_passing: Vec<String>,
    pub newly_failing: Vec<String>,
    pub still_passing: Vec<String>,
    pub still_failing: Vec<String>,
    pub performance_changes: Vec<PerformanceChange>,
}

/// Performance change between runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceChange {
    pub test_id: String,
    pub test_name: String,
    pub old_duration_ms: i64,
    pub new_duration_ms: i64,
    pub change_percent: f64,
}
