<script lang="ts">
  import { serverStore, getServerStatus, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import { 
    Play, 
    Square, 
    Settings, 
    Activity, 
    Cpu, 
    MemoryStick,
    Clock,
    MessageCircle,
    AlertCircle,
    CheckCircle,
    Zap
  } from 'lucide-svelte';

  interface Props {
    server: ServerInfo;
  }

  let { server }: Props = $props();

  // Get safe status once to avoid multiple function calls
  const safeStatus = $derived(getServerStatus(server));

  function getStatusColor(status: string) {
    switch (status) {
      case 'connected': return 'text-green-600 bg-green-100';
      case 'connecting': return 'text-yellow-600 bg-yellow-100';
      case 'error': return 'text-red-600 bg-red-100';
      default: return 'text-gray-600 bg-gray-100';
    }
  }

  function getStatusIcon(status: string) {
    switch (status) {
      case 'connected': return CheckCircle;
      case 'connecting': return Clock;
      case 'error': return AlertCircle;
      default: return Activity;
    }
  }

  async function toggleConnection() {
    const safeStatus = getServerStatus(server);
    try {
      if (safeStatus === 'connected') {
        await serverStore.disconnectServer(server.id);
        uiStore.showSuccess(`Disconnected from ${server.config.name}`);
      } else {
        // For reconnection, pass the server config instead of the full server info
        await serverStore.connectServer(server.config);
        uiStore.showSuccess(`Connected to ${server.config.name}`);
      }
    } catch (error) {
      uiStore.showError(`Failed to ${safeStatus === 'connected' ? 'disconnect from' : 'connect to'} ${server.config.name}: ${error}`);
    }
  }

  function selectServer() {
    serverStore.selectServer(server.id);
    uiStore.setView('tools');
  }

  function openServerConfig() {
    serverStore.selectServer(server.id);
    uiStore.openModal('serverConfig');
  }

  function formatBytes(bytes: number) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function formatUptime(seconds: number) {
    if (!seconds || seconds < 60) return `${seconds || 0}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
    return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
  }

  function navigateToCapability(capability: string, e: Event) {
    e.stopPropagation(); // Prevent triggering the parent server selection

    // Set the server as selected first
    serverStore.selectServer(server.id);

    // Map capability to the corresponding view
    const viewMap: { [key: string]: string } = {
      'tools': 'tools',
      'resources': 'resources',
      'prompts': 'prompts',
      'sampling': 'sampling',
      'elicitation': 'elicitation'
    };

    const view = viewMap[capability.toLowerCase()];
    if (view) {
      uiStore.setView(view as any);
    }
  }
</script>

<div
  onclick={selectServer}
  role="button"
  tabindex="0"
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      selectServer();
    }
  }}
  class="block w-full border border-gray-200 rounded-lg p-4 hover:shadow-md transition-shadow bg-white cursor-pointer"
>
  <!-- Header -->
  <div class="flex items-start justify-between mb-3">
    <div class="flex-1">
      <div class="flex items-center mb-1">
        <h3 class="text-lg font-medium text-gray-900 hover:text-mcp-primary-600 transition-colors">
          {server.config.name || 'Unnamed Server'}
        </h3>
        <div class="ml-2 flex items-center px-2 py-1 rounded-full text-xs font-medium {getStatusColor(safeStatus)}">
          {#if safeStatus === 'connected'}
            <CheckCircle size={12} class="mr-1" />
          {:else if safeStatus === 'connecting'}
            <Clock size={12} class="mr-1" />
          {:else if safeStatus === 'error'}
            <AlertCircle size={12} class="mr-1" />
          {:else}
            <Activity size={12} class="mr-1" />
          {/if}
          {safeStatus}
        </div>
      </div>

      {#if server.config.description}
        <p class="text-sm text-gray-600 mb-2">{server.config.description}</p>
      {/if}

      <div class="flex items-center space-x-4 text-xs text-gray-500">
        <span class="flex items-center">
          <Activity size={12} class="mr-1" />
          {server.config.transport_config?.type?.toUpperCase() || 'UNKNOWN'}
        </span>

        {#if server.config.transport_config?.type === 'stdio' && server.process_info}
          <span class="flex items-center">
            <Cpu size={12} class="mr-1" />
            PID {server.process_info.pid}
          </span>
        {/if}

        {#if server.last_seen}
          <span class="flex items-center">
            <Clock size={12} class="mr-1" />
            {new Date(server.last_seen).toLocaleTimeString()}
          </span>
        {/if}
      </div>
    </div>

    <div class="flex items-center space-x-2">
      <button
        onclick={(e) => {
          e.stopPropagation();
          toggleConnection();
        }}
        class="p-2 rounded-lg {server.status === 'connected' ? 'text-red-600 hover:bg-red-50' : 'text-green-600 hover:bg-green-50'} transition-colors"
        title={server.status === 'connected' ? 'Disconnect' : 'Connect'}
      >
        {#if server.status === 'connected'}
          <Square size={16} />
        {:else}
          <Play size={16} />
        {/if}
      </button>

      <button
        onclick={(e) => {
          e.stopPropagation();
          openServerConfig();
        }}
        class="p-2 text-gray-600 hover:bg-gray-50 rounded-lg transition-colors"
        title="Settings"
      >
        <Settings size={16} />
      </button>
    </div>
  </div>

  <!-- Metrics -->
  {#if server.status === 'connected' && server.metrics}
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-3">
      <div class="bg-gray-50 rounded-lg p-3">
        <div class="flex items-center justify-between">
          <span class="text-xs text-gray-600">Messages</span>
          <MessageCircle size={12} class="text-gray-400" />
        </div>
        <p class="text-sm font-semibold text-gray-900 mt-1">
          {(server.metrics.requests_sent || 0) + (server.metrics.responses_received || 0)}
        </p>
        <p class="text-xs text-gray-500">
          ↑{server.metrics.requests_sent || 0} ↓{server.metrics.responses_received || 0}
        </p>
      </div>

      <div class="bg-gray-50 rounded-lg p-3">
        <div class="flex items-center justify-between">
          <span class="text-xs text-gray-600">Avg Response</span>
          <Zap size={12} class="text-gray-400" />
        </div>
        <p class="text-sm font-semibold text-gray-900 mt-1">
          {Math.round(server.metrics.avg_response_time_ms || 0)}ms
        </p>
      </div>

      <div class="bg-gray-50 rounded-lg p-3">
        <div class="flex items-center justify-between">
          <span class="text-xs text-gray-600">Data</span>
          <Activity size={12} class="text-gray-400" />
        </div>
        <p class="text-sm font-semibold text-gray-900 mt-1">
          {formatBytes((server.metrics.bytes_sent || 0) + (server.metrics.bytes_received || 0))}
        </p>
        <p class="text-xs text-gray-500">
          ↑{formatBytes(server.metrics.bytes_sent || 0)} ↓{formatBytes(server.metrics.bytes_received || 0)}
        </p>
      </div>

      <div class="bg-gray-50 rounded-lg p-3">
        <div class="flex items-center justify-between">
          <span class="text-xs text-gray-600">Uptime</span>
          <Clock size={12} class="text-gray-400" />
        </div>
        <p class="text-sm font-semibold text-gray-900 mt-1">
          {server.metrics.connected_at ? formatUptime(Math.floor((Date.now() - new Date(server.metrics.connected_at).getTime()) / 1000)) : '0s'}
        </p>
      </div>
    </div>
  {/if}

  <!-- Process Info -->
  {#if server.process_info && server.status === 'connected'}
    <div class="border-t border-gray-100 pt-3">
      <div class="grid grid-cols-2 gap-4 text-xs">
        <div class="flex items-center justify-between">
          <span class="text-gray-600">CPU Usage</span>
          <div class="flex items-center">
            <Cpu size={12} class="mr-1 text-gray-400" />
            <span class="font-medium">{server.process_info.cpu_usage.toFixed(1)}%</span>
          </div>
        </div>
        
        <div class="flex items-center justify-between">
          <span class="text-gray-600">Memory</span>
          <div class="flex items-center">
            <MemoryStick size={12} class="mr-1 text-gray-400" />
            <span class="font-medium">{formatBytes(server.process_info.memory_usage)}</span>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Capabilities -->
  {#if server.capabilities}
    <div class="border-t border-gray-100 pt-3 mt-3">
      <div class="flex items-center space-x-2">
        <span class="text-xs text-gray-600">Capabilities:</span>
        <div class="flex items-center space-x-1">
          {#if server.capabilities.tools}
            <button
              onclick={(e) => navigateToCapability('tools', e)}
              class="px-2 py-1 bg-blue-100 text-blue-700 text-xs rounded hover:bg-blue-200 hover:text-blue-800 transition-colors cursor-pointer"
              title="Go to Tools"
            >
              Tools
            </button>
          {/if}
          {#if server.capabilities.resources}
            <button
              onclick={(e) => navigateToCapability('resources', e)}
              class="px-2 py-1 bg-green-100 text-green-700 text-xs rounded hover:bg-green-200 hover:text-green-800 transition-colors cursor-pointer"
              title="Go to Resources"
            >
              Resources
            </button>
          {/if}
          {#if server.capabilities.prompts}
            <button
              onclick={(e) => navigateToCapability('prompts', e)}
              class="px-2 py-1 bg-purple-100 text-purple-700 text-xs rounded hover:bg-purple-200 hover:text-purple-800 transition-colors cursor-pointer"
              title="Go to Prompts"
            >
              Prompts
            </button>
          {/if}
          {#if server.capabilities.sampling}
            <button
              onclick={(e) => navigateToCapability('sampling', e)}
              class="px-2 py-1 bg-orange-100 text-orange-700 text-xs rounded hover:bg-orange-200 hover:text-orange-800 transition-colors cursor-pointer"
              title="Go to Sampling"
            >
              Sampling
            </button>
          {/if}
          {#if server.capabilities.elicitation}
            <button
              onclick={(e) => navigateToCapability('elicitation', e)}
              class="px-2 py-1 bg-pink-100 text-pink-700 text-xs rounded hover:bg-pink-200 hover:text-pink-800 transition-colors cursor-pointer"
              title="Go to Elicitation"
            >
              Elicitation
            </button>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- Error Display -->
  {#if server.status === 'error' && server.metrics.last_error}
    <div class="border-t border-gray-100 pt-3 mt-3">
      <div class="flex items-start">
        <AlertCircle size={14} class="text-red-500 mr-2 mt-0.5 flex-shrink-0" />
        <p class="text-xs text-red-600">{server.metrics.last_error}</p>
      </div>
    </div>
  {/if}
</div>