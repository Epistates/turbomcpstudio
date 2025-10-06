/**
 * World-Class Collections System for MCP Studio
 *
 * This type system enables:
 * - Cross-server workflows with variable passing
 * - Operation chaining and dependencies
 * - Test scenario persistence and replay
 * - Advanced validation and assertions
 */

import type { ServerConfig } from '../stores/serverStore';

// =============================================================================
// Core Collection Types
// =============================================================================

export interface Collection {
  id: string;
  name: string;
  description?: string;
  tags: string[];

  // Workflow definition
  workflow: WorkflowStep[];

  // Global variables and configuration
  variables: Record<string, CollectionVariable>;
  environment: CollectionEnvironment;

  // Metadata
  created_at: string;
  updated_at: string;
  created_by?: string;
  version: string;

  // Execution history
  last_run?: string;
  run_count: number;
}

export interface CollectionVariable {
  name: string;
  description?: string;
  type: 'string' | 'number' | 'boolean' | 'json' | 'secret';
  default_value?: any;
  required: boolean;

  // For runtime values
  current_value?: any;
}

export interface CollectionEnvironment {
  name: string;
  description?: string;
  servers: Record<string, string>; // alias -> server_id mapping
  variables: Record<string, any>;
}

// =============================================================================
// Workflow System
// =============================================================================

export interface WorkflowStep {
  id: string;
  name: string;
  description?: string;

  // Step configuration
  enabled: boolean;
  continue_on_error: boolean;
  timeout_ms?: number;

  // Dependencies
  depends_on: string[]; // Other step IDs that must complete first

  // The actual operation
  operation: McpOperation;

  // Variable management
  variable_extracts: VariableExtract[];
  assertions: Assertion[];

  // Execution metadata
  execution_order: number;
}

export type McpOperation =
  | ToolOperation
  | ResourceOperation
  | PromptOperation
  | SamplingOperation
  | ElicitationOperation
  | DelayOperation
  | ConditionalOperation;

// =============================================================================
// MCP Operation Types
// =============================================================================

export interface ToolOperation {
  type: 'tool';
  server_alias: string; // References Collection.environment.servers
  tool_name: string;
  parameters: Record<string, any>; // Supports variable interpolation: "${variable_name}"
}

export interface ResourceOperation {
  type: 'resource';
  server_alias: string;
  resource_uri: string; // Supports variable interpolation
}

export interface PromptOperation {
  type: 'prompt';
  server_alias: string;
  prompt_name: string;
  parameters: Record<string, string>; // MCP spec requires string values only
}

export interface SamplingOperation {
  type: 'sampling';
  server_alias: string;
  messages: Array<{
    role: 'user' | 'assistant' | 'system';
    content: string; // Supports variable interpolation
  }>;
  max_tokens?: number;
  temperature?: number;
  auto_approve?: boolean; // For automated testing
}

export interface ElicitationOperation {
  type: 'elicitation';
  server_alias: string;
  message: string; // Message to show user
  requested_schema: ElicitationSchema; // Form structure (MCP restricted schema)
  auto_approve?: boolean; // For automated testing
  default_values?: Record<string, string | number | boolean>; // Pre-fill form values
}

// MCP-compliant elicitation schema (restricted to primitives only)
export interface ElicitationSchema {
  type: 'object';
  properties: Record<string, PrimitiveSchemaDefinition>;
  required?: string[];
}

export type PrimitiveSchemaDefinition =
  | StringSchemaDefinition
  | NumberSchemaDefinition
  | BooleanSchemaDefinition
  | EnumSchemaDefinition;

export interface StringSchemaDefinition {
  type: 'string';
  title?: string;
  description?: string;
  minLength?: number;
  maxLength?: number;
  format?: 'email' | 'uri' | 'date' | 'date-time';
}

export interface NumberSchemaDefinition {
  type: 'number' | 'integer';
  title?: string;
  description?: string;
  minimum?: number;
  maximum?: number;
}

export interface BooleanSchemaDefinition {
  type: 'boolean';
  title?: string;
  description?: string;
  default?: boolean;
}

export interface EnumSchemaDefinition {
  type: 'string';
  title?: string;
  description?: string;
  enum: string[];
  enumNames?: string[]; // Display names for enum values
}

export interface DelayOperation {
  type: 'delay';
  duration_ms: number;
}

export interface ConditionalOperation {
  type: 'conditional';
  condition: string; // JavaScript expression using variables
  then_steps: string[]; // Step IDs to execute if true
  else_steps?: string[]; // Step IDs to execute if false
}

// =============================================================================
// Variable System
// =============================================================================

export interface VariableExtract {
  // Where to extract the value from
  source: 'response' | 'status' | 'timing' | 'error';

  // JSONPath or simple property access
  path: string; // e.g., "$.result.token" or "response.user.id"

  // Variable to store the extracted value
  variable_name: string;

  // Optional transformations
  transform?: VariableTransform;
}

export interface VariableTransform {
  type: 'string' | 'number' | 'boolean' | 'json' | 'base64_encode' | 'base64_decode' | 'hash';
  options?: Record<string, any>;
}

// =============================================================================
// Assertion System
// =============================================================================

export interface Assertion {
  id: string;
  name: string;
  description?: string;

  // What to assert
  type: AssertionType;

  // The assertion logic
  condition: AssertionCondition;

  // Behavior on failure
  severity: 'error' | 'warning' | 'info';
  continue_on_failure: boolean;
}

export type AssertionType =
  | 'response_status'
  | 'response_contains'
  | 'response_equals'
  | 'response_json_path'
  | 'timing'
  | 'variable_value'
  | 'custom_script';

export interface AssertionCondition {
  operator: 'equals' | 'not_equals' | 'contains' | 'not_contains' | 'greater_than' | 'less_than' | 'regex_match' | 'json_schema';
  expected_value: any;
  actual_path?: string; // JSONPath for complex response validation
}

// =============================================================================
// Execution System
// =============================================================================

export interface WorkflowExecution {
  id: string;
  collection_id: string;
  collection_name?: string;
  collection_version: string;

  // Execution metadata
  started_at: string;
  completed_at?: string;
  finished_at?: string;
  status: 'running' | 'completed' | 'failed' | 'cancelled' | 'paused';

  // Results
  step_results: Record<string, StepResult>;
  final_variables: Record<string, any>;
  summary: ExecutionSummary;

  // Runtime configuration
  environment_name: string;
  user_variables: Record<string, any>;

  // Real-time events
  events?: WorkflowExecutionEvent[];
}

export interface WorkflowExecutionEvent {
  execution_id: string;
  event_type: 'step_started' | 'step_completed' | 'step_failed' | 'execution_completed' | 'execution_failed' | 'execution_paused' | 'variable_updated';
  timestamp: string;
  step_id?: string;
  message?: string;
  data?: any;
}

export interface StepResult {
  step_id: string;
  step_name?: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'skipped';

  // Operation details
  operation_type?: string;
  operation_target?: string;

  // Timing
  started_at?: string;
  completed_at?: string;
  finished_at?: string;
  duration_ms?: number;

  // Results
  operation_result?: any;
  result?: any;
  extracted_variables: Record<string, any>;
  assertion_results: AssertionResult[];

  // Error handling
  error?: string;
  retry_count: number;
}

export interface AssertionResult {
  assertion_id: string;
  passed: boolean;
  message: string;
  expected?: any;
  actual?: any;
}

export interface ExecutionSummary {
  total_steps: number;
  completed_steps: number;
  failed_steps: number;
  skipped_steps: number;

  total_duration_ms: number;
  total_assertions: number;
  passed_assertions: number;
  failed_assertions: number;

  servers_used: string[];
  operations_performed: Record<string, number>; // operation_type -> count
}

// =============================================================================
// Import/Export System
// =============================================================================

export interface CollectionExport {
  format_version: string;
  exported_at: string;
  exported_by?: string;

  collection: Collection;
  environments: CollectionEnvironment[];

  // Optional: Include recent execution history
  recent_executions?: WorkflowExecution[];
}

export interface CollectionImportOptions {
  merge_environments: boolean;
  override_existing: boolean;
  preserve_ids: boolean;

  // Server mapping for when server IDs don't match
  server_mapping?: Record<string, string>;
}

// =============================================================================
// Collection Templates System
// =============================================================================

export interface CollectionTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  tags: string[];

  // Template-specific fields
  author?: string;
  documentation_url?: string;
  required_server_types: string[]; // e.g., ["auth-server", "api-server"]

  // The template collection (with placeholder values)
  template: Omit<Collection, 'id' | 'created_at' | 'updated_at'>;

  // Instructions for users
  setup_instructions: string;
  usage_examples: string[];
}

// =============================================================================
// UI State Management
// =============================================================================

export interface CollectionUIState {
  // Current collection being edited/viewed
  active_collection_id?: string;

  // Editor state
  selected_step_id?: string;
  editor_mode: 'visual' | 'json';

  // Execution state
  current_execution_id?: string;
  execution_paused: boolean;

  // UI preferences
  show_variable_panel: boolean;
  show_assertion_panel: boolean;
  auto_save_enabled: boolean;
}

// =============================================================================
// Type Guards and Utilities
// =============================================================================

export function isToolOperation(op: McpOperation): op is ToolOperation {
  return op.type === 'tool';
}

export function isResourceOperation(op: McpOperation): op is ResourceOperation {
  return op.type === 'resource';
}

export function isPromptOperation(op: McpOperation): op is PromptOperation {
  return op.type === 'prompt';
}

export function isSamplingOperation(op: McpOperation): op is SamplingOperation {
  return op.type === 'sampling';
}

export function isElicitationOperation(op: McpOperation): op is ElicitationOperation {
  return op.type === 'elicitation';
}

// =============================================================================
// Constants
// =============================================================================

export const COLLECTION_FORMAT_VERSION = '1.0.0';

export const DEFAULT_COLLECTION_ENVIRONMENT: CollectionEnvironment = {
  name: 'Default',
  description: 'Default environment',
  servers: {},
  variables: {}
};

export const SUPPORTED_VARIABLE_TYPES = ['string', 'number', 'boolean', 'json', 'secret'] as const;

export const SUPPORTED_ASSERTION_OPERATORS = [
  'equals', 'not_equals', 'contains', 'not_contains',
  'greater_than', 'less_than', 'regex_match', 'json_schema'
] as const;