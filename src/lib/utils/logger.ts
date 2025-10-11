/**
 * MCP Studio Logging Utility
 * Wrapper around tauri-plugin-log for clean, scoped logging
 */

import { trace, debug, info, warn, error } from '@tauri-apps/plugin-log';
import { logStore } from '$lib/stores/logStore';

/**
 * Log context for structured logging
 */
export interface LogContext {
  [key: string]: unknown;
}

/**
 * Format a value for logging
 * Handles Error objects, structured data, and primitives
 */
function formatValue(value: unknown): string {
  if (value instanceof Error) {
    return value.stack || value.message;
  }
  if (typeof value === 'object' && value !== null) {
    try {
      return JSON.stringify(value);
    } catch {
      return String(value);
    }
  }
  return String(value);
}

/**
 * Scoped logger for component-specific logging
 * Automatically prefixes all log messages with the scope name
 */
export class ScopedLogger {
  constructor(private scopeName: string) {}

  private formatMessage(message: string, ...data: unknown[]): string {
    const prefix = `[${this.scopeName}]`;
    if (data.length === 0) {
      return `${prefix} ${message}`;
    }
    const formattedData = data.map(formatValue).join(' ');
    return `${prefix} ${message} ${formattedData}`;
  }

  trace(message: string, ...data: unknown[]): void {
    const formatted = this.formatMessage(message, ...data);
    trace(formatted);
    // Note: logStore doesn't have trace level, using debug instead
    logStore.debug(this.scopeName, message, data);
  }

  debug(message: string, ...data: unknown[]): void {
    const formatted = this.formatMessage(message, ...data);
    debug(formatted);
    logStore.debug(this.scopeName, message, data);
  }

  info(message: string, ...data: unknown[]): void {
    const formatted = this.formatMessage(message, ...data);
    info(formatted);
    logStore.info(this.scopeName, message, data);
  }

  warn(message: string, ...data: unknown[]): void {
    const formatted = this.formatMessage(message, ...data);
    warn(formatted);
    logStore.warn(this.scopeName, message, data);
  }

  error(message: string, ...data: unknown[]): void {
    const formatted = this.formatMessage(message, ...data);
    error(formatted);
    logStore.error(this.scopeName, message, data);
  }

  /**
   * Log an exception with stack trace
   */
  exception(message: string, err: Error, context?: LogContext): void {
    const errorContext: LogContext = {
      ...context,
      error: err.message,
      stack: err.stack
    };
    error(this.formatMessage(message, errorContext));
  }

  /**
   * Performance timing helpers
   */
  time(label: string): () => void {
    const startTime = performance.now();
    const scopedLabel = `${this.scopeName}:${label}`;

    return () => {
      const duration = performance.now() - startTime;
      this.debug(`${scopedLabel} completed in ${duration.toFixed(2)}ms`);
    };
  }
}

/**
 * Create a scoped logger for a component or module
 *
 * @example
 * const logger = createLogger('ServerManagement');
 * logger.info('Server connected', { serverId: '123' });
 */
export function createLogger(scopeName: string): ScopedLogger {
  return new ScopedLogger(scopeName);
}

/**
 * Global logger (use sparingly, prefer scoped loggers)
 */
export const logger = {
  trace,
  debug,
  info,
  warn,
  error,
  scope: createLogger
};
