/**
 * LLM Configuration and Provider Types
 */

export type LLMProviderType = 'openai' | 'anthropic' | 'local';

export interface RateLimitConfig {
  requests_per_minute: number;
  tokens_per_minute?: number;
  exponential_backoff: boolean;
  initial_backoff_ms: number;
  max_backoff_ms: number;
}

export interface CostConfig {
  input_cost_per_1k: number;
  output_cost_per_1k: number;
  thinking_cost_per_1k?: number;
  currency: string;
}

export interface LLMProviderCapabilities {
  supports_structured_outputs: boolean;
  structured_output_models: string[];
  max_structured_output_tokens?: number;
  supports_batch_processing: boolean;
  batch_discount_percentage?: number;
  supports_parallel_function_calling: boolean;
  supports_strict_function_calling: boolean;
  supports_vision: boolean;
  supported_image_formats: string[];
  supports_audio: boolean;
  supported_audio_formats: string[];
  supports_streaming: boolean;
  supports_function_calling: boolean;
  supports_computer_use: boolean;
  supports_thinking_tokens: boolean;
  max_context_tokens?: number;
}

export interface LLMProviderConfig {
  provider_type: LLMProviderType;
  display_name: string;
  enabled: boolean;
  default_model: string;
  available_models: string[];
  base_url?: string;
  organization?: string;
  max_retries: number;
  timeout_seconds: number;
  max_tokens: number;
  rate_limit: RateLimitConfig;
  cost_config: CostConfig;
  capabilities: LLMProviderCapabilities;
}

export interface GlobalLLMConfig {
  enable_usage_tracking: boolean;
  daily_spending_limit?: number;
  monthly_spending_limit?: number;
  enable_cost_warnings: boolean;
  cost_warning_threshold: number;
  default_temperature: number;
  default_max_tokens?: number;
}

export interface LLMConfiguration {
  active_provider?: string;
  providers: Record<string, LLMProviderConfig>;
  global_config: GlobalLLMConfig;
}

export interface ProviderUsageStats {
  total_requests: number;
  successful_requests: number;
  failed_requests: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cost: number;
  average_response_time_ms: number;
  last_request_at?: string;
}

export interface LLMProviderStatus {
  provider_id: string;
  display_name: string;
  provider_type: LLMProviderType;
  enabled: boolean;
  configured: boolean;
  active: boolean;
  available_models: string[];
  base_url?: string;
  last_error?: string;
  usage_stats?: ProviderUsageStats;
}

export interface LLMMessage {
  role: 'user' | 'assistant' | 'system';
  content: {
    type: 'text';
    text: string;
  };
}

export interface LLMMessageRequest {
  messages: LLMMessage[];
  modelPreferences?: {
    hints?: Array<{ name: string }>;
  };
  maxTokens?: number;
}

export interface LLMMessageResponse {
  content: Array<{
    type: 'text';
    text: string;
  }> | string | { text: string };
  usage?: {
    input_tokens: number;
    output_tokens: number;
  };
}
