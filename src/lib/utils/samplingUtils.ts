/**
 * MCP Studio Sampling Utilities
 *
 * Utility functions for sampling workbench including:
 * - Message content formatting
 * - Status color mapping
 * - Model hint labels
 * - Capability error analysis
 * - LLM response validation
 * - Retry logic with exponential backoff
 * - Clipboard operations
 */

import { RETRY } from '$lib/constants';
import type { ModelPreferences } from '$lib/types/sampling';

// ========================================
// FORMATTING UTILITIES
// ========================================

/**
 * Format message content for display
 * Handles various content formats (string, object with text, etc.)
 */
export function formatMessageContent(content: any): string {
  if (typeof content === 'string') return content;
  if (content?.text) return content.text;
  return JSON.stringify(content, null, 2);
}

/**
 * Get Tailwind CSS classes for status badges
 */
export function getStatusColor(status: string): string {
  const colors = {
    pending:
      'text-orange-700 bg-orange-100 border-orange-200 dark:text-orange-300 dark:bg-orange-900/30 dark:border-orange-700',
    approved:
      'text-blue-700 bg-blue-100 border-blue-200 dark:text-blue-300 dark:bg-blue-900/30 dark:border-blue-700',
    completed:
      'text-green-700 bg-green-100 border-green-200 dark:text-green-300 dark:bg-green-900/30 dark:border-green-700',
    rejected:
      'text-red-700 bg-red-100 border-red-200 dark:text-red-300 dark:bg-red-900/30 dark:border-red-700',
    error:
      'text-red-700 bg-red-100 border-red-200 dark:text-red-300 dark:bg-red-900/30 dark:border-red-700',
    default:
      'text-gray-700 bg-gray-100 border-gray-200 dark:text-gray-300 dark:bg-gray-700/30 dark:border-gray-600'
  };
  return colors[status as keyof typeof colors] || colors.default;
}

/**
 * Get human-readable model hint label
 */
export function getModelHintLabel(preferences?: ModelPreferences): string {
  if (!preferences?.hints?.[0]?.name) return 'Human in the Loop';
  return preferences.hints[0].name;
}

// ========================================
// ERROR ANALYSIS
// ========================================

export interface CapabilityError {
  isCapabilityError: boolean;
  explanation: string;
  solution: string;
}

/**
 * Analyze error messages for MCP capability issues
 * Provides actionable guidance for common capability errors
 */
export function analyzeCapabilityError(error: string): CapabilityError {
  const errorLower = error.toLowerCase();

  if (errorLower.includes('server capabilities not available for sampling')) {
    return {
      isCapabilityError: true,
      explanation:
        'The server-side tool is incorrectly checking for "server capabilities" for sampling. Sampling is a CLIENT capability that MCP Studio provides.',
      solution:
        "This is likely a bug in the server's tool implementation. Try different tools or contact the server developer."
    };
  }

  if (
    errorLower.includes('sampling') &&
    (errorLower.includes('not supported') || errorLower.includes('not available'))
  ) {
    return {
      isCapabilityError: true,
      explanation: 'Sampling capability issue detected.',
      solution:
        'MCP Studio advertises sampling capability correctly. Check the server-side tool implementation.'
    };
  }

  return { isCapabilityError: false, explanation: '', solution: '' };
}

// ========================================
// VALIDATION
// ========================================

export interface ValidationResult {
  valid: boolean;
  reason?: string;
}

/**
 * Validate LLM response for common issues
 * Helps diagnose empty responses, token limits, etc.
 */
export function validateLLMResponse(result: any): ValidationResult {
  if (!result) {
    return { valid: false, reason: 'Empty or null response from LLM' };
  }

  if (result.status === 'error') {
    return { valid: false, reason: result.message || 'LLM returned error status' };
  }

  const content = result.content || result.text || '';
  if (!content || content.trim().length === 0) {
    return {
      valid: false,
      reason:
        'LLM returned empty response. This often happens when max_tokens is too low or the model hits a token limit. Try increasing max_tokens in the request.'
    };
  }

  if (result.stop_reason === 'length' && content.trim().length === 0) {
    return {
      valid: false,
      reason:
        'LLM hit token limit before generating any content. Increase max_tokens or check your system prompt length.'
    };
  }

  return { valid: true };
}

// ========================================
// RETRY LOGIC
// ========================================

/**
 * Retry operation with exponential backoff and jitter
 *
 * @param operation - Async operation to retry
 * @param maxRetries - Maximum number of retries (default from RETRY.MAX_ATTEMPTS)
 * @param initialDelayMs - Initial delay in milliseconds (default from RETRY.INITIAL_DELAY)
 * @returns Result of successful operation
 * @throws Last error if all retries exhausted or non-retryable error
 */
export async function retryWithExponentialBackoff<T>(
  operation: () => Promise<T>,
  maxRetries: number = RETRY.MAX_ATTEMPTS,
  initialDelayMs: number = RETRY.INITIAL_DELAY
): Promise<T> {
  let lastError: Error;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await operation();
    } catch (error) {
      lastError = error as Error;

      // Don't retry on specific error types
      if (attempt === maxRetries || isNonRetryableError(error)) {
        throw error;
      }

      // Calculate delay with exponential backoff and jitter
      const delay = initialDelayMs * Math.pow(2, attempt);
      const jitter = Math.random() * RETRY.JITTER_PERCENT * delay;

      console.log(
        `Request failed, retrying in ${Math.round(delay + jitter)}ms... (attempt ${attempt + 1}/${maxRetries + 1})`
      );
      await new Promise((resolve) => setTimeout(resolve, delay + jitter));
    }
  }

  throw lastError!;
}

/**
 * Determine if error is non-retryable (auth, validation, etc.)
 */
function isNonRetryableError(error: any): boolean {
  const errorMessage = error?.toString().toLowerCase() || '';
  return (
    errorMessage.includes('api key') ||
    errorMessage.includes('unauthorized') ||
    errorMessage.includes('invalid') ||
    errorMessage.includes('not found')
  );
}

// ========================================
// CLIPBOARD OPERATIONS
// ========================================

/**
 * Copy text to clipboard using navigator.clipboard API
 *
 * @param text - Text to copy
 * @throws Error if clipboard API not available
 */
export async function copyToClipboard(text: string): Promise<void> {
  if (!navigator.clipboard) {
    throw new Error('Clipboard API not available');
  }
  await navigator.clipboard.writeText(text);
}
