<script lang="ts">
  import { serverStore, getServerStatus, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import {
    Activity,
    Zap,
    AlertCircle,
    TrendingUp,
    Clock,
    CheckCircle,
    Server,
    Filter
  } from 'lucide-svelte';

  const serverState = $derived($serverStore);
  const servers = $derived(
    serverState.servers instanceof Map
      ? Array.from(serverState.servers.values())
      : []
  );

  // Advanced metrics
  const metrics = $derived(() => {
    const connected = servers.filter(s => getServerStatus(s) === 'connected');
    const total = servers.length;
    const errors = servers.filter(s => getServerStatus(s) === 'error');

    // Performance metrics
    const avgResponseTime = connected.length > 0
      ? Math.round(
          connected.reduce((sum, s) => sum + (s.metrics?.avg_response_time_ms || 0), 0) / connected.length
        )
      : 0;

    // Health score (0-100)
    const healthScore = total > 0
      ? Math.round(
          ((connected.length / total) * 70) + // 70% weight on connectivity
          ((1 - (errors.length / total)) * 30) // 30% weight on error-free
        )
      : 100;

    // Total activity
    const totalRequests = servers.reduce((sum, s) => sum + (s.metrics?.requests_sent || 0), 0);
    const totalData = servers.reduce((sum, s) =>
      sum + (s.metrics?.bytes_sent || 0) + (s.metrics?.bytes_received || 0), 0
    );

    // Error rate
    const totalErrors = servers.reduce((sum, s) => sum + (s.metrics?.error_count || 0), 0);
    const errorRate = totalRequests > 0
      ? ((totalErrors / totalRequests) * 100).toFixed(2)
      : '0.00';

    // Recently active (active in last hour)
    const oneHourAgo = Date.now() - 3600000;
    const recentlyActive = servers.filter(s =>
      s.last_seen && new Date(s.last_seen).getTime() > oneHourAgo
    ).length;

    // Most active server
    const mostActive = servers.reduce((max, s) => {
      const requests = s.metrics?.requests_sent || 0;
      return requests > (max.requests || 0) ? { server: s, requests } : max;
    }, { server: null as ServerInfo | null, requests: 0 });

    return {
      total,
      connected: connected.length,
      errors: errors.length,
      disconnected: total - connected.length - errors.length,
      healthScore,
      avgResponseTime,
      totalRequests,
      totalData,
      errorRate,
      recentlyActive,
      mostActive: mostActive.server,
      mostActiveRequests: mostActive.requests
    };
  });

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function getHealthColor(score: number): string {
    if (score >= 80) return 'text-green-600 dark:text-green-400';
    if (score >= 60) return 'text-yellow-600 dark:text-yellow-400';
    return 'text-red-600 dark:text-red-400';
  }

  function getHealthLabel(score: number): string {
    if (score >= 80) return 'Excellent';
    if (score >= 60) return 'Good';
    return 'Needs Attention';
  }
</script>

<div class="enhanced-dashboard">
  <!-- System Health Overview -->
  <div class="health-banner">
    <div class="health-indicator">
      <div class="health-score {getHealthColor(metrics().healthScore)}">
        {metrics().healthScore}
      </div>
      <div class="health-label">
        <h2>System Health</h2>
        <p class={getHealthColor(metrics().healthScore)}>{getHealthLabel(metrics().healthScore)}</p>
      </div>
    </div>
    <div class="health-stats">
      <div class="stat">
        <CheckCircle size={20} class="text-green-600" />
        <span class="stat-value">{metrics().connected}/{metrics().total}</span>
        <span class="stat-label">Connected</span>
      </div>
      <div class="stat">
        <Activity size={20} class="text-blue-600" />
        <span class="stat-value">{metrics().recentlyActive}</span>
        <span class="stat-label">Recently Active</span>
      </div>
      <div class="stat">
        <AlertCircle size={20} class="text-red-600" />
        <span class="stat-value">{metrics().errors}</span>
        <span class="stat-label">Errors</span>
      </div>
    </div>
  </div>

  <!-- Detailed Metrics Grid -->
  <div class="metrics-grid">
    <!-- Connection Status -->
    <div class="metric-card">
      <div class="metric-header">
        <Server size={20} />
        <h3>Connection Status</h3>
      </div>
      <div class="connection-breakdown">
        <div class="status-bar">
          <div
            class="status-segment status-connected"
            style="width: {(metrics().connected / metrics().total) * 100}%"
            title="Connected: {metrics().connected}"
          ></div>
          <div
            class="status-segment status-disconnected"
            style="width: {(metrics().disconnected / metrics().total) * 100}%"
            title="Disconnected: {metrics().disconnected}"
          ></div>
          <div
            class="status-segment status-error"
            style="width: {(metrics().errors / metrics().total) * 100}%"
            title="Errors: {metrics().errors}"
          ></div>
        </div>
        <div class="status-legend">
          <div class="legend-item">
            <div class="legend-dot status-connected"></div>
            <span>Connected ({metrics().connected})</span>
          </div>
          <div class="legend-item">
            <div class="legend-dot status-disconnected"></div>
            <span>Disconnected ({metrics().disconnected})</span>
          </div>
          <div class="legend-item">
            <div class="legend-dot status-error"></div>
            <span>Errors ({metrics().errors})</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Performance -->
    <div class="metric-card">
      <div class="metric-header">
        <Zap size={20} />
        <h3>Performance</h3>
      </div>
      <div class="performance-metrics">
        <div class="perf-stat">
          <span class="perf-label">Avg Response Time</span>
          <span class="perf-value">{metrics().avgResponseTime}ms</span>
        </div>
        <div class="perf-stat">
          <span class="perf-label">Error Rate</span>
          <span class="perf-value">{metrics().errorRate}%</span>
        </div>
        <div class="perf-stat">
          <span class="perf-label">Total Requests</span>
          <span class="perf-value">{metrics().totalRequests.toLocaleString()}</span>
        </div>
      </div>
    </div>

    <!-- Activity -->
    <div class="metric-card">
      <div class="metric-header">
        <Activity size={20} />
        <h3>Activity</h3>
      </div>
      <div class="activity-metrics">
        <div class="activity-stat">
          <span class="activity-label">Data Transferred</span>
          <span class="activity-value">{formatBytes(metrics().totalData)}</span>
        </div>
        {#if metrics().mostActive}
          <div class="activity-stat">
            <span class="activity-label">Most Active Server</span>
            <span class="activity-value text-primary">{metrics().mostActive.config.name}</span>
            <span class="activity-detail">{metrics().mostActiveRequests} requests</span>
          </div>
        {/if}
      </div>
    </div>

    <!-- Quick Actions -->
    <div class="metric-card quick-actions">
      <div class="metric-header">
        <TrendingUp size={20} />
        <h3>Quick Actions</h3>
      </div>
      <div class="action-buttons">
        <button class="action-btn" onclick={() => uiStore.setView('servers')}>
          <Server size={16} />
          <span>Manage Servers</span>
        </button>
        <button class="action-btn" onclick={() => uiStore.setView('protocol')}>
          <Activity size={16} />
          <span>View Protocol</span>
        </button>
        <button class="action-btn" onclick={() => uiStore.openModal('addServer')}>
          <Zap size={16} />
          <span>Add Server</span>
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .enhanced-dashboard {
    padding: var(--mcp-space-6);
    max-width: 1400px;
    margin: 0 auto;
  }

  /* Health Banner */
  .health-banner {
    display: flex;
    gap: var(--mcp-space-6);
    padding: var(--mcp-space-6);
    margin-bottom: var(--mcp-space-6);
    background: linear-gradient(135deg, var(--mcp-primary-50) 0%, var(--mcp-primary-100) 100%);
    border-radius: var(--mcp-radius-lg);
    border: 1px solid var(--mcp-primary-200);
  }

  [data-theme="dark"] .health-banner {
    background: linear-gradient(135deg, rgba(12, 74, 110, 0.2) 0%, rgba(12, 74, 110, 0.3) 100%);
    border-color: var(--mcp-primary-800);
  }

  .health-indicator {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-4);
  }

  .health-score {
    font-size: 3rem;
    font-weight: var(--mcp-font-bold);
    line-height: 1;
  }

  .health-label h2 {
    margin: 0;
    font-size: var(--mcp-text-xl);
    color: var(--mcp-text-primary);
  }

  .health-label p {
    margin: var(--mcp-space-1) 0 0;
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
  }

  .health-stats {
    display: flex;
    gap: var(--mcp-space-6);
    margin-left: auto;
    align-items: center;
  }

  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--mcp-space-2);
  }

  .stat-value {
    font-size: var(--mcp-text-2xl);
    font-weight: var(--mcp-font-bold);
    color: var(--mcp-text-primary);
  }

  .stat-label {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* Metrics Grid */
  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: var(--mcp-space-6);
  }

  .metric-card {
    padding: var(--mcp-space-6);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    box-shadow: var(--mcp-shadow-sm);
  }

  .metric-header {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    margin-bottom: var(--mcp-space-4);
    color: var(--mcp-primary-600);
  }

  .metric-header h3 {
    margin: 0;
    font-size: var(--mcp-text-lg);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
  }

  /* Connection Breakdown */
  .connection-breakdown {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-4);
  }

  .status-bar {
    display: flex;
    height: 40px;
    border-radius: var(--mcp-radius-md);
    overflow: hidden;
    background: var(--mcp-surface-tertiary);
  }

  .status-segment {
    transition: width 0.3s ease;
  }

  .status-segment.status-connected {
    background: var(--mcp-success-500);
  }

  .status-segment.status-disconnected {
    background: var(--mcp-gray-400);
  }

  .status-segment.status-error {
    background: var(--mcp-error-500);
  }

  .status-legend {
    display: flex;
    gap: var(--mcp-space-4);
    font-size: var(--mcp-text-sm);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
  }

  .legend-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
  }

  /* Performance Metrics */
  .performance-metrics, .activity-metrics {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-4);
  }

  .perf-stat, .activity-stat {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--mcp-space-3);
    background: var(--mcp-surface-secondary);
    border-radius: var(--mcp-radius-md);
  }

  .perf-label, .activity-label {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
  }

  .perf-value, .activity-value {
    font-size: var(--mcp-text-lg);
    font-weight: var(--mcp-font-bold);
    color: var(--mcp-text-primary);
  }

  .activity-detail {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    margin-left: auto;
  }

  /* Quick Actions */
  .action-buttons {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-3);
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-3) var(--mcp-space-4);
    background: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-primary);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .action-btn:hover {
    background: var(--mcp-surface-tertiary);
    border-color: var(--mcp-primary-500);
    transform: translateY(-1px);
  }

  /* Responsive */
  @media (max-width: 768px) {
    .health-banner {
      flex-direction: column;
    }

    .health-stats {
      margin-left: 0;
      width: 100%;
      justify-content: space-around;
    }

    .metrics-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
