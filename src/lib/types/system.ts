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
