/**
 * Benchmark Store - State management for TurboMCP benchmark sessions
 *
 * Provides reactive state and methods for:
 * - Starting / stopping benchmark sessions tied to a proxy
 * - Polling live call records during an active session
 * - Generating and comparing benchmark reports
 * - Persisting reports in-memory for comparison workflows
 */

import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// ---------------------------------------------------------------------------
// Types (mirror Rust structs from src-tauri/src/proxy/benchmark.rs)
// ---------------------------------------------------------------------------

export interface CallRecord {
  backend: string;
  method: string;
  tool_name?: string;
  resource_uri?: string;
  prompt_name?: string;
  /** Latency in microseconds */
  latency_us: number;
  request_bytes: number;
  response_bytes: number;
  estimated_input_tokens: number;
  estimated_output_tokens: number;
  success: boolean;
  error_message?: string;
  /** ISO-8601 timestamp */
  timestamp: string;
}

export interface LatencyStats {
  count: number;
  min_ms: number;
  max_ms: number;
  mean_ms: number;
  p50_ms: number;
  p95_ms: number;
  p99_ms: number;
  std_dev_ms: number;
}

export interface ToolSummary {
  name: string;
  call_count: number;
  success_count: number;
  failure_count: number;
  total_input_bytes: number;
  total_output_bytes: number;
  estimated_input_tokens: number;
  estimated_output_tokens: number;
  latency?: LatencyStats;
}

export interface MethodSummary {
  method: string;
  call_count: number;
  total_input_bytes: number;
  total_output_bytes: number;
  estimated_input_tokens: number;
  estimated_output_tokens: number;
  latency?: LatencyStats;
}

export interface BackendSummary {
  name: string;
  total_calls: number;
  total_tool_calls: number;
  success_count: number;
  failure_count: number;
  success_rate: number;
  total_input_bytes: number;
  total_output_bytes: number;
  total_bytes: number;
  estimated_input_tokens: number;
  estimated_output_tokens: number;
  estimated_total_tokens: number;
  overall_latency?: LatencyStats;
  tool_call_latency?: LatencyStats;
  tools: ToolSummary[];
  methods: MethodSummary[];
}

export interface BenchmarkReport {
  session_id: string;
  started_at: string;
  ended_at: string;
  duration_secs: number;
  backends: BackendSummary[];
  records: CallRecord[];
}

export interface BenchmarkSession {
  id: string;
  name: string;
  backend_name: string;
  started_at: string;
  active: boolean;
}

export interface LatencyDelta {
  delta_ms: number;
  delta_pct: number | null;
}

export interface BackendLatencyComparison {
  backend_name: string;
  p50: LatencyDelta;
  p95: LatencyDelta;
  p99: LatencyDelta;
  mean: LatencyDelta;
}

export interface ThroughputComparison {
  backend_name: string;
  bytes_per_sec_a: number;
  bytes_per_sec_b: number;
  delta_pct: number | null;
}

export interface SuccessRateComparison {
  backend_name: string;
  success_rate_a: number;
  success_rate_b: number;
  delta_pct_points: number;
}

export interface ReportComparison {
  session_id_a: string;
  session_id_b: string;
  latency: BackendLatencyComparison[];
  throughput: ThroughputComparison[];
  success_rates: SuccessRateComparison[];
}

// ---------------------------------------------------------------------------
// Store state
// ---------------------------------------------------------------------------

interface BenchmarkStoreState {
  sessions: BenchmarkSession[];
  activeSessionId: string | null;
  currentReport: BenchmarkReport | null;
  liveRecords: CallRecord[];
  comparison: ReportComparison | null;
  savedReports: BenchmarkReport[];
  loading: boolean;
  error: string | null;
}

// ---------------------------------------------------------------------------
// Store factory
// ---------------------------------------------------------------------------

function createBenchmarkStore() {
  const initialState: BenchmarkStoreState = {
    sessions: [],
    activeSessionId: null,
    currentReport: null,
    liveRecords: [],
    comparison: null,
    savedReports: [],
    loading: false,
    error: null,
  };

  const { subscribe, set, update } = writable<BenchmarkStoreState>(initialState);

  return {
    subscribe,

    /**
     * Start a new benchmark session for the given proxy.
     * Returns the session ID that must be used for subsequent calls.
     */
    async startSession(proxyId: string, name?: string): Promise<string> {
      update((s) => ({ ...s, loading: true, error: null }));
      try {
        const sessionId = (await invoke('start_benchmark_session', {
          proxy_id: proxyId,
          session_name: name ?? null,
        })) as string;

        // Refresh the session list so the new session appears immediately.
        await this.loadSessions();

        update((s) => ({ ...s, activeSessionId: sessionId, loading: false }));
        return sessionId;
      } catch (err) {
        const error = err instanceof Error ? err.message : String(err);
        update((s) => ({ ...s, error, loading: false }));
        throw err;
      }
    },

    /**
     * Stop an active session and store the resulting report.
     * Also saves the report into `savedReports` for later comparison.
     */
    async stopSession(sessionId: string): Promise<BenchmarkReport> {
      update((s) => ({ ...s, loading: true, error: null }));
      try {
        const report = (await invoke('stop_benchmark_session', {
          session_id: sessionId,
        })) as BenchmarkReport;

        update((s) => ({
          ...s,
          currentReport: report,
          activeSessionId: s.activeSessionId === sessionId ? null : s.activeSessionId,
          savedReports: [report, ...s.savedReports].slice(0, 20), // keep last 20
          loading: false,
        }));

        // Refresh the session list to reflect the stopped state.
        await this.loadSessions();
        return report;
      } catch (err) {
        const error = err instanceof Error ? err.message : String(err);
        update((s) => ({ ...s, error, loading: false }));
        throw err;
      }
    },

    /** Refresh the list of all known sessions (active and stopped). */
    async loadSessions(): Promise<void> {
      try {
        const sessions = (await invoke('list_benchmark_sessions')) as BenchmarkSession[];
        update((s) => ({ ...s, sessions }));
      } catch (err) {
        const error = err instanceof Error ? err.message : String(err);
        update((s) => ({ ...s, error }));
      }
    },

    /**
     * Fetch the current call records for a session without stopping it.
     * Stores them in `liveRecords` (capped at 100 for the live feed view).
     */
    async getLiveRecords(sessionId: string): Promise<CallRecord[]> {
      try {
        const records = (await invoke('get_benchmark_records', {
          session_id: sessionId,
        })) as CallRecord[];

        update((s) => ({
          ...s,
          liveRecords: records.slice(-100),
        }));

        return records;
      } catch (err) {
        const error = err instanceof Error ? err.message : String(err);
        update((s) => ({ ...s, error }));
        throw err;
      }
    },

    /**
     * Generate a full benchmark report for a session without stopping it.
     * Stores the result in `currentReport`.
     */
    async getReport(sessionId: string): Promise<BenchmarkReport> {
      update((s) => ({ ...s, loading: true, error: null }));
      try {
        const report = (await invoke('get_benchmark_report', {
          session_id: sessionId,
        })) as BenchmarkReport;

        update((s) => ({ ...s, currentReport: report, loading: false }));
        return report;
      } catch (err) {
        const error = err instanceof Error ? err.message : String(err);
        update((s) => ({ ...s, error, loading: false }));
        throw err;
      }
    },

    /**
     * Compare two reports. The results are stored in `comparison`.
     * Pass the full report objects so the backend can compute deltas without
     * requiring them to still be held in manager state.
     */
    async compareReports(
      reportA: BenchmarkReport,
      reportB: BenchmarkReport
    ): Promise<ReportComparison> {
      update((s) => ({ ...s, loading: true, error: null }));
      try {
        const comparison = (await invoke('compare_benchmark_reports', {
          report_a: reportA,
          report_b: reportB,
        })) as ReportComparison;

        update((s) => ({ ...s, comparison, loading: false }));
        return comparison;
      } catch (err) {
        const error = err instanceof Error ? err.message : String(err);
        update((s) => ({ ...s, error, loading: false }));
        throw err;
      }
    },

    /** Persist a report into the `savedReports` list (in-memory only). */
    saveReport(report: BenchmarkReport): void {
      update((s) => {
        // Avoid duplicates by session_id.
        const exists = s.savedReports.some((r) => r.session_id === report.session_id);
        if (exists) return s;
        return {
          ...s,
          savedReports: [report, ...s.savedReports].slice(0, 20),
        };
      });
    },

    /** Clear the current comparison result. */
    clearComparison(): void {
      update((s) => ({ ...s, comparison: null }));
    },

    /** Set which session is considered "active" in the UI. */
    setActiveSession(sessionId: string | null): void {
      update((s) => ({ ...s, activeSessionId: sessionId }));
    },

    /** Clear any stored error message. */
    clearError(): void {
      update((s) => ({ ...s, error: null }));
    },

    /** Reset to initial state (e.g. when navigating away). */
    reset(): void {
      set(initialState);
    },
  };
}

// ---------------------------------------------------------------------------
// Singleton store instance
// ---------------------------------------------------------------------------

export const benchmarkStore = createBenchmarkStore();

// ---------------------------------------------------------------------------
// Derived convenience stores
// ---------------------------------------------------------------------------

export const activeSessions = derived(benchmarkStore, ($s) =>
  $s.sessions.filter((sess) => sess.active)
);

export const completedReports = derived(benchmarkStore, ($s) => $s.savedReports);

export const benchmarkLoading = derived(benchmarkStore, ($s) => $s.loading);

export const benchmarkError = derived(benchmarkStore, ($s) => $s.error);
