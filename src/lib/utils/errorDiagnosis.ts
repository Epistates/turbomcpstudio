/**
 * Error Diagnosis Utility
 *
 * Provides intelligent error analysis with actionable suggestions
 * for sampling and elicitation failures.
 */

export interface DiagnosedError {
  originalError: string;
  category: 'server' | 'llm' | 'network' | 'validation' | 'protocol' | 'unknown';
  diagnosis: string;
  suggestions: string[];
  documentation?: string;
  technicalDetails?: Record<string, any>;
}

export interface ErrorContext {
  estimatedTokens?: number;
  maxContextWindow?: number;
  serverStatus?: string;
  hasApiKey?: boolean;
}

/**
 * Diagnose a sampling or elicitation error with helpful context
 */
export function diagnoseError(error: any, context: ErrorContext = {}): DiagnosedError {
  const errorStr = error?.toString().toLowerCase() || '';
  const errorMessage = error?.message || error?.toString() || 'Unknown error';

  // ========================================
  // SERVER CONNECTION ERRORS
  // ========================================

  if (errorStr.includes('server not found') ||
      errorStr.includes('no server') ||
      errorStr.includes('server id')) {
    return {
      originalError: errorMessage,
      category: 'server',
      diagnosis: 'MCP server not found or not selected',
      suggestions: [
        'Select a server from the dropdown',
        'Check that the server is configured in Settings',
        'Verify server ID is correct'
      ],
      documentation: '/docs/troubleshooting#server-not-found'
    };
  }

  if (errorStr.includes('not connected') ||
      errorStr.includes('disconnected') ||
      errorStr.includes('connection refused')) {
    return {
      originalError: errorMessage,
      category: 'server',
      diagnosis: 'MCP server is not connected',
      suggestions: [
        'Check server status in the sidebar (should show "Connected")',
        'Restart the server using the refresh button',
        'Verify server configuration is correct',
        'Check server logs for startup errors'
      ],
      documentation: '/docs/troubleshooting#server-connection',
      technicalDetails: {
        serverStatus: context.serverStatus || 'unknown'
      }
    };
  }

  if (errorStr.includes('server error') || errorStr.includes('internal error')) {
    return {
      originalError: errorMessage,
      category: 'server',
      diagnosis: 'MCP server encountered an internal error',
      suggestions: [
        'Check server logs for detailed error messages',
        'Try restarting the server',
        'Verify the server implementation is correct',
        'Report this to the server developer if it persists'
      ]
    };
  }

  // ========================================
  // LLM API ERRORS
  // ========================================

  if (errorStr.includes('api key') ||
      errorStr.includes('authentication') ||
      errorStr.includes('unauthorized') ||
      errorStr.includes('invalid key')) {
    return {
      originalError: errorMessage,
      category: 'llm',
      diagnosis: 'LLM provider authentication failed',
      suggestions: [
        'Check that your API key is correctly entered',
        'Verify the API key format (OpenAI: sk-..., Anthropic: sk-ant-...)',
        'Ensure API key has not expired',
        'Confirm API key has proper permissions',
        'Try regenerating your API key from the provider dashboard'
      ],
      documentation: '/docs/llm-setup#api-keys',
      technicalDetails: {
        hasApiKey: context.hasApiKey
      }
    };
  }

  if (errorStr.includes('rate limit') ||
      errorStr.includes('quota exceeded') ||
      errorStr.includes('too many requests')) {
    return {
      originalError: errorMessage,
      category: 'llm',
      diagnosis: 'LLM API rate limit exceeded',
      suggestions: [
        'Wait a few minutes before trying again',
        'Reduce request frequency',
        'Check your API usage dashboard',
        'Upgrade your API plan if you need higher limits',
        'Consider using a different model with lower traffic'
      ],
      documentation: '/docs/llm-setup#rate-limits'
    };
  }

  if (errorStr.includes('insufficient funds') ||
      errorStr.includes('billing') ||
      errorStr.includes('payment')) {
    return {
      originalError: errorMessage,
      category: 'llm',
      diagnosis: 'LLM API account has insufficient credits',
      suggestions: [
        'Check your billing status in the provider dashboard',
        'Add credits to your account',
        'Verify payment method is valid',
        'Contact provider support if billing is correct'
      ]
    };
  }

  if (errorStr.includes('model not found') ||
      errorStr.includes('invalid model') ||
      errorStr.includes('model does not exist')) {
    return {
      originalError: errorMessage,
      category: 'llm',
      diagnosis: 'Requested model is not available',
      suggestions: [
        'Check that the model name is spelled correctly',
        'Verify you have access to this model',
        'Some models require special API access',
        'Try using a different model (e.g., gpt-4o, claude-3-sonnet)'
      ]
    };
  }

  // ========================================
  // CONTEXT WINDOW / TOKEN ERRORS
  // ========================================

  if (errorStr.includes('context length') ||
      errorStr.includes('context window') ||
      errorStr.includes('too long') ||
      errorStr.includes('maximum context') ||
      errorStr.includes('token limit')) {
    const tokensInfo = context.estimatedTokens && context.maxContextWindow
      ? `\nCurrent: ${context.estimatedTokens.toLocaleString()} tokens, Max: ${context.maxContextWindow.toLocaleString()} tokens`
      : '';

    return {
      originalError: errorMessage,
      category: 'validation',
      diagnosis: 'Message exceeds model context window',
      suggestions: [
        'Reduce message length' + tokensInfo,
        'Clear conversation history to free up tokens',
        'Use a model with larger context window (e.g., Claude Opus 200K)',
        'Split your request into smaller chunks',
        'Remove unnecessary context from includeContext setting'
      ],
      documentation: '/docs/troubleshooting#context-window',
      technicalDetails: {
        estimatedTokens: context.estimatedTokens,
        maxContextWindow: context.maxContextWindow
      }
    };
  }

  // ========================================
  // VALIDATION ERRORS
  // ========================================

  if (errorStr.includes('invalid') ||
      errorStr.includes('validation failed') ||
      errorStr.includes('schema')) {
    return {
      originalError: errorMessage,
      category: 'validation',
      diagnosis: 'Request validation failed',
      suggestions: [
        'Check that all required fields are present',
        'Verify parameter types match expected schema',
        'Review the Protocol Inspector for detailed request structure',
        'Ensure message content is properly formatted',
        'Check for invalid characters in fields'
      ],
      documentation: '/docs/troubleshooting#validation-errors'
    };
  }

  if (errorStr.includes('empty') || errorStr.includes('no message')) {
    return {
      originalError: errorMessage,
      category: 'validation',
      diagnosis: 'Message content is empty',
      suggestions: [
        'Enter a message before sending',
        'Check that message field is not blank',
        'Verify conversation history contains valid messages'
      ]
    };
  }

  // ========================================
  // NETWORK ERRORS
  // ========================================

  if (errorStr.includes('timeout') || errorStr.includes('timed out')) {
    return {
      originalError: errorMessage,
      category: 'network',
      diagnosis: 'Request timed out',
      suggestions: [
        'Check your internet connection',
        'The LLM API may be experiencing high latency',
        'Try increasing timeout settings',
        'Use a faster model or reduce max tokens',
        'Retry the request in a few moments'
      ]
    };
  }

  if (errorStr.includes('network') ||
      errorStr.includes('connection failed') ||
      errorStr.includes('fetch failed')) {
    return {
      originalError: errorMessage,
      category: 'network',
      diagnosis: 'Network connection failed',
      suggestions: [
        'Check your internet connection',
        'Verify firewall is not blocking requests',
        'Check if LLM provider API is accessible',
        'Try disabling VPN if enabled',
        'Check proxy settings'
      ]
    };
  }

  // ========================================
  // PROTOCOL ERRORS
  // ========================================

  if (errorStr.includes('json-rpc') ||
      errorStr.includes('jsonrpc') ||
      errorStr.includes('invalid request')) {
    return {
      originalError: errorMessage,
      category: 'protocol',
      diagnosis: 'MCP protocol error - malformed request',
      suggestions: [
        'This is likely a bug in the application',
        'Check the Protocol Inspector for request structure',
        'Try a simpler request to isolate the issue',
        'Report this error if it persists'
      ],
      documentation: '/docs/troubleshooting#protocol-errors'
    };
  }

  if (errorStr.includes('method not found') ||
      errorStr.includes('not supported')) {
    return {
      originalError: errorMessage,
      category: 'protocol',
      diagnosis: 'MCP server does not support this operation',
      suggestions: [
        'Check that server supports sampling (look for sampling capability)',
        'Verify server implements the correct MCP version',
        'Some servers may not support all MCP features',
        'Contact server developer for capability information'
      ]
    };
  }

  // ========================================
  // ELICITATION-SPECIFIC ERRORS
  // ========================================

  if (errorStr.includes('elicitation') || errorStr.includes('form')) {
    return {
      originalError: errorMessage,
      category: 'validation',
      diagnosis: 'Elicitation request failed',
      suggestions: [
        'Check that schema is valid (use Schema Validator)',
        'Verify all required fields are present',
        'Ensure field types are primitives only (string, number, boolean)',
        'Review form field definitions for errors'
      ],
      documentation: '/docs/elicitation#schema-validation'
    };
  }

  // ========================================
  // UNKNOWN ERRORS
  // ========================================

  return {
    originalError: errorMessage,
    category: 'unknown',
    diagnosis: 'An unexpected error occurred',
    suggestions: [
      'Check browser console for detailed error logs (F12)',
      'Try again with a simpler request to isolate the issue',
      'Restart the application if the error persists',
      'Report this issue with error details if it continues',
      'Check MCP server logs for additional context'
    ],
    documentation: '/docs/troubleshooting#general'
  };
}

/**
 * Get a friendly category label
 */
export function getCategoryLabel(category: DiagnosedError['category']): string {
  const labels = {
    server: 'Server Error',
    llm: 'LLM API Error',
    network: 'Network Error',
    validation: 'Validation Error',
    protocol: 'Protocol Error',
    unknown: 'Unknown Error'
  };
  return labels[category] || 'Error';
}

/**
 * Get a color class for the error category
 */
export function getCategoryColor(category: DiagnosedError['category']): string {
  const colors = {
    server: 'text-orange-600 dark:text-orange-400 bg-orange-100 dark:bg-orange-900/30',
    llm: 'text-purple-600 dark:text-purple-400 bg-purple-100 dark:bg-purple-900/30',
    network: 'text-blue-600 dark:text-blue-400 bg-blue-100 dark:bg-blue-900/30',
    validation: 'text-yellow-600 dark:text-yellow-400 bg-yellow-100 dark:bg-yellow-900/30',
    protocol: 'text-red-600 dark:text-red-400 bg-red-100 dark:bg-red-900/30',
    unknown: 'text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-800'
  };
  return colors[category] || colors.unknown;
}

/**
 * Check if error is retryable
 */
export function isRetryable(diagnosis: DiagnosedError): boolean {
  // Retryable: network issues, rate limits, timeouts
  if (diagnosis.category === 'network') return true;

  const retryablePatterns = [
    'rate limit',
    'timeout',
    'temporary',
    'try again'
  ];

  return retryablePatterns.some(pattern =>
    diagnosis.diagnosis.toLowerCase().includes(pattern) ||
    diagnosis.suggestions.some(s => s.toLowerCase().includes(pattern))
  );
}
