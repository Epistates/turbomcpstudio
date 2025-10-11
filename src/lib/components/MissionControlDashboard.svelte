<script lang="ts">
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import {
    Activity,
    CheckCircle,
    AlertCircle,
    Clock,
    Zap,
    Database,
    FileText,
    ArrowRight,
    PlayCircle,
    RefreshCw
  } from 'lucide-svelte';

  const serverState = $derived($serverStore);
  const servers = $derived(
    serverState.servers instanceof Map
      ? Array.from(serverState.servers.values())
      : []
  );
  const toolExecutions = $derived(serverState.toolExecutions || []);

  // System Health Metrics
  const health = $derived(() => {
    const connected = servers.filter(s => s.status === 'connected');
    const errors = servers.filter(s => s.status === 'error');
    const avgResponseTime = connected.length > 0
      ? Math.round(
          connected.reduce((sum, s) => sum + (s.metrics?.avg_response_time_ms || 0), 0) / connected.length
        )
      : 0;

    return {
      connected: connected.length,
      total: servers.length,
      errors: errors.length,
      avgResponseTime,
      status: errors.length > 0 ? 'warning' : connected.length === servers.length ? 'healthy' : 'partial'
    };
  });

  // Capability Matrix Data
  const capabilityMatrix = $derived(() => {
    return servers.map(s => ({
      name: s.config.name,
      id: s.id,
      status: s.status,
      tools: s.capabilities?.tools ? '✓' : '✗',
      resources: s.capabilities?.resources ? '✓' : '✗',
      prompts: s.capabilities?.prompts ? '✓' : '✗',
      sampling: s.capabilities?.sampling ? '✓' : '✗',
      // Count if available (would need to fetch from backend)
      toolCount: 0, // TODO: Add to ServerInfo
      resourceCount: 0,
      promptCount: 0
    }));
  });

  // Recent Activity
  const recentActivity = $derived(() => {
    return toolExecutions
      .slice(0, 10)
      .map(exec => ({
        ...exec,
        timeAgo: getTimeAgo(new Date(exec.timestamp))
      }));
  });

  function getTimeAgo(date: Date): string {
    const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    return `${Math.floor(seconds / 86400)}d ago`;
  }

  function getHealthIcon() {
    const h = health();
    if (h.status === 'healthy') return CheckCircle;
    if (h.status === 'warning') return AlertCircle;
    return Activity;
  }

  function getHealthColor() {
    const h = health();
    if (h.status === 'healthy') return 'text-green-600';
    if (h.status === 'warning') return 'text-red-600';
    return 'text-yellow-600';
  }

  function navigateToCapability(serverId: string, capability: string) {
    serverStore.selectServer(serverId);
    uiStore.setView(capability as any);
  }

  function replayExecution(execution: any) {
    serverStore.selectServer(execution.serverId);
    uiStore.setSelectedTool(execution.tool, execution.serverId);
    uiStore.setView('tools');
  }
</script>

<div class="mission-control">
  <!-- System Health Bar -->
  <div class="health-bar">
    <div class="health-status">
      <svelte:component this={getHealthIcon()} size={24} class={getHealthColor()} />
      <div class="health-text">
        <span class="health-count">
          {health().connected}/{health().total} connected
        </span>
        {#if health().errors > 0}
          <span class="health-warning">⚠️ {health().errors} error{health().errors > 1 ? 's' : ''}</span>
        {/if}
      </div>
    </div>
    <div class="health-metrics">
      {#if toolExecutions.length > 0}
        <div class="metric">
          <Clock size={16} />
          <span>Last activity: {getTimeAgo(new Date(toolExecutions[0].timestamp))}</span>
        </div>
      {/if}
      {#if health().connected > 0}
        <div class="metric">
          <Zap size={16} />
          <span>Avg response: {health().avgResponseTime}ms</span>
        </div>
      {/if}
    </div>
  </div>

  <div class="dashboard-grid">
    <!-- Capability Matrix -->
    <div class="dashboard-card capability-matrix">
      <div class="card-header">
        <Database size={20} />
        <h2>Server Capabilities</h2>
      </div>
      {#if capabilityMatrix().length > 0}
        <div class="matrix-table">
          <table>
            <thead>
              <tr>
                <th>Server</th>
                <th>Tools</th>
                <th>Resources</th>
                <th>Prompts</th>
                <th>Sampling</th>
              </tr>
            </thead>
            <tbody>
              {#each capabilityMatrix() as server}
                <tr class:disconnected={server.status !== 'connected'}>
                  <td class="server-name">
                    <span class="status-dot status-{server.status}"></span>
                    {server.name}
                  </td>
                  <td>
                    {#if server.tools === '✓'}
                      <button
                        class="capability-cell active"
                        onclick={() => navigateToCapability(server.id, 'tools')}
                        title="View tools"
                      >
                        {server.tools}
                      </button>
                    {:else}
                      <span class="capability-cell inactive">{server.tools}</span>
                    {/if}
                  </td>
                  <td>
                    {#if server.resources === '✓'}
                      <button
                        class="capability-cell active"
                        onclick={() => navigateToCapability(server.id, 'resources')}
                        title="View resources"
                      >
                        {server.resources}
                      </button>
                    {:else}
                      <span class="capability-cell inactive">{server.resources}</span>
                    {/if}
                  </td>
                  <td>
                    {#if server.prompts === '✓'}
                      <button
                        class="capability-cell active"
                        onclick={() => navigateToCapability(server.id, 'prompts')}
                        title="View prompts"
                      >
                        {server.prompts}
                      </button>
                    {:else}
                      <span class="capability-cell inactive">{server.prompts}</span>
                    {/if}
                  </td>
                  <td>
                    {#if server.sampling === '✓'}
                      <button
                        class="capability-cell active"
                        onclick={() => navigateToCapability(server.id, 'sampling')}
                        title="View sampling"
                      >
                        {server.sampling}
                      </button>
                    {:else}
                      <span class="capability-cell inactive">{server.sampling}</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <div class="empty-state">
          <Database size={32} class="empty-icon" />
          <p>No servers configured</p>
          <button class="btn-primary" onclick={() => uiStore.setView('servers')}>
            Add Server
          </button>
        </div>
      {/if}
    </div>

    <!-- Recent Activity -->
    <div class="dashboard-card recent-activity">
      <div class="card-header">
        <Activity size={20} />
        <h2>Recent Activity</h2>
        {#if recentActivity().length > 0}
          <button class="header-link" onclick={() => uiStore.setView('protocol')}>
            View All
          </button>
        {/if}
      </div>
      {#if recentActivity().length > 0}
        <div class="activity-list">
          {#each recentActivity() as activity}
            <div class="activity-item">
              <div class="activity-status {activity.status}">
                {activity.status === 'success' ? '✓' : '✗'}
              </div>
              <div class="activity-content">
                <div class="activity-header">
                  <span class="activity-tool">{activity.tool}</span>
                  <span class="activity-time">{activity.timeAgo}</span>
                </div>
                <div class="activity-details">
                  <span class="activity-server">{activity.serverName}</span>
                  {#if activity.duration}
                    <span class="activity-duration">{activity.duration}ms</span>
                  {/if}
                </div>
              </div>
              <button
                class="activity-action"
                onclick={() => replayExecution(activity)}
                title="Replay this operation"
              >
                <PlayCircle size={16} />
              </button>
            </div>
          {/each}
        </div>
      {:else}
        <div class="empty-state">
          <Activity size={32} class="empty-icon" />
          <p>No recent activity</p>
          <button class="btn-secondary" onclick={() => uiStore.setView('tools')}>
            <Zap size={16} />
            Explore Tools
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .mission-control {
    padding: var(--mcp-space-6);
    max-width: 1400px;
    margin: 0 auto;
  }

  /* Health Bar */
  .health-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--mcp-space-4) var(--mcp-space-6);
    margin-bottom: var(--mcp-space-6);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    box-shadow: var(--mcp-shadow-sm);
  }

  .health-status {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
  }

  .health-text {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-1);
  }

  .health-count {
    font-size: var(--mcp-text-lg);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
  }

  .health-warning {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-error-600);
  }

  .health-metrics {
    display: flex;
    gap: var(--mcp-space-6);
  }

  .metric {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
  }

  /* Dashboard Grid */
  .dashboard-grid {
    display: grid;
    grid-template-columns: 1.5fr 1fr;
    gap: var(--mcp-space-6);
  }

  .dashboard-card {
    padding: var(--mcp-space-6);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    box-shadow: var(--mcp-shadow-sm);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    margin-bottom: var(--mcp-space-4);
    color: var(--mcp-primary-600);
  }

  .card-header h2 {
    flex: 1;
    margin: 0;
    font-size: var(--mcp-text-lg);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
  }

  .header-link {
    padding: var(--mcp-space-1) var(--mcp-space-2);
    background: transparent;
    border: none;
    color: var(--mcp-primary-600);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    cursor: pointer;
    border-radius: var(--mcp-radius-sm);
    transition: background 0.2s;
  }

  .header-link:hover {
    background: var(--mcp-primary-50);
  }

  /* Capability Matrix */
  .matrix-table {
    overflow-x: auto;
  }

  .matrix-table table {
    width: 100%;
    border-collapse: collapse;
  }

  .matrix-table th {
    padding: var(--mcp-space-2) var(--mcp-space-3);
    text-align: left;
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-bottom: 2px solid var(--mcp-border-primary);
  }

  .matrix-table td {
    padding: var(--mcp-space-3);
    border-bottom: 1px solid var(--mcp-border-primary);
  }

  .matrix-table tr.disconnected {
    opacity: 0.5;
  }

  .server-name {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-primary);
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .status-dot.status-connected {
    background: var(--mcp-success-500);
  }

  .status-dot.status-disconnected {
    background: var(--mcp-gray-400);
  }

  .status-dot.status-error {
    background: var(--mcp-error-500);
  }

  .capability-cell {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    padding: 0 var(--mcp-space-2);
    font-size: var(--mcp-text-sm);
    border-radius: var(--mcp-radius-sm);
    transition: all 0.2s;
  }

  .capability-cell.active {
    background: var(--mcp-success-100);
    color: var(--mcp-success-700);
    border: none;
    cursor: pointer;
    font-weight: var(--mcp-font-semibold);
  }

  .capability-cell.active:hover {
    background: var(--mcp-success-200);
    transform: scale(1.05);
  }

  .capability-cell.inactive {
    color: var(--mcp-text-tertiary);
  }

  /* Recent Activity */
  .activity-list {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-3);
  }

  .activity-item {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-3);
    background: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    transition: all 0.2s;
  }

  .activity-item:hover {
    background: var(--mcp-surface-tertiary);
    border-color: var(--mcp-primary-500);
  }

  .activity-status {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    font-weight: var(--mcp-font-bold);
    font-size: var(--mcp-text-sm);
  }

  .activity-status.success {
    background: var(--mcp-success-100);
    color: var(--mcp-success-700);
  }

  .activity-status.error {
    background: var(--mcp-error-100);
    color: var(--mcp-error-700);
  }

  .activity-content {
    flex: 1;
    min-width: 0;
  }

  .activity-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--mcp-space-1);
  }

  .activity-tool {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-time {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
  }

  .activity-details {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
  }

  .activity-server {
    padding: 2px 6px;
    background: var(--mcp-surface-tertiary);
    border-radius: var(--mcp-radius-sm);
  }

  .activity-action {
    padding: var(--mcp-space-2);
    background: transparent;
    border: none;
    color: var(--mcp-primary-600);
    cursor: pointer;
    border-radius: var(--mcp-radius-sm);
    transition: all 0.2s;
  }

  .activity-action:hover {
    background: var(--mcp-primary-50);
  }

  /* Empty State */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--mcp-space-8) var(--mcp-space-4);
    text-align: center;
  }

  .empty-icon {
    color: var(--mcp-text-tertiary);
    margin-bottom: var(--mcp-space-3);
  }

  .empty-state p {
    margin: 0 0 var(--mcp-space-4) 0;
    color: var(--mcp-text-secondary);
  }

  /* Responsive */
  @media (max-width: 1024px) {
    .dashboard-grid {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 768px) {
    .health-bar {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--mcp-space-4);
    }

    .health-metrics {
      width: 100%;
      justify-content: space-between;
    }

    .matrix-table {
      font-size: var(--mcp-text-xs);
    }
  }
</style>
