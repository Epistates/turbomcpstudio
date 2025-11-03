/**
 * Proxy Store - State management for MCP proxy creation and management
 *
 * Provides reactive state and methods for:
 * - Creating, starting, stopping proxies
 * - Listing and monitoring proxies
 * - Real-time metrics and status updates
 * - Server introspection for capability discovery
 */

import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// Types
export interface ProxyId {
  value: string;
}

export interface ProxyConfig {
  id: ProxyId;
  name: string;
  description?: string;
  running: boolean;
  frontend_url?: string;
  created_at: number;
  frontend_type: 'http' | 'websocket' | 'tcp';
  uptime_seconds?: number;
  total_requests?: number;
  error_count?: number;
  avg_latency_ms?: number;
}

export interface ProxyMetrics {
  total_requests: number;
  error_count: number;
  p50_latency_ms: number;
  p95_latency_ms: number;
  p99_latency_ms: number;
  uptime_seconds: number;
}

export interface ProxyStatus {
  id: ProxyId;
  name: string;
  running: boolean;
  frontend_url?: string;
  uptime_seconds?: number;
  total_requests?: number;
  error_count?: number;
  avg_latency_ms?: number;
  last_error?: string;
}

export interface ServerSpec {
  name: string;
  version?: string;
  tools: Array<{
    name: string;
    description?: string;
    input_schema?: Record<string, unknown>;
  }>;
  resources: Array<{
    uri: string;
    description?: string;
    mime_type?: string;
  }>;
  prompts: Array<{
    name: string;
    description?: string;
  }>;
}

export interface CreateProxyInput {
  name: string;
  description?: string;
  backend_type: string;
  backend_config: Record<string, unknown>;
  frontend_type: string;
}

// Store state
interface ProxyStoreState {
  proxies: ProxyConfig[];
  activeProxyId: string | null;
  loading: boolean;
  error: string | null;
  metrics: Map<string, ProxyMetrics>;
}

// Create the main store
function createProxyStore() {
  const initialState: ProxyStoreState = {
    proxies: [],
    activeProxyId: null,
    loading: false,
    error: null,
    metrics: new Map(),
  };

  const { subscribe, set, update } = writable<ProxyStoreState>(initialState);

  return {
    subscribe,

    // Load all proxies from database
    async loadProxies() {
      update((state) => ({ ...state, loading: true, error: null }));
      try {
        const proxies = (await invoke('list_proxies')) as ProxyConfig[];
        update((state) => ({
          ...state,
          proxies,
          loading: false,
        }));
      } catch (err) {
        const error = err instanceof Error ? err.message : 'Failed to load proxies';
        update((state) => ({ ...state, error, loading: false }));
      }
    },

    // Create a new proxy configuration
    async createProxy(input: CreateProxyInput): Promise<string> {
      update((state) => ({ ...state, loading: true, error: null }));
      try {
        const proxyId = (await invoke('create_proxy', {
          name: input.name,
          description: input.description,
          backend_type: input.backend_type,
          backend_config: input.backend_config,
          frontend_type: input.frontend_type,
        })) as string;

        // Reload proxies to get updated list
        await this.loadProxies();
        return proxyId;
      } catch (err) {
        const error = err instanceof Error ? err.message : 'Failed to create proxy';
        update((state) => ({ ...state, error, loading: false }));
        throw err;
      }
    },

    // Start a proxy
    async startProxy(proxyId: string) {
      update((state) => ({ ...state, loading: true, error: null }));
      try {
        const status = (await invoke('start_proxy', { proxy_id: proxyId })) as ProxyStatus;
        update((state) => ({
          ...state,
          proxies: state.proxies.map((p) =>
            p.id.value === proxyId
              ? { ...p, running: true, frontend_url: status.frontend_url }
              : p
          ),
          loading: false,
        }));
        return status;
      } catch (err) {
        const error = err instanceof Error ? err.message : 'Failed to start proxy';
        update((state) => ({ ...state, error, loading: false }));
        throw err;
      }
    },

    // Stop a proxy
    async stopProxy(proxyId: string) {
      update((state) => ({ ...state, loading: true, error: null }));
      try {
        await invoke('stop_proxy', { proxy_id: proxyId });
        update((state) => ({
          ...state,
          proxies: state.proxies.map((p) =>
            p.id.value === proxyId ? { ...p, running: false, frontend_url: undefined } : p
          ),
          loading: false,
        }));
      } catch (err) {
        const error = err instanceof Error ? err.message : 'Failed to stop proxy';
        update((state) => ({ ...state, error, loading: false }));
        throw err;
      }
    },

    // Delete a proxy
    async deleteProxy(proxyId: string) {
      update((state) => ({ ...state, loading: true, error: null }));
      try {
        await invoke('delete_proxy', { proxy_id: proxyId });
        update((state) => ({
          ...state,
          proxies: state.proxies.filter((p) => p.id.value !== proxyId),
          activeProxyId:
            state.activeProxyId === proxyId ? null : state.activeProxyId,
          loading: false,
        }));
      } catch (err) {
        const error = err instanceof Error ? err.message : 'Failed to delete proxy';
        update((state) => ({ ...state, error, loading: false }));
        throw err;
      }
    },

    // Get current status of a proxy
    async getProxyStatus(proxyId: string): Promise<ProxyStatus> {
      try {
        return (await invoke('get_proxy_status', {
          proxy_id: proxyId,
        })) as ProxyStatus;
      } catch (err) {
        const error = err instanceof Error ? err.message : 'Failed to get proxy status';
        update((state) => ({ ...state, error }));
        throw err;
      }
    },

    // Get live metrics for a proxy
    async getProxyMetrics(proxyId: string): Promise<ProxyMetrics> {
      try {
        const metrics = (await invoke('get_proxy_metrics', {
          proxy_id: proxyId,
        })) as ProxyMetrics;

        update((state) => {
          const newMetrics = new Map(state.metrics);
          newMetrics.set(proxyId, metrics);
          return { ...state, metrics: newMetrics };
        });

        return metrics;
      } catch (err) {
        const error = err instanceof Error ? err.message : 'Failed to get metrics';
        update((state) => ({ ...state, error }));
        throw err;
      }
    },

    // Introspect a backend to discover capabilities
    async introspectBackend(
      backend_type: string,
      backend_config: Record<string, unknown>,
      timeout_seconds?: number
    ): Promise<ServerSpec> {
      update((state) => ({ ...state, loading: true, error: null }));
      try {
        const spec = (await invoke('introspect_backend', {
          backend_type,
          backend_config,
          timeout_seconds,
        })) as ServerSpec;
        update((state) => ({ ...state, loading: false }));
        return spec;
      } catch (err) {
        const error = err instanceof Error ? err.message : 'Failed to introspect backend';
        update((state) => ({ ...state, error, loading: false }));
        throw err;
      }
    },

    // Set active proxy for detailed view
    setActiveProxy(proxyId: string | null) {
      update((state) => ({ ...state, activeProxyId: proxyId }));
    },

    // Clear error message
    clearError() {
      update((state) => ({ ...state, error: null }));
    },

    // Reset store to initial state
    reset() {
      set(initialState);
    },
  };
}

// Create the proxy store instance
export const proxyStore = createProxyStore();

// Derived stores for convenience
export const proxies = derived(proxyStore, ($store) => $store.proxies);
export const activeProxyId = derived(proxyStore, ($store) => $store.activeProxyId);
export const activeProxy = derived(
  [proxyStore, activeProxyId],
  ([$store, $activeId]) => $store.proxies.find((p) => p.id.value === $activeId)
);
export const runningProxies = derived(proxyStore, ($store) =>
  $store.proxies.filter((p) => p.running)
);
export const proxyCount = derived(proxyStore, ($store) => $store.proxies.length);
export const proxyLoading = derived(proxyStore, ($store) => $store.loading);
export const proxyError = derived(proxyStore, ($store) => $store.error);
