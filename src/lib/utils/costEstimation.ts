/**
 * MCP Studio Cost Estimation
 *
 * Accurate cost estimation for LLM API usage including:
 * - Input/output token pricing
 * - Thinking/reasoning token costs (GPT-5, Claude 4.1+)
 * - Provider-specific features
 * - Effort and budget multipliers
 */

import type {
  SamplingMessage,
  LLMProvider,
  ReasoningEffort,
  ThinkingBudget,
  CostEstimate
} from '$lib/types/sampling';

// ========================================
// COST ESTIMATION
// ========================================

/**
 * Estimate cost for a sampling request
 *
 * @param messages - Array of messages in the request
 * @param maxTokens - Maximum tokens for response (default: 500)
 * @param activeProvider - LLM provider with cost configuration
 * @param reasoningEffort - OpenAI GPT-5 reasoning effort level
 * @param enableExtendedThinking - Claude 4.1+ extended thinking mode
 * @param thinkingBudget - Claude thinking budget level
 * @returns Detailed cost breakdown
 */
export function estimateRequestCost(
  messages: SamplingMessage[],
  maxTokens: number = 500,
  activeProvider: LLMProvider | null,
  reasoningEffort: ReasoningEffort = 'medium',
  enableExtendedThinking: boolean = false,
  thinkingBudget: ThinkingBudget = 'medium'
): CostEstimate {
  if (!activeProvider?.cost_config) {
    return { inputCost: 0, outputCost: 0, thinkingCost: 0, totalCost: 0 };
  }

  // Token estimation (rough approximation: ~4 characters per token)
  const inputText = messages.map((m) => m.content?.text || '').join(' ');
  const estimatedInputTokens = Math.ceil(inputText.length / 4);
  const estimatedOutputTokens = Math.min(maxTokens, 150); // Conservative estimate

  // Base costs
  const inputCost =
    (estimatedInputTokens / 1000) * activeProvider.cost_config.input_cost_per_1k;
  const outputCost =
    (estimatedOutputTokens / 1000) * activeProvider.cost_config.output_cost_per_1k;

  let thinkingCost = 0;

  // Provider-specific cost calculations
  const isOpenAIGPT5 =
    activeProvider.provider_type === 'openai' &&
    activeProvider.default_model?.includes('gpt-5');

  const isClaudeWithThinking =
    activeProvider.provider_type === 'anthropic' &&
    activeProvider.default_model?.includes('claude-4.1-opus');

  // OpenAI GPT-5: Reasoning tokens count as output tokens
  if (isOpenAIGPT5 && reasoningEffort !== 'minimal') {
    const reasoningMultipliers: Record<ReasoningEffort, number> = {
      minimal: 1.0,
      low: 1.2,
      medium: 1.5,
      high: 2.0
    };

    const multiplier = reasoningMultipliers[reasoningEffort];
    const adjustedOutputTokens = estimatedOutputTokens * multiplier;
    const adjustedOutputCost =
      (adjustedOutputTokens / 1000) * activeProvider.cost_config.output_cost_per_1k;

    thinkingCost = adjustedOutputCost - outputCost;
  }

  // Claude 4.1 Opus: Separate thinking token pricing
  else if (
    isClaudeWithThinking &&
    enableExtendedThinking &&
    activeProvider.cost_config.thinking_cost_per_1k
  ) {
    const thinkingMultipliers: Record<ThinkingBudget, number> = {
      low: 0.2,
      medium: 0.4,
      high: 0.6
    };

    const multiplier = thinkingMultipliers[thinkingBudget];
    const estimatedThinkingTokens = Math.ceil(estimatedInputTokens * multiplier);

    thinkingCost =
      (estimatedThinkingTokens / 1000) * activeProvider.cost_config.thinking_cost_per_1k;
  }

  const totalCost = inputCost + outputCost + thinkingCost;

  return {
    inputCost: Number(inputCost.toFixed(4)),
    outputCost: Number(outputCost.toFixed(4)),
    thinkingCost: Number(thinkingCost.toFixed(4)),
    totalCost: Number(totalCost.toFixed(4))
  };
}

/**
 * Calculate actual cost from LLM response with usage data
 *
 * @param inputTokens - Actual input tokens used
 * @param outputTokens - Actual output tokens used
 * @param thinkingTokens - Actual thinking tokens used (if supported)
 * @param activeProvider - LLM provider with cost configuration
 * @returns Detailed cost breakdown
 */
export function calculateActualCost(
  inputTokens: number,
  outputTokens: number,
  thinkingTokens: number = 0,
  activeProvider: LLMProvider | null
): CostEstimate {
  if (!activeProvider?.cost_config) {
    return { inputCost: 0, outputCost: 0, thinkingCost: 0, totalCost: 0 };
  }

  const inputCost = (inputTokens / 1000) * activeProvider.cost_config.input_cost_per_1k;
  const outputCost = (outputTokens / 1000) * activeProvider.cost_config.output_cost_per_1k;

  let thinkingCost = 0;
  if (thinkingTokens > 0 && activeProvider.cost_config.thinking_cost_per_1k) {
    thinkingCost = (thinkingTokens / 1000) * activeProvider.cost_config.thinking_cost_per_1k;
  }

  const totalCost = inputCost + outputCost + thinkingCost;

  return {
    inputCost: Number(inputCost.toFixed(4)),
    outputCost: Number(outputCost.toFixed(4)),
    thinkingCost: Number(thinkingCost.toFixed(4)),
    totalCost: Number(totalCost.toFixed(4))
  };
}

/**
 * Format cost for display with appropriate precision
 *
 * @param cost - Cost in dollars
 * @returns Formatted cost string (e.g., "$0.0012" or "$1.23")
 */
export function formatCost(cost: number): string {
  if (cost === 0) return '$0.00';
  if (cost < 0.01) return `$${cost.toFixed(4)}`;
  return `$${cost.toFixed(2)}`;
}
