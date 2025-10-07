<script lang="ts">
  import { serverStore, getServerStatus, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import { profileStore } from '$lib/stores/profileStore';
  import ServerOverview from './ServerOverview.svelte';
  import AddServerModal from './AddServerModal.svelte';
  import ServerConfigModal from './ServerConfigModal.svelte';
  import {
    Plus,
    Activity,
    Zap,
    Database,
    FileText,
    AlertCircle,
    CheckCircle,
    Clock,
    Square,
    Settings,
    BarChart3,
    RefreshCw,
    FolderOpen
  } from 'lucide-svelte';

  // Reactive store state
  const serverState = $derived($serverStore);
  const uiState = $derived($uiStore);
  const profileState = $derived($profileStore);

  const servers = $derived(serverState.servers || []);
  const selectedServerId = $derived(serverState.selectedServerId);
  const modals = $derived(uiState.modals);
  const error = $derived(uiState.error);
  const activeProfile = $derived(profileState.activeProfile);

  // Profile-aware filtering
  const shouldShowProfileView = $derived(activeProfile?.profile != null);

  const displayServers = $derived(() => {
    if (shouldShowProfileView && activeProfile) {
      const profileServerIds = new Set(activeProfile.servers.map(ps => ps.server_id));
      return servers.filter(s => profileServerIds.has(s.id));
    } else {
      return servers;
    }
  });

  const selectedServer = $derived(
    servers.find(s => s.id === selectedServerId)
  );

  const connectedServers = $derived(
    displayServers().filter((s: any) => getServerStatus(s) === 'connected')
  );

  const stats = $derived({
    total: displayServers().length,
    connected: connectedServers.length,
    disconnected: displayServers().filter((s: any) => getServerStatus(s) === 'disconnected').length,
    error: displayServers().filter((s: any) => getServerStatus(s) === 'error').length,
  });

  // Profile metrics when active
  const profileMetrics = $derived(() => {
    if (!shouldShowProfileView || !activeProfile?.profile) return null;

    const profileServers = displayServers();
    const connectedCount = profileServers.filter(s => s.status === 'connected').length;

    // Calculate aggregate metrics
    const totalRequests = profileServers.reduce((sum, s) =>
      sum + (s.metrics?.requests_sent || 0), 0);
    const totalErrors = profileServers.reduce((sum, s) =>
      sum + (s.metrics?.error_count || 0), 0);
    const avgResponseTime = profileServers.length > 0
      ? Math.round(profileServers.reduce((sum, s) =>
          sum + (s.metrics?.avg_response_time_ms || 0), 0) / profileServers.length)
      : 0;

    const successRate = totalRequests > 0
      ? ((totalRequests - totalErrors) / totalRequests * 100).toFixed(1)
      : '100.0';

    // Find most active server
    const mostActive = profileServers.reduce((max, s) => {
      const requests = s.metrics?.requests_sent || 0;
      return requests > (max.requests || 0) ? { server: s, requests } : max;
    }, { server: null as ServerInfo | null, requests: 0 });

    return {
      connectedCount,
      totalCount: profileServers.length,
      totalRequests,
      totalErrors,
      avgResponseTime,
      successRate,
      mostActive: mostActive.server,
      mostActiveRequests: mostActive.requests,
      profileHealth: connectedCount === profileServers.length && totalErrors === 0 ? 'Excellent' :
                    connectedCount > profileServers.length / 2 ? 'Good' : 'Needs Attention'
    };
  });

  // Time ago helper
  function getTimeAgo(date: Date): string {
    const seconds = Math.floor((new Date().getTime() - date.getTime()) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    return `${Math.floor(seconds / 86400)}d ago`;
  }

  function openAddServer() {
    uiStore.openModal('addServer');
  }

  async function refreshServers() {
    await serverStore.loadServers();
    await serverStore.loadTemplates();
  }

  function dismissError() {
    uiStore.clearError();
  }
</script>

<!-- Error Banner -->
{#if error}
  <div class="bg-red-50 border-l-4 border-red-400 p-4 m-4 rounded">
    <div class="flex items-center justify-between">
      <div class="flex items-center">
        <AlertCircle size={20} class="text-red-400 mr-3" />
        <p class="text-sm text-red-700">{error}</p>
      </div>
      <button
        onclick={dismissError}
        class="text-red-400 hover:text-red-600"
      >
        ×
      </button>
    </div>
  </div>
{/if}

<div class="flex-1 overflow-hidden">
  <!-- Dashboard View -->
  <div class="h-full p-6 overflow-y-auto">
      <!-- Profile Banner (when active) -->
      {#if shouldShowProfileView && activeProfile?.profile && profileMetrics()}
        {@const metrics = profileMetrics()}
        {@const activatedAt = activeProfile.activation?.activated_at ? new Date(activeProfile.activation.activated_at) : null}
        {#if metrics && activeProfile.profile}
          <div class="mb-6 p-6 bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
            <div class="flex items-center justify-between mb-4">
              <div class="flex items-center gap-3">
                <Zap size={24} class="text-blue-600 dark:text-blue-400" />
                <div>
                  <h2 class="text-xl font-bold text-primary">{activeProfile.profile.name}</h2>
                  <p class="text-sm text-secondary">
                    {metrics.connectedCount}/{metrics.totalCount} servers connected
                    {#if activatedAt}
                      • Activated {getTimeAgo(activatedAt)}
                    {/if}
                  </p>
                </div>
              </div>
              <button
                onclick={async () => await profileStore.deactivateProfile()}
                class="btn-secondary flex items-center gap-2"
              >
                <Square size={16} />
                Deactivate
              </button>
            </div>

            <!-- Profile Metrics -->
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
              <div class="bg-white dark:bg-gray-800 p-3 rounded-lg">
                <p class="text-xs text-secondary mb-1">Total Requests</p>
                <p class="text-2xl font-bold text-primary">{metrics.totalRequests}</p>
              </div>
              <div class="bg-white dark:bg-gray-800 p-3 rounded-lg">
                <p class="text-xs text-secondary mb-1">Success Rate</p>
                <p class="text-2xl font-bold text-green-600 dark:text-green-400">{metrics.successRate}%</p>
              </div>
              <div class="bg-white dark:bg-gray-800 p-3 rounded-lg">
                <p class="text-xs text-secondary mb-1">Avg Response</p>
                <p class="text-2xl font-bold text-primary">{metrics.avgResponseTime}ms</p>
              </div>
              <div class="bg-white dark:bg-gray-800 p-3 rounded-lg">
                <p class="text-xs text-secondary mb-1">Profile Health</p>
                <p class="text-2xl font-bold {metrics.profileHealth === 'Excellent' ? 'text-green-600 dark:text-green-400' : metrics.profileHealth === 'Good' ? 'text-yellow-600 dark:text-yellow-400' : 'text-red-600 dark:text-red-400'}">
                  {metrics.profileHealth}
                </p>
              </div>
            </div>

          <!-- Quick Actions -->
          <div class="flex flex-wrap gap-2">
            <button
              onclick={() => uiStore.setView('tools')}
              class="btn-secondary text-sm flex items-center gap-2"
            >
              <Zap size={14} />
              All Tools
            </button>
            <button
              onclick={() => uiStore.setView('resources')}
              class="btn-secondary text-sm flex items-center gap-2"
            >
              <Database size={14} />
              All Resources
            </button>
            <button
              onclick={() => uiStore.setView('prompts')}
              class="btn-secondary text-sm flex items-center gap-2"
            >
              <FileText size={14} />
              All Prompts
            </button>
            <button
              onclick={() => uiStore.setView('servers')}
              class="btn-secondary text-sm flex items-center gap-2"
            >
              <Settings size={14} />
              Profile Settings
            </button>
          </div>
        </div>
        {/if}
      {/if}

      <!-- Header -->
      <div class="mb-6">
        <div class="flex items-center justify-between">
          <div>
            <h1 class="text-2xl font-bold text-primary">
              {#if shouldShowProfileView && activeProfile?.profile}
                {activeProfile.profile.name} Dashboard
              {:else}
                Dashboard
              {/if}
            </h1>
            <p class="text-secondary">
              {#if shouldShowProfileView}
                Monitoring your active profile servers
              {:else}
                Manage your MCP server connections and monitor activity
              {/if}
            </p>
          </div>
          <div class="flex space-x-3">
            <button
              onclick={refreshServers}
              class="btn-secondary"
            >
              <Activity size={16} class="mr-2" />
              Refresh
            </button>
            {#if !shouldShowProfileView}
              <button
                onclick={openAddServer}
                class="btn-primary"
              >
                <Plus size={16} class="mr-2" />
                Add Server
              </button>
            {/if}
          </div>
        </div>
      </div>

      <!-- Stats Cards -->
      <div class="grid grid-cols-1 md:grid-cols-4 gap-6 mb-6">
        <div class="card">
          <div class="flex items-center">
            <div class="p-2 bg-blue-100 rounded-lg">
              <Database size={20} class="text-blue-600" />
            </div>
            <div class="ml-4">
              <p class="text-sm font-medium text-secondary">Total Servers</p>
              <p class="text-2xl font-bold text-primary">{stats.total}</p>
            </div>
          </div>
        </div>

        <div class="card">
          <div class="flex items-center">
            <div class="p-2 bg-green-100 rounded-lg">
              <CheckCircle size={20} class="text-green-600" />
            </div>
            <div class="ml-4">
              <p class="text-sm font-medium text-secondary">Connected</p>
              <p class="text-2xl font-bold text-primary">{stats.connected}</p>
            </div>
          </div>
        </div>

        <div class="card">
          <div class="flex items-center">
            <div class="p-2 bg-gray-100 rounded-lg">
              <Clock size={20} class="text-gray-600" />
            </div>
            <div class="ml-4">
              <p class="text-sm font-medium text-secondary">Disconnected</p>
              <p class="text-2xl font-bold text-primary">{stats.disconnected}</p>
            </div>
          </div>
        </div>

        <div class="card">
          <div class="flex items-center">
            <div class="p-2 bg-red-100 rounded-lg">
              <AlertCircle size={20} class="text-red-600" />
            </div>
            <div class="ml-4">
              <p class="text-sm font-medium text-secondary">Errors</p>
              <p class="text-2xl font-bold text-primary">{stats.error}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- Quick Start Cards (when NO profile active) -->
      {#if !shouldShowProfileView && servers.length > 0}
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
          <!-- Activate Profile Card -->
          <div class="card bg-gradient-to-br from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20 border-blue-200 dark:border-blue-800">
            <Zap size={32} class="text-blue-600 dark:text-blue-400 mb-3" />
            <h3 class="text-lg font-semibold mb-2 text-primary">🚀 Activate a Profile</h3>
            <p class="text-sm text-secondary mb-4">
              Group your servers and start them with one click. Perfect for switching between dev, test, and production environments.
            </p>
            <div class="flex gap-2">
              <button
                onclick={() => uiStore.setView('servers')}
                class="btn-primary text-sm"
              >
                <FolderOpen size={14} class="mr-2" />
                Browse Profiles
              </button>
            </div>
          </div>

          <!-- Add Server Card -->
          <div class="card bg-gradient-to-br from-green-50 to-emerald-50 dark:from-green-900/20 dark:to-emerald-900/20 border-green-200 dark:border-green-800">
            <Plus size={32} class="text-green-600 dark:text-green-400 mb-3" />
            <h3 class="text-lg font-semibold mb-2 text-primary">➕ Add Your First Server</h3>
            <p class="text-sm text-secondary mb-4">
              Connect to an MCP server via STDIO, HTTP, WebSocket, TCP, or Unix socket. Instantly access tools, resources, and prompts.
            </p>
            <button
              onclick={openAddServer}
              class="btn-primary text-sm"
            >
              <Plus size={14} class="mr-2" />
              Add Server
            </button>
          </div>
        </div>
      {/if}

      <!-- Main Content -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Server List -->
        <div class="lg:col-span-2">
          <div class="card">
            <div class="flex items-center justify-between mb-4">
              <h2 class="text-lg font-semibold text-primary">
                {#if shouldShowProfileView && activeProfile}
                  Profile Servers
                {:else}
                  All Servers
                {/if}
              </h2>
              {#if !shouldShowProfileView}
                <button
                  onclick={openAddServer}
                  class="text-sm text-mcp-primary-600 hover:text-mcp-primary-700"
                >
                  Add Server
                </button>
              {/if}
            </div>

            {#if displayServers().length === 0}
              {#if shouldShowProfileView}
                <div class="text-center py-12">
                  <Database size={48} class="mx-auto text-gray-400 mb-4" />
                  <h3 class="text-lg font-medium mb-2 text-primary">No servers in this profile</h3>
                  <p class="mb-4 text-secondary">Add servers to this profile to get started</p>
                  <button
                    onclick={() => uiStore.setView('servers')}
                    class="btn-primary"
                  >
                    <Settings size={16} class="mr-2" />
                    Manage Profile
                  </button>
                </div>
              {:else}
                <div class="text-center py-12">
                  <Database size={48} class="mx-auto text-gray-400 mb-4" />
                  <h3 class="text-lg font-medium mb-2 text-primary">No servers configured</h3>
                  <p class="mb-4 text-secondary">Get started by adding your first MCP server</p>
                  <button
                    onclick={openAddServer}
                    class="btn-primary"
                  >
                    <Plus size={16} class="mr-2" />
                    Add Server
                  </button>
                </div>
              {/if}
            {:else}
              <div class="space-y-3">
                {#each displayServers() as server}
                  <ServerOverview {server} />
                {/each}
              </div>
            {/if}
          </div>
        </div>

        <!-- Quick Actions -->
        <div class="space-y-6">
          <!-- Quick Start -->
          <div class="card">
            <h3 class="text-lg font-semibold mb-4 text-primary">Quick Start</h3>
            <div class="space-y-3">
              <button
                onclick={() => uiStore.setView('tools')}
                class="w-full flex items-center p-3 text-left rounded-lg transition-colors bg-surface-secondary text-primary"
                onmouseenter={(e: MouseEvent) => (e.target as HTMLElement).style.background = 'var(--mcp-surface-tertiary)'}
                onmouseleave={(e: MouseEvent) => (e.target as HTMLElement).style.background = 'var(--mcp-surface-secondary)'}
              >
                <Zap size={20} class="text-mcp-primary-600 mr-3" />
                <div>
                  <p class="font-medium text-primary">Explore Tools</p>
                  <p class="text-xs text-tertiary">Call MCP server tools</p>
                </div>
              </button>

              <button
                onclick={() => uiStore.setView('resources')}
                class="w-full flex items-center p-3 text-left rounded-lg transition-colors bg-surface-secondary text-primary"
                onmouseenter={(e: MouseEvent) => (e.target as HTMLElement).style.background = 'var(--mcp-surface-tertiary)'}
                onmouseleave={(e: MouseEvent) => (e.target as HTMLElement).style.background = 'var(--mcp-surface-secondary)'}
              >
                <Database size={20} class="text-mcp-primary-600 mr-3" />
                <div>
                  <p class="font-medium text-primary">Browse Resources</p>
                  <p class="text-xs text-tertiary">Access server resources</p>
                </div>
              </button>

              <button
                onclick={() => uiStore.setView('prompts')}
                class="w-full flex items-center p-3 text-left rounded-lg transition-colors bg-surface-secondary text-primary"
                onmouseenter={(e: MouseEvent) => (e.target as HTMLElement).style.background = 'var(--mcp-surface-tertiary)'}
                onmouseleave={(e: MouseEvent) => (e.target as HTMLElement).style.background = 'var(--mcp-surface-secondary)'}
              >
                <FileText size={20} class="text-mcp-primary-600 mr-3" />
                <div>
                  <p class="font-medium text-primary">Design Prompts</p>
                  <p class="text-xs text-tertiary">Create prompt templates</p>
                </div>
              </button>
            </div>
          </div>

          <!-- Selected Server Details -->
          {#if selectedServer}
            <div class="card">
              <h3 class="text-lg font-semibold mb-4 text-primary">
                {selectedServer?.config.name || 'Unknown Server'}
              </h3>
              <div class="space-y-3 text-sm">
                <div class="flex justify-between">
                  <span class="text-secondary">Status</span>
                  <span class="status-{selectedServer?.status || 'unknown'} capitalize">
                    {selectedServer?.status || 'unknown'}
                  </span>
                </div>
                <div class="flex justify-between">
                  <span class="text-secondary">Transport</span>
                  <span class="capitalize">{selectedServer.config.transport_config?.type || 'unknown'}</span>
                </div>
                {#if selectedServer.metrics}
                  <div class="flex justify-between">
                    <span class="text-secondary">Messages</span>
                    <span>{(selectedServer?.metrics?.requests_sent || 0) + (selectedServer?.metrics?.responses_received || 0)}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-secondary">Avg Response</span>
                    <span>{Math.round(selectedServer?.metrics?.avg_response_time_ms || 0)}ms</span>
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>

<!-- Modals -->
{#if modals.addServer}
  <AddServerModal />
{/if}

{#if modals.serverConfig}
  <ServerConfigModal />
{/if}