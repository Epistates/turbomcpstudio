<script lang="ts">
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore, type View } from '$lib/stores/uiStore';
  import { 
    Activity, 
    Database, 
    FileText, 
    MessageSquare, 
    Settings, 
    Plus,
    Zap,
    FolderOpen,
    Monitor
  } from 'lucide-svelte';

  let servers: ServerInfo[] = $state([]);
  let currentView: View = $state('dashboard');

  // Subscribe to stores
  $effect(() => {
    const unsubscribeServers = serverStore.subscribe(state => {
      servers = state.servers;
    });

    const unsubscribeUi = uiStore.subscribe(state => {
      currentView = state.currentView;
    });

    return () => {
      unsubscribeServers();
      unsubscribeUi();
    };
  });

  const navigationItems = [
    { id: 'dashboard', label: 'Dashboard', icon: Monitor },
    { id: 'tools', label: 'Tools', icon: Zap },
    { id: 'resources', label: 'Resources', icon: Database },
    { id: 'prompts', label: 'Prompts', icon: FileText },
    { id: 'sampling', label: 'Sampling', icon: MessageSquare },
    { id: 'elicitation', label: 'Elicitation', icon: Activity },
    { id: 'collections', label: 'Collections', icon: FolderOpen },
  ];

  function navigateTo(view: View) {
    uiStore.setView(view);
  }

  function getStatusColor(status: string) {
    switch (status) {
      case 'connected': return 'bg-green-400';
      case 'connecting': return 'bg-yellow-400';
      case 'error': return 'bg-red-400';
      default: return 'bg-gray-400';
    }
  }

  function selectServer(serverId: string) {
    serverStore.selectServer(serverId);
  }

  function addServer() {
    uiStore.openModal('addServer');
  }
</script>

<div class="h-full flex flex-col bg-white">
  <!-- Header -->
  <div class="p-4 border-b border-gray-200">
    <div class="flex items-center justify-between">
      <h1 class="text-lg font-semibold text-gray-900">MCP Studio</h1>
      <button
        onclick={addServer}
        class="p-1 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded"
        title="Add Server"
      >
        <Plus size={16} />
      </button>
    </div>
  </div>

  <!-- Navigation -->
  <nav class="p-4 space-y-1">
    {#each navigationItems as item}
      <button
        onclick={() => navigateTo(item.id as View)}
        class="nav-item w-full text-left {currentView === item.id ? 'nav-item-active' : 'nav-item-inactive'}"
      >
        {#if item.icon}
          {@const IconComponent = item.icon}
          <IconComponent size={16} class="mr-3" />
        {/if}
        {item.label}
      </button>
    {/each}
  </nav>

  <!-- Servers Section -->
  <div class="flex-1 p-4 overflow-y-auto">
    <div class="mb-3 flex items-center justify-between">
      <h2 class="text-sm font-medium text-gray-700">Connected Servers</h2>
      <span class="text-xs text-gray-500">{servers.length}</span>
    </div>

    {#if servers.length === 0}
      <div class="text-center py-8">
        <Database size={32} class="mx-auto text-gray-400 mb-2" />
        <p class="text-sm text-gray-500">No servers connected</p>
        <button
          onclick={addServer}
          class="mt-2 text-sm text-mcp-primary-600 hover:text-mcp-primary-700"
        >
          Add your first server
        </button>
      </div>
    {:else}
      <div class="space-y-2">
        {#each servers as server}
          <button
            onclick={() => selectServer(server.id)}
            class="w-full p-3 text-left bg-gray-50 hover:bg-gray-100 rounded-lg border border-gray-200 transition-colors"
          >
            <div class="flex items-start justify-between">
              <div class="flex-1 min-w-0">
                <div class="flex items-center mb-1">
                  <div class="w-2 h-2 rounded-full {getStatusColor(server.status)} mr-2"></div>
                  <h3 class="text-sm font-medium text-gray-900 truncate">
                    {server.config.name}
                  </h3>
                </div>
                {#if server.config.description}
                  <p class="text-xs text-gray-500 truncate">
                    {server.config.description}
                  </p>
                {/if}
                <div class="mt-1 flex items-center space-x-2 text-xs text-gray-400">
                  <span class="capitalize">{server.config.transport.type}</span>
                  {#if server.metrics}
                    <span>•</span>
                    <span>{server.metrics.messages_sent + server.metrics.messages_received} msgs</span>
                  {/if}
                </div>
              </div>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Settings -->
  <div class="p-4 border-t border-gray-200">
    <button
      onclick={() => navigateTo('settings')}
      class="nav-item w-full text-left {currentView === 'settings' ? 'nav-item-active' : 'nav-item-inactive'}"
    >
      <Settings size={16} class="mr-3" />
      Settings
    </button>
  </div>
</div>