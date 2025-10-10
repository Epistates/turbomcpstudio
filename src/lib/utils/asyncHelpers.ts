/**
 * Async Utilities - Comprehensive async operation management
 *
 * Features:
 * - Operation timeouts with cleanup
 * - Request deduplication
 * - Cancellation support
 * - Retry logic
 */

/**
 * Wraps a promise or async function with a timeout
 * @param promiseOrFn - The promise or async function to wrap
 * @param timeoutMs - Timeout in milliseconds
 * @param timeoutMessage - Custom timeout message
 * @returns Promise that rejects if timeout is reached
 */
export async function withTimeout<T>(
  promiseOrFn: Promise<T> | (() => Promise<T>),
  timeoutMs: number,
  timeoutMessage?: string
): Promise<T> {
  let timeoutId: number;

  const timeoutPromise = new Promise<never>((_, reject) => {
    timeoutId = window.setTimeout(() => {
      reject(new Error(timeoutMessage || `Operation timed out after ${timeoutMs}ms`));
    }, timeoutMs);
  });

  try {
    // ✅ FIXED: Handle both Promise and function
    const actualPromise = typeof promiseOrFn === 'function' ? promiseOrFn() : promiseOrFn;
    const result = await Promise.race([actualPromise, timeoutPromise]);
    clearTimeout(timeoutId!);
    return result;
  } catch (error) {
    clearTimeout(timeoutId!);
    throw error;
  }
}

/**
 * Request deduplication manager
 * Ensures only one instance of an operation runs at a time
 */
export class RequestManager {
  private activeRequests = new Map<string, { id: string; promise: Promise<any> }>();

  /**
   * Execute a request with deduplication
   * @param key - Unique key for the request type
   * @param operation - The async operation to execute
   * @param timeoutMs - Optional timeout (default 30s)
   * @returns The result of the operation
   */
  async execute<T>(
    key: string,
    operation: () => Promise<T>,
    timeoutMs = 30000
  ): Promise<T> {
    // Check if request is already in progress
    const existing = this.activeRequests.get(key);
    if (existing) {
      console.warn(`⚠️ Request "${key}" already in progress, returning existing promise`);
      return existing.promise as Promise<T>;
    }

    // Create new request
    const requestId = crypto.randomUUID();
    const promise = withTimeout(operation(), timeoutMs, `Request "${key}" timed out`)
      .finally(() => {
        // Cleanup on completion
        const current = this.activeRequests.get(key);
        if (current?.id === requestId) {
          this.activeRequests.delete(key);
        }
      });

    this.activeRequests.set(key, { id: requestId, promise });

    return promise;
  }

  /**
   * Check if a request is currently active
   */
  isActive(key: string): boolean {
    return this.activeRequests.has(key);
  }

  /**
   * Cancel all active requests
   */
  cancelAll(): void {
    this.activeRequests.clear();
  }

  /**
   * Cancel a specific request
   */
  cancel(key: string): void {
    this.activeRequests.delete(key);
  }

  /**
   * Get active request count
   */
  getActiveCount(): number {
    return this.activeRequests.size;
  }
}

/**
 * Creates an AbortController with timeout
 */
export function createAbortController(timeoutMs?: number): AbortController {
  const controller = new AbortController();

  if (timeoutMs) {
    setTimeout(() => {
      controller.abort(new Error(`Operation aborted after ${timeoutMs}ms`));
    }, timeoutMs);
  }

  return controller;
}

/**
 * Retry an operation with exponential backoff
 * @param operation - The async operation to retry
 * @param maxRetries - Maximum number of retries (default 3)
 * @param baseDelayMs - Base delay in milliseconds (default 1000)
 * @returns The result of the operation
 */
export async function withRetry<T>(
  operation: () => Promise<T>,
  maxRetries = 3,
  baseDelayMs = 1000
): Promise<T> {
  let lastError: Error | unknown;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await operation();
    } catch (error) {
      lastError = error;

      if (attempt < maxRetries) {
        // Exponential backoff: 1s, 2s, 4s, ...
        const delayMs = baseDelayMs * Math.pow(2, attempt);
        console.warn(`⚠️ Attempt ${attempt + 1} failed, retrying in ${delayMs}ms...`);
        await new Promise(resolve => setTimeout(resolve, delayMs));
      }
    }
  }

  throw lastError;
}

/**
 * Debounce an async function
 * @param func - The async function to debounce
 * @param waitMs - Wait time in milliseconds
 * @returns Debounced function
 */
export function debounceAsync<T extends (...args: any[]) => Promise<any>>(
  func: T,
  waitMs: number
): (...args: Parameters<T>) => Promise<ReturnType<T>> {
  let timeoutId: number | null = null;
  let latestResolve: ((value: any) => void) | null = null;
  let latestReject: ((error: any) => void) | null = null;

  return (...args: Parameters<T>): Promise<ReturnType<T>> => {
    // Cancel previous timeout
    if (timeoutId !== null) {
      clearTimeout(timeoutId);
      timeoutId = null;
    }

    // Reject previous promise
    if (latestReject) {
      latestReject(new Error('Debounced call cancelled'));
      latestReject = null;
      latestResolve = null;
    }

    return new Promise<ReturnType<T>>((resolve, reject) => {
      latestResolve = resolve;
      latestReject = reject;

      timeoutId = window.setTimeout(async () => {
        try {
          const result = await func(...args);
          if (latestResolve) {
            latestResolve(result);
          }
        } catch (error) {
          if (latestReject) {
            latestReject(error);
          }
        } finally {
          latestResolve = null;
          latestReject = null;
          timeoutId = null;
        }
      }, waitMs);
    });
  };
}

/**
 * Global request manager instance
 */
export const globalRequestManager = new RequestManager();
