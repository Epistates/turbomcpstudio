/**
 * MCP Studio Sampling Types
 *
 * Comprehensive type definitions for the sampling workbench, including:
 * - Sampling requests and responses
 * - LLM configuration and providers
 * - Model preferences and capabilities
 * - Tool context and elicitation
 */

// ========================================
// CORE SAMPLING TYPES
// ========================================

export interface SamplingMessage {
  role: 'user' | 'assistant' | 'system';
  content: {
    type: 'text' | 'image' | 'audio';
    text?: string;
    data?: string;
    mimeType?: string;
    annotations?: any;
    meta?: any;
  };
}

export interface ModelPreferences {
  costPriority?: number; // 0-1
  speedPriority?: number; // 0-1
  intelligencePriority?: number; // 0-1
  hints?: { name: string }[];
}

export interface SamplingResponse {
  role: 'assistant';
  content: { type: 'text'; text: string };
  model?: string;
  stopReason?: string;
  usage?: {
    inputTokens: number;
    outputTokens: number;
  };
}

export interface ToolContext {
  toolName: string;
  parameters: Record<string, any>;
  callId: string;
}

export interface SamplingRequest {
  id: string;
  serverId: string;
  serverName: string;
  messages: SamplingMessage[];
  modelPreferences?: ModelPreferences;
  systemPrompt?: string;
  includeContext?: 'none' | 'thisServer' | 'allServers';
  maxTokens?: number;
  temperature?: number;
  stopSequences?: string[];
  timestamp: string;
  status: 'pending' | 'approved' | 'rejected' | 'completed' | 'error';
  response?: SamplingResponse;
  duration?: number;
  cost?: number;
  // MCP context - what triggered this sampling request
  toolContext?: ToolContext | null;
  operationContext?: string; // Human-readable description
}

// ========================================
// LLM CONFIGURATION TYPES
// ========================================

export interface LLMConfig {
  provider: 'openai' | 'anthropic' | 'local';
  apiKey: string;
  baseUrl?: string;
  organization?: string;
  model: string;
  defaultMaxTokens: number;
  defaultTemperature: number;
}

export interface LLMProvider {
  id: string;
  display_name: string;
  provider_type: string;
  enabled: boolean;
  configured: boolean;
  active: boolean;
  default_model: string;
  base_url?: string;
  available_models?: string[];
  capabilities?: {
    supports_structured_outputs?: boolean;
    supports_batch_processing?: boolean;
    supports_parallel_function_calling?: boolean;
    supports_computer_use?: boolean;
    max_context_tokens?: number;
    structured_output_models?: string[];
  };
  cost_config?: {
    input_cost_per_1k: number;
    output_cost_per_1k: number;
    thinking_cost_per_1k?: number;
  };
  usage_stats?: {
    total_requests: number;
    successful_requests: number;
    failed_requests: number;
    average_response_time_ms: number;
    total_cost: number;
  };
}

// ========================================
// SAMPLING MODES
// ========================================

export type SamplingMode = 'hitl' | 'hybrid' | 'ai';

export interface HybridSettings {
  interceptRequests: boolean;
  interceptResponses: boolean;
}

// ========================================
// ADVANCED LLM FEATURES
// ========================================

export type ReasoningEffort = 'minimal' | 'low' | 'medium' | 'high';
export type Verbosity = 'low' | 'medium' | 'high';
export type ThinkingBudget = 'low' | 'medium' | 'high';

export interface AdvancedLLMFeatures {
  // Structured outputs
  useStructuredOutput: boolean;
  structuredOutputSchema?: string;

  // Batch and parallel processing
  enableBatchMode: boolean;
  enableParallelFunctionCalls: boolean;

  // OpenAI GPT-5 features
  reasoningEffort: ReasoningEffort;
  verbosity: Verbosity;

  // Claude 4.1+ features
  enableExtendedThinking: boolean;
  enableComputerUse: boolean;
  showThinkingProcess: boolean;
  thinkingBudget: ThinkingBudget;

  // Context window
  maxContextTokens: number;
}

// ========================================
// ELICITATION TYPES
// ========================================

export interface ElicitationRequest {
  id: string;
  serverId: string;
  serverName?: string;
  samplingId: string;
  message: string;
  requestedSchema: any;
}

// ========================================
// USAGE STATS
// ========================================

export interface UsageStats {
  totalRequests: number;
  successfulRequests: number;
  failedRequests: number;
  averageResponseTime: number;
  totalCost: number;
}

// ========================================
// COST ESTIMATION
// ========================================

export interface CostEstimate {
  inputCost: number;
  outputCost: number;
  thinkingCost: number;
  totalCost: number;
}
