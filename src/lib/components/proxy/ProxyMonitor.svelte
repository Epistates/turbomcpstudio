<script lang="ts">
  import { proxyStore, activeProxy, runningProxies } from '$lib/stores/proxyStore';
  import { createEventDispatcher, onMount } from 'svelte';

  const dispatch = createEventDispatcher();

  interface Props {
    selectedProxyId: string;
  }

  let { selectedProxyId } = $props();

  let proxy = $derived(
    proxyStore.subscribe(($store) => $store.proxies.find((p) => p.id.value === selectedProxyId))
  );

  let metrics = $state(null as any);
  let metricsError = $state<string | null>(null);
  let loadingMetrics = $state(false);
  let autoRefresh = $state(true);
  let refreshInterval: ReturnType<typeof setInterval> | null = null;

  async function loadMetrics() {
    if (!proxy) return;

    loadingMetrics = true;
    metricsError = null;

    try {
      metrics = await proxyStore.getProxyMetrics(selectedProxyId);
    } catch (err) {
      metricsError = err instanceof Error ? err.message : 'Failed to load metrics';
      metrics = null;
    } finally {
      loadingMetrics = false;
    }
  }

  onMount(async () => {
    await loadMetrics();

    refreshInterval = setInterval(() => {
      if (autoRefresh && proxy) {
        loadMetrics();
      }
    }, 5000); // Refresh every 5 seconds

    return () => {
      if (refreshInterval) clearInterval(refreshInterval);
    };
  });

  function handleClose() {
    dispatch('close');
  }

  function formatLatency(ms: number): string {
    return ms.toFixed(1) + ' ms';
  }

  function formatUptime(seconds: number): string {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;

    if (hours > 0) return `${hours}h ${minutes}m`;
    if (minutes > 0) return `${minutes}m ${secs}s`;
    return `${secs}s`;
  }
</script>

<div class="proxy-monitor">
  <div class="monitor-header">
    <button class="btn-back" on:click={handleClose}>
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M15 19l-7-7 7-7"
        />
      </svg>
      Back
    </button>

    {#if proxy}
      <div class="header-info">
        <h2 class="monitor-title">{proxy.name}</h2>
        <div class="header-meta">
          {#if proxy.running}
            <span class="badge running">Running</span>
          {:else}
            <span class="badge stopped">Stopped</span>
          {/if}
          <span class="text-sm text-gray-500">{proxy.frontend_type}</span>
        </div>
      </div>
    {/if}

    <div class="header-actions">
      <button
        class="btn-icon"
        on:click={() => (autoRefresh = !autoRefresh)}
        title={autoRefresh ? 'Auto-refresh enabled' : 'Auto-refresh disabled'}
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
          />
        </svg>
      </button>

      <button class="btn-icon" on:click={loadMetrics} disabled={loadingMetrics}>
        {#if loadingMetrics}
          <svg class="spinner" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2" />
          </svg>
        {:else}
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            />
          </svg>
        {/if}
      </button>
    </div>
  </div>

  {#if proxy}
    <div class="monitor-content">
      <!-- Configuration -->
      <div class="info-section">
        <h3 class="section-title">Configuration</h3>

        <div class="info-grid">
          <div class="info-item">
            <span class="info-label">Frontend Type</span>
            <span class="info-value code">{proxy.frontend_type}</span>
          </div>

          {#if proxy.frontend_url}
            <div class="info-item">
              <span class="info-label">Frontend URL</span>
              <span class="info-value code">{proxy.frontend_url}</span>
            </div>
          {/if}

          {#if proxy.description}
            <div class="info-item full-width">
              <span class="info-label">Description</span>
              <span class="info-value">{proxy.description}</span>
            </div>
          {/if}
        </div>
      </div>

      <!-- Metrics -->
      {#if proxy.running && metrics}
        <div class="info-section">
          <h3 class="section-title">Metrics</h3>

          {#if metricsError}
            <div class="error-box">{metricsError}</div>
          {:else}
            <div class="metrics-grid">
              <div class="metric-card">
                <div class="metric-label">Total Requests</div>
                <div class="metric-value">{metrics.total_requests}</div>
              </div>

              <div class="metric-card">
                <div class="metric-label">Errors</div>
                <div class="metric-value" class:error={metrics.error_count > 0}>
                  {metrics.error_count}
                </div>
              </div>

              <div class="metric-card">
                <div class="metric-label">P50 Latency</div>
                <div class="metric-value">{formatLatency(metrics.p50_latency_ms)}</div>
              </div>

              <div class="metric-card">
                <div class="metric-label">P95 Latency</div>
                <div class="metric-value">{formatLatency(metrics.p95_latency_ms)}</div>
              </div>

              <div class="metric-card">
                <div class="metric-label">P99 Latency</div>
                <div class="metric-value">{formatLatency(metrics.p99_latency_ms)}</div>
              </div>

              <div class="metric-card">
                <div class="metric-label">Uptime</div>
                <div class="metric-value">{formatUptime(metrics.uptime_seconds)}</div>
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <!-- Status -->
      {#if proxy.uptime_seconds !== undefined}
        <div class="info-section">
          <h3 class="section-title">Status</h3>

          <div class="info-grid">
            <div class="info-item">
              <span class="info-label">Uptime</span>
              <span class="info-value">{formatUptime(proxy.uptime_seconds)}</span>
            </div>

            {#if proxy.total_requests !== undefined}
              <div class="info-item">
                <span class="info-label">Total Requests</span>
                <span class="info-value">{proxy.total_requests}</span>
              </div>
            {/if}

            {#if proxy.error_count !== undefined}
              <div class="info-item">
                <span class="info-label">Error Count</span>
                <span class="info-value" class:error={proxy.error_count > 0}>
                  {proxy.error_count}
                </span>
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Help -->
      <div class="info-section help-section">
        <h3 class="section-title">What's Next?</h3>

        <ul class="help-list">
          <li>
            <strong>Test the proxy:</strong>
            {#if proxy.frontend_url}
              Connect to <code>{proxy.frontend_url}</code> to test the proxy with MCP clients
            {:else}
              Start the proxy to get a frontend URL
            {/if}
          </li>
          <li>
            <strong>Monitor traffic:</strong>
            Check the Protocol Inspector to see all requests and responses in real-time
          </li>
          <li>
            <strong>Share configuration:</strong>
            Export this proxy configuration to share with team members
          </li>
        </ul>
      </div>
    </div>
  {/if}
</div>

<style>
  .proxy-monitor {
    display: flex;
    flex-direction: column;
    gap: 0;
    height: 100%;
  }

  .monitor-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-secondary);
  }

  .btn-back {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: transparent;
    cursor: pointer;
    font-size: 0.875rem;
    color: var(--color-text);
    transition: all 0.2s;
  }

  .btn-back:hover {
    background: var(--color-bg);
  }

  .header-info {
    flex: 1;
  }

  .monitor-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--color-text);
    margin: 0;
  }

  .header-meta {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-top: 0.25rem;
    font-size: 0.875rem;
  }

  .badge {
    display: inline-block;
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .badge.running {
    background: var(--color-success);
    color: white;
  }

  .badge.stopped {
    background: var(--color-border);
    color: var(--color-text);
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .btn-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: transparent;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-icon:hover:not(:disabled) {
    background: var(--color-bg);
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .btn-icon:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .spinner {
    width: 16px;
    height: 16px;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .monitor-content {
    flex: 1;
    overflow: auto;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .info-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    border: 1px solid var(--color-border);
    border-radius: 0.5rem;
    background: var(--color-bg-secondary);
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--color-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 0;
  }

  .info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }

  .info-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .info-item.full-width {
    grid-column: 1 / -1;
  }

  .info-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--color-text-secondary);
    letter-spacing: 0.5px;
  }

  .info-value {
    font-size: 0.875rem;
    color: var(--color-text);
    font-weight: 500;
  }

  .info-value.code {
    font-family: monospace;
    font-size: 0.8rem;
    word-break: break-all;
    padding: 0.25rem 0.5rem;
    background: var(--color-bg);
    border-radius: 0.25rem;
  }

  .info-value.error {
    color: var(--color-error);
    font-weight: 600;
  }

  .error-box {
    padding: 0.75rem;
    background: #fee;
    border: 1px solid #fcc;
    border-radius: 0.375rem;
    color: #c00;
    font-size: 0.875rem;
  }

  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 0.75rem;
  }

  .metric-card {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
  }

  .metric-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .metric-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-primary);
    font-family: monospace;
    line-height: 1;
  }

  .metric-value.error {
    color: var(--color-error);
  }

  .help-section {
    background: #f0f9ff;
  }

  .help-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .help-list li {
    font-size: 0.875rem;
    color: var(--color-text);
  }

  .help-list code {
    font-family: monospace;
    font-size: 0.8rem;
    padding: 0.125rem 0.375rem;
    background: white;
    border-radius: 0.25rem;
  }
</style>
