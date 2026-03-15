/**
 * System Information Types
 *
 * Types for OS and system information retrieved from the backend
 */

/**
 * System information including OS details
 */
export interface SystemInfo {
  /** Operating system identifier (e.g., 'linux', 'windows', 'macos') */
  os: string;

  /** OS family classification (e.g., 'unix', 'windows') */
  family: string;

  /** CPU/system architecture (e.g., 'x86_64', 'aarch64', 'arm') */
  arch: string;

  /** OS version string */
  version: string;

  /** System locale/language (e.g., 'en-US', 'de-DE') */
  locale: string;
}

/**
 * Helper type for update checking results
 */
export interface UpdateInfo {
  available: boolean;
  version: string;
  date?: string;
  body?: string;
}

/**
 * App state information for diagnostics
 */
export interface AppDiagnostics {
  system: SystemInfo;
  appVersion: string;
  appId: string;
  appDataDir: string;
}

/**
 * MCP Event types from the backend
 */
export interface ConnectionMetrics {
  connected_at?: string;
  requests_sent: number;
  responses_received: number;
  avg_response_time_ms: number;
  error_count: number;
  last_error?: string;
  bytes_sent: number;
  bytes_received: number;
  uptime_seconds: number;
}

export interface ProcessInfo {
  pid: number;
  command: string;
  args: string[];
  started_at: string;
  cpu_usage: number;
  memory_usage: number;
  status: 'running' | 'stopped' | 'crashed';
}

export interface ServerCapabilities {
  tools?: { list_changed?: boolean };
  resources?: { subscribe?: boolean; list_changed?: boolean };
  prompts?: { list_changed?: boolean };
  sampling?: Record<string, unknown>;
  elicitation?: Record<string, unknown>;
}

export type McpEvent =
  | { StatusChanged: { server_id: string; status: string } }
  | { CapabilitiesUpdated: { server_id: string; capabilities: ServerCapabilities } }
  | { MetricsUpdated: { server_id: string; metrics: ConnectionMetrics } }
  | { ProcessUpdated: { server_id: string; process_info: ProcessInfo } }
  | { Error: { server_id: string; error: string } };
