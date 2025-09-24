<script lang="ts">
  import { serverStore, getServerStatus, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore, type View } from '$lib/stores/uiStore';
  import ToolExplorer from './ToolExplorer.svelte';
  import ResourceBrowser from './ResourceBrowser.svelte';
  import PromptDesigner from './PromptDesigner.svelte';
  import SamplingTester from './SamplingTester.svelte';
  import ElicitationFlow from './ElicitationFlow.svelte';
  import CollectionsManager from './CollectionsManager.svelte';
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
    Clock
  } from 'lucide-svelte';

  let currentView: View = $state('dashboard');
  let servers: ServerInfo[] = $state([]);
  let selectedServerId: string | undefined = $state(undefined);
  let modals = $state({ addServer: false, serverConfig: false, toolCall: false });
  let error: string | undefined = $state(undefined);

  // Subscribe to stores
  $effect(() => {
    const unsubscribeUi = uiStore.subscribe(state => {
      currentView = state.currentView;
      modals = state.modals;
      error = state.error;
    });

    const unsubscribeServers = serverStore.subscribe(state => {
      servers = state.servers;
      selectedServerId = state.selectedServerId;
    });

    return () => {
      unsubscribeUi();
      unsubscribeServers();
    };
  });

  const selectedServer = $derived(
    servers.find(s => s.id === selectedServerId)
  );

  const connectedServers = $derived(
    servers.filter(s => getServerStatus(s) === 'connected')
  );

  const stats = $derived({
    total: servers.length,
    connected: connectedServers.length,
    disconnected: servers.filter(s => getServerStatus(s) === 'disconnected').length,
    error: servers.filter(s => getServerStatus(s) === 'error').length,
  });

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
  {#if currentView === 'dashboard'}
    <!-- Dashboard View -->
    <div class="h-full p-6 overflow-y-auto">
      <!-- Header -->
      <div class="mb-6">
        <div class="flex items-center justify-between">
          <div>
            <h1 class="text-2xl font-bold" style="color: var(--mcp-text-primary)">Dashboard</h1>
            <p style="color: var(--mcp-text-secondary)">Manage your MCP server connections and monitor activity</p>
          </div>
          <div class="flex space-x-3">
            <button
              onclick={refreshServers}
              class="btn-secondary"
            >
              <Activity size={16} class="mr-2" />
              Refresh
            </button>
            <button
              onclick={openAddServer}
              class="btn-primary"
            >
              <Plus size={16} class="mr-2" />
              Add Server
            </button>
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
              <p class="text-sm font-medium" style="color: var(--mcp-text-secondary)">Total Servers</p>
              <p class="text-2xl font-bold" style="color: var(--mcp-text-primary)">{stats.total}</p>
            </div>
          </div>
        </div>

        <div class="card">
          <div class="flex items-center">
            <div class="p-2 bg-green-100 rounded-lg">
              <CheckCircle size={20} class="text-green-600" />
            </div>
            <div class="ml-4">
              <p class="text-sm font-medium" style="color: var(--mcp-text-secondary)">Connected</p>
              <p class="text-2xl font-bold" style="color: var(--mcp-text-primary)">{stats.connected}</p>
            </div>
          </div>
        </div>

        <div class="card">
          <div class="flex items-center">
            <div class="p-2 bg-gray-100 rounded-lg">
              <Clock size={20} class="text-gray-600" />
            </div>
            <div class="ml-4">
              <p class="text-sm font-medium" style="color: var(--mcp-text-secondary)">Disconnected</p>
              <p class="text-2xl font-bold" style="color: var(--mcp-text-primary)">{stats.disconnected}</p>
            </div>
          </div>
        </div>

        <div class="card">
          <div class="flex items-center">
            <div class="p-2 bg-red-100 rounded-lg">
              <AlertCircle size={20} class="text-red-600" />
            </div>
            <div class="ml-4">
              <p class="text-sm font-medium" style="color: var(--mcp-text-secondary)">Errors</p>
              <p class="text-2xl font-bold" style="color: var(--mcp-text-primary)">{stats.error}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- Main Content -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Server List -->
        <div class="lg:col-span-2">
          <div class="card">
            <div class="flex items-center justify-between mb-4">
              <h2 class="text-lg font-semibold" style="color: var(--mcp-text-primary)">Servers</h2>
              <button
                onclick={openAddServer}
                class="text-sm text-mcp-primary-600 hover:text-mcp-primary-700"
              >
                Add Server
              </button>
            </div>

            {#if servers.length === 0}
              <div class="text-center py-12">
                <Database size={48} class="mx-auto text-gray-400 mb-4" />
                <h3 class="text-lg font-medium mb-2" style="color: var(--mcp-text-primary)">No servers configured</h3>
                <p class="mb-4" style="color: var(--mcp-text-secondary)">Get started by adding your first MCP server</p>
                <button
                  onclick={openAddServer}
                  class="btn-primary"
                >
                  <Plus size={16} class="mr-2" />
                  Add Server
                </button>
              </div>
            {:else}
              <div class="space-y-3">
                {#each servers as server}
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
            <h3 class="text-lg font-semibold mb-4" style="color: var(--mcp-text-primary)">Quick Start</h3>
            <div class="space-y-3">
              <button
                onclick={() => uiStore.setView('tools')}
                class="w-full flex items-center p-3 text-left rounded-lg transition-colors"
                style="background: var(--mcp-surface-secondary); color: var(--mcp-text-primary);"
                onmouseenter={(e) => e.target.style.background = 'var(--mcp-surface-tertiary)'}
                onmouseleave={(e) => e.target.style.background = 'var(--mcp-surface-secondary)'}
              >
                <Zap size={20} class="text-mcp-primary-600 mr-3" />
                <div>
                  <p class="font-medium" style="color: var(--mcp-text-primary)">Explore Tools</p>
                  <p class="text-xs" style="color: var(--mcp-text-tertiary)">Call MCP server tools</p>
                </div>
              </button>

              <button
                onclick={() => uiStore.setView('resources')}
                class="w-full flex items-center p-3 text-left rounded-lg transition-colors"
                style="background: var(--mcp-surface-secondary); color: var(--mcp-text-primary);"
                onmouseenter={(e) => e.target.style.background = 'var(--mcp-surface-tertiary)'}
                onmouseleave={(e) => e.target.style.background = 'var(--mcp-surface-secondary)'}
              >
                <Database size={20} class="text-mcp-primary-600 mr-3" />
                <div>
                  <p class="font-medium" style="color: var(--mcp-text-primary)">Browse Resources</p>
                  <p class="text-xs" style="color: var(--mcp-text-tertiary)">Access server resources</p>
                </div>
              </button>

              <button
                onclick={() => uiStore.setView('prompts')}
                class="w-full flex items-center p-3 text-left rounded-lg transition-colors"
                style="background: var(--mcp-surface-secondary); color: var(--mcp-text-primary);"
                onmouseenter={(e) => e.target.style.background = 'var(--mcp-surface-tertiary)'}
                onmouseleave={(e) => e.target.style.background = 'var(--mcp-surface-secondary)'}
              >
                <FileText size={20} class="text-mcp-primary-600 mr-3" />
                <div>
                  <p class="font-medium" style="color: var(--mcp-text-primary)">Design Prompts</p>
                  <p class="text-xs" style="color: var(--mcp-text-tertiary)">Create prompt templates</p>
                </div>
              </button>
            </div>
          </div>

          <!-- Selected Server Details -->
          {#if selectedServer}
            <div class="card">
              <h3 class="text-lg font-semibold mb-4" style="color: var(--mcp-text-primary)">
                {selectedServer?.config.name || 'Unknown Server'}
              </h3>
              <div class="space-y-3 text-sm">
                <div class="flex justify-between">
                  <span style="color: var(--mcp-text-secondary)">Status</span>
                  <span class="status-{selectedServer?.status || 'unknown'} capitalize">
                    {selectedServer?.status || 'unknown'}
                  </span>
                </div>
                <div class="flex justify-between">
                  <span style="color: var(--mcp-text-secondary)">Transport</span>
                  <span class="capitalize">{selectedServer.config.transport?.type || 'unknown'}</span>
                </div>
                {#if selectedServer.metrics}
                  <div class="flex justify-between">
                    <span style="color: var(--mcp-text-secondary)">Messages</span>
                    <span>{(selectedServer?.metrics?.messages_sent || 0) + (selectedServer?.metrics?.messages_received || 0)}</span>
                  </div>
                  <div class="flex justify-between">
                    <span style="color: var(--mcp-text-secondary)">Avg Response</span>
                    <span>{Math.round(selectedServer?.metrics?.avg_response_time_ms || 0)}ms</span>
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>

  {:else if currentView === 'tools'}
    <ToolExplorer />

  {:else if currentView === 'resources'}
    <ResourceBrowser />

  {:else if currentView === 'prompts'}
    <PromptDesigner />

  {:else if currentView === 'sampling'}
    <SamplingTester {selectedServerId} />

  {:else if currentView === 'elicitation'}
    <ElicitationFlow />

  {:else if currentView === 'collections'}
    <CollectionsManager />

  {:else}
    <!-- Other views placeholder -->
    <div class="h-full flex items-center justify-center">
      <div class="text-center">
        <h2 class="text-xl font-semibold mb-2" style="color: var(--mcp-text-primary)">
          {currentView.charAt(0).toUpperCase() + currentView.slice(1)}
        </h2>
        <p style="color: var(--mcp-text-secondary)">This view is coming soon!</p>
      </div>
    </div>
  {/if}
</div>

<!-- Modals -->
{#if modals.addServer}
  <AddServerModal />
{/if}

{#if modals.serverConfig}
  <ServerConfigModal />
{/if}