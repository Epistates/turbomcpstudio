/**
 * MCP Protocol Parser - Testing Tool Edition
 *
 * TurboMCP 2.0 only supports MCP 2025-06-18 specification.
 * Legacy MCP 2024-11-05 support has been dropped.
 *
 * Philosophy: PERMISSIVE parsing with DETAILED observability
 * - Accept ANY response format
 * - Track conformance to MCP 2025-06-18
 * - Provide actionable feedback
 * - Never block or throw errors
 */

export interface ConformanceIssue {
  severity: 'error' | 'warning' | 'info';
  field?: string;
  message: string;
  expected?: string;
  received?: string;
}

export interface ConformanceReport {
  version: 'MCP-2025-06-18' | 'unknown';
  isCompliant: boolean;
  issues: ConformanceIssue[];
}

export interface ParseResult<T = unknown> {
  /** Raw response exactly as received */
  raw: any;
  /** Parsed result if successful */
  parsed?: T;
  /** Conformance analysis */
  conformance: ConformanceReport;
}

/**
 * MCP 2025-06-18 ContentBlock type
 * Full spec-compliant definition
 */
export interface ContentBlock {
  type: 'text' | 'image' | 'audio' | 'resource_link' | 'resource';
  // Text content fields
  text?: string;
  // Image/Audio content fields
  data?: string;
  mimeType?: string;
  // Resource link fields
  uri?: string;
  name?: string;
  title?: string;
  description?: string;
  // Embedded resource fields
  resource?: {
    uri: string;
    mimeType?: string;
    text?: string;
    blob?: string;
  };
  // Common optional fields
  annotations?: Record<string, unknown>;
  _meta?: Record<string, unknown>;
}

/**
 * MCP 2025-06-18 CallToolResult type
 */
export interface CallToolResult {
  content: ContentBlock[];
  isError?: boolean;
  structuredContent?: Record<string, any>;
  _meta?: Record<string, unknown>;
}

/**
 * Parse a tool call response with full conformance tracking
 */
/**
 * Parse a tool call response with MCP 2025-06-18 conformance tracking
 *
 * TurboMCP 2.0 only supports MCP 2025-06-18 - no legacy format support.
 */
export function parseToolResult(response: any): ParseResult<CallToolResult> {
  const issues: ConformanceIssue[] = [];
  let version: 'MCP-2025-06-18' | 'unknown' = 'unknown';
  let parsed: CallToolResult | undefined;

  // Check if response is an object
  if (!response || typeof response !== 'object') {
    issues.push({
      severity: 'error',
      message: 'Response is not an object',
      expected: 'Object with content array',
      received: typeof response
    });
    return {
      raw: response,
      conformance: { version: 'unknown', isCompliant: false, issues }
    };
  }

  // MCP 2025-06-18 format detection
  if ('content' in response && Array.isArray(response.content)) {
    version = 'MCP-2025-06-18';

    // Validate content array
    if (response.content.length === 0) {
      issues.push({
        severity: 'warning',
        field: 'content',
        message: 'Content array is empty',
        expected: 'At least one ContentBlock'
      });
    }

    // Validate content blocks
    response.content.forEach((block: any, index: number) => {
      if (!block.type) {
        issues.push({
          severity: 'error',
          field: `content[${index}]`,
          message: 'ContentBlock missing required "type" field',
          expected: 'type: "text" | "image" | "audio" | "resource_link" | "resource"'
        });
      } else if (!['text', 'image', 'audio', 'resource_link', 'resource'].includes(block.type)) {
        issues.push({
          severity: 'error',
          field: `content[${index}].type`,
          message: `Unknown ContentBlock type: ${block.type}`,
          expected: 'type: "text" | "image" | "audio" | "resource_link" | "resource"',
          received: block.type
        });
      }

      // Type-specific validation
      if (block.type === 'text' && !block.text) {
        issues.push({
          severity: 'error',
          field: `content[${index}].text`,
          message: 'Text ContentBlock missing "text" field'
        });
      }
      if ((block.type === 'image' || block.type === 'audio') && (!block.data || !block.mimeType)) {
        issues.push({
          severity: 'error',
          field: `content[${index}]`,
          message: `${block.type} ContentBlock missing required "data" or "mimeType" field`
        });
      }
      if (block.type === 'resource_link' && !block.uri) {
        issues.push({
          severity: 'error',
          field: `content[${index}].uri`,
          message: 'Resource link ContentBlock missing "uri" field'
        });
      }
    });

    // Check isError field (should be camelCase, not snake_case)
    if ('is_error' in response) {
      issues.push({
        severity: 'warning',
        field: 'is_error',
        message: 'Field uses snake_case instead of camelCase',
        expected: 'isError (camelCase)',
        received: 'is_error (snake_case)'
      });
    }

    // Successfully parsed as MCP 2025-06-18
    parsed = {
      content: response.content,
      isError: response.isError ?? response.is_error,
      structuredContent: response.structuredContent,
      _meta: response._meta
    };
  }
  // Unknown format
  else {
    issues.push({
      severity: 'error',
      message: 'Response does not match MCP 2025-06-18 format',
      expected: 'MCP 2025-06-18: {content: [{type: "text", text: "..."}], isError?: boolean}'
    });
  }

  const isCompliant = version === 'MCP-2025-06-18' &&
                      issues.filter(i => i.severity === 'error').length === 0;

  return {
    raw: response,
    parsed,
    conformance: {
      version,
      isCompliant,
      issues
    }
  };
}

/**
 * Get a human-readable summary of conformance issues
 */
/**
 * Get a human-readable summary of conformance issues
 */
export function getConformanceSummary(report: ConformanceReport): string {
  const errorCount = report.issues.filter(i => i.severity === 'error').length;
  const warningCount = report.issues.filter(i => i.severity === 'warning').length;

  if (report.isCompliant) {
    return 'Fully compliant with MCP 2025-06-18';
  }

  if (errorCount > 0) {
    return `${errorCount} error${errorCount > 1 ? 's' : ''}, ${warningCount} warning${warningCount > 1 ? 's' : ''}`;
  }

  if (warningCount > 0) {
    return `${warningCount} warning${warningCount > 1 ? 's' : ''}`;
  }

  return 'Unknown format';
}

/**
 * Type guard for CallToolResult
 * Non-blocking - returns boolean but doesn't throw
 */
export function isCallToolResult(value: unknown): value is CallToolResult {
  if (!value || typeof value !== 'object') return false;
  return 'content' in value && Array.isArray((value as any).content);
}
