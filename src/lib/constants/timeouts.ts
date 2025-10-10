/**
 * MCP Studio Timeout and Duration Constants
 * Centralized configuration for all timeout and delay values
 */

/**
 * Operation timeouts (in milliseconds)
 */
export const TIMEOUTS = {
  /**
   * Default timeout for most MCP operations (30 seconds)
   */
  DEFAULT_OPERATION: 30000,

  /**
   * Quick operations like connection tests (5 seconds)
   */
  QUICK_OPERATION: 5000,

  /**
   * Long-running operations like tool execution (60 seconds)
   */
  LONG_OPERATION: 60000,

  /**
   * Very short operations like health checks (2 seconds)
   */
  HEALTH_CHECK: 2000,

  /**
   * Server connection timeout (10 seconds)
   */
  SERVER_CONNECT: 10000,

  /**
   * Server disconnection timeout (5 seconds)
   */
  SERVER_DISCONNECT: 5000
} as const;

/**
 * UI interaction delays (in milliseconds)
 */
export const DELAYS = {
  /**
   * Debounce delay for search input (300ms)
   */
  SEARCH_DEBOUNCE: 300,

  /**
   * Debounce delay for form validation (500ms)
   */
  VALIDATION_DEBOUNCE: 500,

  /**
   * Toast notification duration (5 seconds)
   */
  TOAST_DURATION: 5000,

  /**
   * Error toast duration (7 seconds)
   */
  ERROR_TOAST_DURATION: 7000,

  /**
   * Success toast duration (3 seconds)
   */
  SUCCESS_TOAST_DURATION: 3000,

  /**
   * Short animation duration (150ms)
   */
  ANIMATION_SHORT: 150,

  /**
   * Standard animation duration (300ms)
   */
  ANIMATION_STANDARD: 300,

  /**
   * Long animation duration (500ms)
   */
  ANIMATION_LONG: 500,

  /**
   * Retry delay for failed operations (1 second)
   */
  RETRY_DELAY: 1000,

  /**
   * Polling interval for status checks (2 seconds)
   */
  POLLING_INTERVAL: 2000
} as const;

/**
 * Retry configuration
 */
export const RETRY = {
  /**
   * Maximum number of retry attempts
   */
  MAX_ATTEMPTS: 3,

  /**
   * Base delay for exponential backoff (1 second)
   */
  BASE_DELAY: 1000,

  /**
   * Maximum delay for exponential backoff (10 seconds)
   */
  MAX_DELAY: 10000
} as const;

/**
 * Helper function to calculate exponential backoff delay
 *
 * @param attempt - Current attempt number (0-indexed)
 * @returns Delay in milliseconds
 */
export function calculateBackoffDelay(attempt: number): number {
  const delay = RETRY.BASE_DELAY * Math.pow(2, attempt);
  return Math.min(delay, RETRY.MAX_DELAY);
}

/**
 * Type-safe timeout names
 */
export type TimeoutName = keyof typeof TIMEOUTS;
export type DelayName = keyof typeof DELAYS;
