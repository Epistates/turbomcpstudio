<script lang="ts">
  import { serverStore, type ServerInfo, type ToolExecution } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import { profileStore } from '$lib/stores/profileStore';
  import {
    Clock,
    Zap,
    Star,
    TrendingUp,
    Play,
    History,
    AlertTriangle,
    Sparkles,
    ArrowRight,
    Database,
    FileText,
    Activity
  } from 'lucide-svelte';

  const serverState = $derived($serverStore);
  const profileState = $derived($profileStore);

  const servers = $derived(
    serverState.servers instanceof Map
      ? Array.from(serverState.servers.values())
      : []
  );

  const toolExecutions = $derived(serverState.toolExecutions || []);

  // Recent Activity (last 10 operations)
  const recentActivity = $derived(() => {
    return toolExecutions
      .slice(0, 10)
      .map(exec => {
        const timestampDate = typeof exec.timestamp === 'string'
          ? new Date(exec.timestamp)
          : exec.timestamp;
        const timestampStr = timestampDate.toISOString();
        return {
          ...exec,
          timestamp: timestampStr,
          timeAgo: getTimeAgo(timestampDate)
        };
      });
  });

  // Most Used Tools (by frequency)
  const mostUsedTools = $derived(() => {
    const toolCounts = new Map<string, { count: number; serverName: string; lastUsed: string }>();

    toolExecutions.forEach(exec => {
      const key = `${exec.serverName}::${exec.tool}`;
      const existing = toolCounts.get(key);
      if (existing) {
        existing.count++;
        if (new Date(exec.timestamp) > new Date(existing.lastUsed)) {
          existing.lastUsed = new Date(exec.timestamp).toISOString();
        }
      } else {
        toolCounts.set(key, {
          count: 1,
          serverName: exec.serverName,
          lastUsed: new Date(exec.timestamp).toISOString()
        });
      }
    });

    return Array.from(toolCounts.entries())
      .map(([key, data]) => {
        const [, toolName] = key.split('::');
        return { toolName, ...data };
      })
      .sort((a, b) => b.count - a.count)
      .slice(0, 5);
  });

  // Active Servers (connected)
  const activeServers = $derived(
    servers.filter(s => s.status === 'connected')
  );

  // Performance Insights
  const performanceInsights = $derived(() => {
    const insights: Array<{ type: 'warning' | 'info' | 'success'; message: string }> = [];

    // Slow servers
    const slowServers = activeServers.filter(s =>
      s.metrics?.avg_response_time_ms && s.metrics.avg_response_time_ms > 1000
    );
    if (slowServers.length > 0) {
      insights.push({
        type: 'warning',
        message: `${slowServers.length} server${slowServers.length > 1 ? 's are' : ' is'} responding slowly (>1s)`
      });
    }

    // Servers with errors
    const errorServers = servers.filter(s => s.status === 'error');
    if (errorServers.length > 0) {
      insights.push({
        type: 'warning',
        message: `${errorServers.length} server${errorServers.length > 1 ? 's have' : ' has'} connection errors`
      });
    }

    // High activity
    const highActivityServers = activeServers.filter(s =>
      s.metrics && s.metrics.requests_sent > 100
    );
    if (highActivityServers.length > 0) {
      insights.push({
        type: 'info',
        message: `${highActivityServers.length} server${highActivityServers.length > 1 ? 's are' : ' is'} highly active (100+ requests)`
      });
    }

    // All healthy
    if (errorServers.length === 0 && slowServers.length === 0 && activeServers.length > 0) {
      insights.push({
        type: 'success',
        message: 'All connected servers are healthy and performing well'
      });
    }

    return insights;
  });

  // Quick Actions based on recent activity
  const suggestedActions = $derived(() => {
    const actions: Array<{ icon: any; label: string; action: () => void; description: string }> = [];

    // Resume last tool
    if (recentActivity().length > 0) {
      const lastExec = recentActivity()[0];
      actions.push({
        icon: Play,
        label: `Replay: ${lastExec.tool}`,
        action: () => {
          // Navigate to tool explorer with this tool pre-selected
          uiStore.setSelectedTool(lastExec.tool, lastExec.serverId);
          uiStore.setView('tools');
        },
        description: `on ${lastExec.serverName}`
      });
    }

    // Activate profile if inactive
    if (!profileState.activeProfile && profileState.profiles.length > 0) {
      actions.push({
        icon: Zap,
        label: 'Activate Profile',
        action: () => uiStore.setView('servers'),
        description: `${profileState.profiles.length} profile${profileState.profiles.length > 1 ? 's' : ''} available`
      });
    }

    // Explore tools if servers connected
    if (activeServers.length > 0) {
      actions.push({
        icon: Sparkles,
        label: 'Explore Tools',
        action: () => uiStore.setView('tools'),
        description: `${activeServers.length} server${activeServers.length > 1 ? 's' : ''} connected`
      });
    }

    return actions;
  });

  function getTimeAgo(date: Date): string {
    const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    return `${Math.floor(seconds / 86400)}d ago`;
  }

  function getStatusColor(status: string): string {
    return status === 'success' ? 'text-green-600' : 'text-red-600';
  }

  function getStatusIcon(status: string) {
    return status === 'success' ? '✓' : '✗';
  }

  function navigateToExecution(execution: ToolExecution) {
    // Select the server and navigate to tools view
    serverStore.selectServer(execution.serverId);
    uiStore.setSelectedTool(execution.tool, execution.serverId);
    uiStore.setView('tools');
  }
</script>

<div class="workflow-dashboard">
  <!-- Welcome Header -->
  <div class="welcome-header">
    <div class="welcome-content">
      <h1 class="welcome-title">
        {#if profileState.activeProfile?.profile}
          <Zap size={28} class="text-blue-600" />
          {profileState.activeProfile.profile.name || 'Active Profile'}
        {:else}
          Welcome back!
        {/if}
      </h1>
      <p class="welcome-subtitle">
        {#if activeServers.length > 0}
          {activeServers.length} server{activeServers.length !== 1 ? 's' : ''} connected • Ready to work
        {:else}
          Connect to servers to get started
        {/if}
      </p>
    </div>
    <div class="quick-stats">
      <div class="quick-stat">
        <Activity size={16} />
        <span class="stat-value">{toolExecutions.length}</span>
        <span class="stat-label">Operations</span>
      </div>
      <div class="quick-stat">
        <Database size={16} />
        <span class="stat-value">{servers.length}</span>
        <span class="stat-label">Servers</span>
      </div>
    </div>
  </div>

  <div class="dashboard-grid">
    <!-- Quick Actions -->
    <div class="dashboard-card quick-actions-card">
      <div class="card-header">
        <Sparkles size={20} />
        <h2>Quick Actions</h2>
      </div>
      {#if suggestedActions().length > 0}
        <div class="action-list">
          {#each suggestedActions() as action}
            <button class="action-item" onclick={action.action}>
              <div class="action-icon">
                <svelte:component this={action.icon} size={20} />
              </div>
              <div class="action-content">
                <span class="action-label">{action.label}</span>
                <span class="action-description">{action.description}</span>
              </div>
              <ArrowRight size={16} class="action-arrow" />
            </button>
          {/each}
        </div>
      {:else}
        <div class="empty-state">
          <Play size={32} class="empty-icon" />
          <p>No recent activity. Connect to a server to get started!</p>
        </div>
      {/if}
    </div>

    <!-- Performance Insights -->
    {#if performanceInsights().length > 0}
      <div class="dashboard-card insights-card">
        <div class="card-header">
          <TrendingUp size={20} />
          <h2>Insights</h2>
        </div>
        <div class="insights-list">
          {#each performanceInsights() as insight}
            <div class="insight-item insight-{insight.type}">
              {#if insight.type === 'warning'}
                <AlertTriangle size={16} />
              {:else if insight.type === 'success'}
                <Sparkles size={16} />
              {:else}
                <Activity size={16} />
              {/if}
              <span>{insight.message}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Recent Activity -->
    <div class="dashboard-card activity-card">
      <div class="card-header">
        <Clock size={20} />
        <h2>Recent Activity</h2>
        {#if recentActivity().length > 0}
          <button
            class="header-action"
            onclick={() => uiStore.setView('protocol')}
          >
            View All
          </button>
        {/if}
      </div>
      {#if recentActivity().length > 0}
        <div class="activity-list">
          {#each recentActivity() as activity}
            <button class="activity-item" onclick={() => navigateToExecution(activity)}>
              <div class="activity-status {getStatusColor(activity.status)}">
                {getStatusIcon(activity.status)}
              </div>
              <div class="activity-content">
                <div class="activity-header">
                  <span class="activity-tool">{activity.tool}</span>
                  <span class="activity-server">{activity.serverName}</span>
                </div>
                <div class="activity-meta">
                  <span class="activity-time">{activity.timeAgo}</span>
                  {#if activity.duration}
                    <span class="activity-duration">{activity.duration}ms</span>
                  {/if}
                </div>
              </div>
              <ArrowRight size={14} class="activity-arrow" />
            </button>
          {/each}
        </div>
      {:else}
        <div class="empty-state">
          <History size={32} class="empty-icon" />
          <p>No recent activity</p>
          <button class="btn-primary" onclick={() => uiStore.setView('tools')}>
            <Zap size={16} />
            Explore Tools
          </button>
        </div>
      {/if}
    </div>

    <!-- Most Used Tools -->
    {#if mostUsedTools().length > 0}
      <div class="dashboard-card favorites-card">
        <div class="card-header">
          <Star size={20} />
          <h2>Most Used Tools</h2>
        </div>
        <div class="favorites-list">
          {#each mostUsedTools() as tool}
            <button
              class="favorite-item"
              onclick={() => {
                // Find server by name and navigate
                const server = servers.find(s => s.config.name === tool.serverName);
                if (server) {
                  serverStore.selectServer(server.id);
                  uiStore.setSelectedTool(tool.toolName, server.id);
                  uiStore.setView('tools');
                }
              }}
            >
              <div class="favorite-icon">
                <Zap size={16} />
              </div>
              <div class="favorite-content">
                <span class="favorite-name">{tool.toolName}</span>
                <span class="favorite-server">{tool.serverName}</span>
              </div>
              <div class="favorite-meta">
                <span class="usage-count">{tool.count}× used</span>
              </div>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .workflow-dashboard {
    padding: var(--mcp-space-6);
    max-width: 1400px;
    margin: 0 auto;
  }

  /* Welcome Header */
  .welcome-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--mcp-space-8);
    padding-bottom: var(--mcp-space-6);
    border-bottom: 2px solid var(--mcp-border-primary);
  }

  .welcome-content {
    flex: 1;
  }

  .welcome-title {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    margin: 0 0 var(--mcp-space-2) 0;
    font-size: var(--mcp-text-3xl);
    font-weight: var(--mcp-font-bold);
    color: var(--mcp-text-primary);
  }

  .welcome-subtitle {
    margin: 0;
    font-size: var(--mcp-text-lg);
    color: var(--mcp-text-secondary);
  }

  .quick-stats {
    display: flex;
    gap: var(--mcp-space-6);
  }

  .quick-stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--mcp-space-1);
    color: var(--mcp-text-tertiary);
  }

  .stat-value {
    font-size: var(--mcp-text-2xl);
    font-weight: var(--mcp-font-bold);
    color: var(--mcp-text-primary);
  }

  .stat-label {
    font-size: var(--mcp-text-xs);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* Dashboard Grid */
  .dashboard-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
    gap: var(--mcp-space-6);
  }

  .dashboard-card {
    padding: var(--mcp-space-6);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-xl);
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

  .header-action {
    padding: var(--mcp-space-1-5) var(--mcp-space-3);
    background: transparent;
    border: none;
    color: var(--mcp-primary-600);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    cursor: pointer;
    border-radius: var(--mcp-radius-md);
    transition: all 0.2s ease;
  }

  .header-action:hover {
    background: var(--mcp-primary-50);
  }

  /* Quick Actions */
  .action-list {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-3);
  }

  .action-item {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-4);
    padding: var(--mcp-space-4);
    background: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
  }

  .action-item:hover {
    background: var(--mcp-surface-tertiary);
    border-color: var(--mcp-primary-500);
    transform: translateX(4px);
  }

  .action-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    background: var(--mcp-primary-100);
    color: var(--mcp-primary-600);
    border-radius: var(--mcp-radius-lg);
  }

  .action-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-1);
  }

  .action-label {
    font-size: var(--mcp-text-base);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
  }

  .action-description {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-tertiary);
  }

  .action-arrow {
    color: var(--mcp-text-tertiary);
    transition: transform 0.2s ease;
  }

  .action-item:hover .action-arrow {
    transform: translateX(4px);
  }

  /* Insights */
  .insights-list {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-3);
  }

  .insight-item {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-3) var(--mcp-space-4);
    border-radius: var(--mcp-radius-lg);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
  }

  .insight-warning {
    background: var(--mcp-warning-50);
    color: var(--mcp-warning-800);
    border: 1px solid var(--mcp-warning-200);
  }

  .insight-info {
    background: var(--mcp-primary-50);
    color: var(--mcp-primary-800);
    border: 1px solid var(--mcp-primary-200);
  }

  .insight-success {
    background: var(--mcp-success-50);
    color: var(--mcp-success-800);
    border: 1px solid var(--mcp-success-200);
  }

  /* Activity List */
  .activity-list {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-2);
  }

  .activity-item {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-3);
    background: transparent;
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
  }

  .activity-item:hover {
    background: var(--mcp-surface-secondary);
    border-color: var(--mcp-primary-500);
  }

  .activity-status {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    font-weight: var(--mcp-font-bold);
    border-radius: 50%;
  }

  .activity-content {
    flex: 1;
    min-width: 0;
  }

  .activity-header {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
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

  .activity-server {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    padding: 2px 6px;
    background: var(--mcp-surface-tertiary);
    border-radius: var(--mcp-radius-sm);
  }

  .activity-meta {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
  }

  .activity-arrow {
    color: var(--mcp-text-tertiary);
  }

  /* Favorites */
  .favorites-list {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-2);
  }

  .favorite-item {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-3);
    background: transparent;
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
  }

  .favorite-item:hover {
    background: var(--mcp-surface-secondary);
    border-color: var(--mcp-primary-500);
  }

  .favorite-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: var(--mcp-primary-100);
    color: var(--mcp-primary-600);
    border-radius: var(--mcp-radius-md);
  }

  .favorite-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-1);
  }

  .favorite-name {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
  }

  .favorite-server {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
  }

  .usage-count {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    font-weight: var(--mcp-font-medium);
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
  @media (max-width: 768px) {
    .dashboard-grid {
      grid-template-columns: 1fr;
    }

    .welcome-header {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--mcp-space-4);
    }

    .quick-stats {
      width: 100%;
      justify-content: space-around;
    }
  }
</style>
