//! World-Class Collections System - Rust Implementation
//!
//! This module provides enterprise-grade collection and workflow types that mirror
//! the TypeScript interface but with Rust's safety and performance advantages.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// Core Collection Types
// =============================================================================

/// Enhanced Collection with advanced workflow capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,

    // Workflow definition
    pub workflow: Vec<WorkflowStep>,

    // Global variables and configuration
    pub variables: HashMap<String, CollectionVariable>,
    pub environment: CollectionEnvironment,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub version: String,

    // Execution history
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionVariable {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub var_type: VariableType,
    pub default_value: Option<serde_json::Value>,
    pub required: bool,
    pub current_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    String,
    Number,
    Boolean,
    Json,
    Secret,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionEnvironment {
    pub name: String,
    pub description: Option<String>,
    pub servers: HashMap<String, Uuid>, // alias -> server_id mapping
    pub variables: HashMap<String, serde_json::Value>,
}

// =============================================================================
// Workflow System
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,

    // Step configuration
    pub enabled: bool,
    pub continue_on_error: bool,
    pub timeout_ms: Option<u32>,

    // Dependencies
    pub depends_on: Vec<Uuid>, // Other step IDs that must complete first

    // The actual operation
    pub operation: McpOperation,

    // Variable management
    pub variable_extracts: Vec<VariableExtract>,
    pub assertions: Vec<Assertion>,

    // Execution metadata
    pub execution_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpOperation {
    Tool {
        server_alias: String,
        tool_name: String,
        parameters: HashMap<String, serde_json::Value>,
    },
    Resource {
        server_alias: String,
        resource_uri: String,
    },
    Prompt {
        server_alias: String,
        prompt_name: String,
        parameters: HashMap<String, serde_json::Value>,
    },
    Sampling {
        server_alias: String,
        messages: Vec<SamplingMessage>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        auto_approve: Option<bool>,
    },
    Elicitation {
        server_alias: String,
        request_id: String,
        response_data: serde_json::Value,
    },
    Delay {
        duration_ms: u32,
    },
    Conditional {
        condition: String, // JavaScript expression using variables
        then_steps: Vec<Uuid>, // Step IDs to execute if true
        else_steps: Option<Vec<Uuid>>, // Step IDs to execute if false
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingMessage {
    pub role: MessageRole,
    pub content: serde_json::Value, // Can be string or content array
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

// =============================================================================
// Variable System
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableExtract {
    // Where to extract the value from
    pub source: ExtractSource,

    // JSONPath or simple property access
    pub path: String, // e.g., "$.result.token" or "response.user.id"

    // Variable to store the extracted value
    pub variable_name: String,

    // Optional transformations
    pub transform: Option<VariableTransform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractSource {
    Response,
    Status,
    Timing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableTransform {
    #[serde(rename = "type")]
    pub transform_type: TransformType,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformType {
    String,
    Number,
    Boolean,
    Json,
    Base64Encode,
    Base64Decode,
    Hash,
}

// =============================================================================
// Assertion System
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,

    // What to assert
    #[serde(rename = "type")]
    pub assertion_type: AssertionType,

    // The assertion logic
    pub condition: AssertionCondition,

    // Behavior on failure
    pub severity: AssertionSeverity,
    pub continue_on_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionType {
    ResponseStatus,
    ResponseContains,
    ResponseEquals,
    ResponseJsonPath,
    Timing,
    VariableValue,
    CustomScript,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionCondition {
    pub operator: AssertionOperator,
    pub expected_value: serde_json::Value,
    pub actual_path: Option<String>, // JSONPath for complex response validation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    RegexMatch,
    JsonSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionSeverity {
    Error,
    Warning,
    Info,
}

// =============================================================================
// Execution System
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub collection_version: String,

    // Execution metadata
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,

    // Results
    pub step_results: HashMap<Uuid, StepResult>,
    pub final_variables: HashMap<String, serde_json::Value>,
    pub summary: ExecutionSummary,

    // Runtime configuration
    pub environment_name: String,
    pub user_variables: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// Event emitted during workflow execution for real-time monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionEvent {
    pub execution_id: String,
    pub event_type: String,
    pub timestamp: String,
    pub step_id: Option<String>,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: Uuid,
    pub status: StepStatus,

    // Timing
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u32>,

    // Results
    pub operation_result: Option<serde_json::Value>,
    pub extracted_variables: HashMap<String, serde_json::Value>,
    pub assertion_results: Vec<AssertionResult>,

    // Error handling
    pub error: Option<String>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    pub assertion_id: Uuid,
    pub passed: bool,
    pub message: String,
    pub expected: Option<serde_json::Value>,
    pub actual: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub total_steps: u32,
    pub completed_steps: u32,
    pub failed_steps: u32,
    pub skipped_steps: u32,

    pub total_duration_ms: u32,
    pub total_assertions: u32,
    pub passed_assertions: u32,
    pub failed_assertions: u32,

    pub servers_used: Vec<String>,
    pub operations_performed: HashMap<String, u32>, // operation_type -> count
}

// =============================================================================
// Import/Export System
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionExport {
    pub format_version: String,
    pub exported_at: DateTime<Utc>,
    pub exported_by: Option<String>,

    pub collection: Collection,
    pub environments: Vec<CollectionEnvironment>,

    // Optional: Include recent execution history
    pub recent_executions: Option<Vec<WorkflowExecution>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionImportOptions {
    pub merge_environments: bool,
    pub override_existing: bool,
    pub preserve_ids: bool,

    // Server mapping for when server IDs don't match
    pub server_mapping: Option<HashMap<Uuid, Uuid>>,
}

// =============================================================================
// Collection Templates System
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,

    // Template-specific fields
    pub author: Option<String>,
    pub documentation_url: Option<String>,
    pub required_server_types: Vec<String>, // e.g., ["auth-server", "api-server"]

    // The template collection (with placeholder values)
    pub template_collection: Collection,

    // Instructions for users
    pub setup_instructions: String,
    pub usage_examples: Vec<String>,
}

// =============================================================================
// Constants
// =============================================================================

pub const COLLECTION_FORMAT_VERSION: &str = "1.0.0";

impl Default for CollectionEnvironment {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            description: Some("Default environment".to_string()),
            servers: HashMap::new(),
            variables: HashMap::new(),
        }
    }
}

impl Collection {
    /// Create a new empty collection
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            tags: Vec::new(),
            workflow: Vec::new(),
            variables: HashMap::new(),
            environment: CollectionEnvironment::default(),
            created_at: now,
            updated_at: now,
            created_by: None,
            version: COLLECTION_FORMAT_VERSION.to_string(),
            last_run: None,
            run_count: 0,
        }
    }

    /// Add a workflow step to the collection
    pub fn add_step(&mut self, step: WorkflowStep) {
        self.workflow.push(step);
        self.updated_at = Utc::now();
    }

    /// Update a workflow step
    pub fn update_step(&mut self, step_id: Uuid, step: WorkflowStep) -> Result<(), String> {
        if let Some(existing_step) = self.workflow.iter_mut().find(|s| s.id == step_id) {
            *existing_step = step;
            self.updated_at = Utc::now();
            Ok(())
        } else {
            Err(format!("Step with ID {} not found", step_id))
        }
    }

    /// Remove a workflow step
    pub fn remove_step(&mut self, step_id: Uuid) -> Result<(), String> {
        let initial_len = self.workflow.len();
        self.workflow.retain(|s| s.id != step_id);

        if self.workflow.len() < initial_len {
            self.updated_at = Utc::now();
            Ok(())
        } else {
            Err(format!("Step with ID {} not found", step_id))
        }
    }

    /// Get steps in execution order, respecting dependencies
    pub fn get_execution_order(&self) -> Result<Vec<&WorkflowStep>, String> {
        let mut executed = std::collections::HashSet::new();
        let mut execution_order = Vec::new();
        let mut remaining_steps: std::collections::HashMap<Uuid, &WorkflowStep> =
            self.workflow.iter().map(|s| (s.id, s)).collect();

        // Simple dependency resolution (topological sort)
        while !remaining_steps.is_empty() {
            let mut progress_made = false;

            // Find steps with all dependencies satisfied
            let ready_steps: Vec<Uuid> = remaining_steps
                .iter()
                .filter(|(_, step)| {
                    step.depends_on.iter().all(|dep_id| executed.contains(dep_id))
                })
                .map(|(id, _)| *id)
                .collect();

            if ready_steps.is_empty() {
                return Err("Circular dependency detected in workflow steps".to_string());
            }

            // Sort ready steps by execution_order
            let mut ready_with_order: Vec<_> = ready_steps
                .into_iter()
                .map(|id| (remaining_steps[&id].execution_order, id))
                .collect();
            ready_with_order.sort_by_key(|(order, _)| *order);

            // Add ready steps to execution order
            for (_, step_id) in ready_with_order {
                if let Some(step) = remaining_steps.remove(&step_id) {
                    execution_order.push(step);
                    executed.insert(step_id);
                    progress_made = true;
                }
            }

            if !progress_made {
                return Err("Unable to resolve step dependencies".to_string());
            }
        }

        Ok(execution_order)
    }
}

impl WorkflowStep {
    /// Create a new workflow step
    pub fn new(name: String, operation: McpOperation) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            enabled: true,
            continue_on_error: false,
            timeout_ms: None,
            depends_on: Vec::new(),
            operation,
            variable_extracts: Vec::new(),
            assertions: Vec::new(),
            execution_order: 0,
        }
    }
}