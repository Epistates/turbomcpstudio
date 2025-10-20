<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { uiStore } from '$lib/stores/uiStore';
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { profileStore } from '$lib/stores/profileStore';
  import { createLogger } from '$lib/utils/logger';
  import {
    X,
    ChevronRight,
    ChevronLeft,
    CheckCircle,
    AlertCircle,
    Loader,
    Download,
    Copy,
    ChevronDown,
  } from 'lucide-svelte';

  const logger = createLogger('InstallClientModal');

  interface Props {
    isOpen: boolean;
    onClose: () => void;
    servers: ServerInfo[];
    profiles: any[];
    localProfileServerMap: Map<string, Set<string>>;
  }

  let {
    isOpen,
    onClose,
    servers,
    profiles,
    localProfileServerMap,
  }: Props = $props();

  // Modal state
  let step = $state<1 | 2>(1);
  let selectedServers = $state<Set<string>>(new Set());
  let serverSelectionMode = $state<'all' | 'profiles' | 'individual'>('all');
  let selectedProfiles = $state<Set<string>>(new Set());
  let selectedClients = $state<Map<string, boolean>>(new Map());
  let isInstalling = $state(false);
  let installResults = $state<Record<string, any>>({});
  let detectedApps = $state<any[]>([]);
  let isDetecting = $state(false);
  let showCopyDropdown = $state(false);

  // Computed values - only count servers that will actually be installed (supported transports)
  const serverSelectionCount = $derived(() => {
    if (serverSelectionMode === 'all') {
      const supported = servers.filter((s) => isSupportedTransport(s)).length;
      logger.debug(`[serverSelectionCount] all mode: ${supported} supported of ${servers.length} total`);
      return supported;
    }
    if (serverSelectionMode === 'profiles' && selectedProfiles.size > 0) {
      // Deduplicate servers across multiple profiles, then filter supported
      const uniqueServerIds = new Set<string>();
      for (const profileId of selectedProfiles) {
        const profileServers = localProfileServerMap.get(profileId) || new Set();
        for (const serverId of profileServers) {
          uniqueServerIds.add(serverId);
        }
      }
      const profileServers = servers.filter((s) => uniqueServerIds.has(s.id));
      const supported = profileServers.filter((s) => isSupportedTransport(s)).length;
      logger.debug(`[serverSelectionCount] profiles mode: ${supported} supported of ${profileServers.length} total`);
      return supported;
    }
    // Individual mode: count only selected AND supported servers
    const individualServers = servers.filter((s) => selectedServers.has(s.id));
    const supported = individualServers.filter((s) => isSupportedTransport(s)).length;
    logger.debug(`[serverSelectionCount] individual mode: ${supported} supported of ${selectedServers.size} selected`);
    return supported;
  });

  const selectedClientsArray = $derived(
    Array.from(selectedClients.entries())
      .filter(([_, selected]) => selected)
      .map(([app]) => app)
  );

  // Reset modal state
  function resetModal() {
    step = 1;
    selectedServers = new Set();
    serverSelectionMode = 'all';
    selectedProfiles = new Set();
    selectedClients = new Map();
    isInstalling = false;
    installResults = {};
    showCopyDropdown = false;
  }

  // Close modal
  function handleClose() {
    resetModal();
    onClose();
  }

  // Detect installed clients on modal open
  async function detectClients() {
    if (detectedApps.length > 0) return;

    isDetecting = true;
    try {
      const results = await invoke<any[]>('detect_installed_clients');
      detectedApps = results;
      logger.info('Detected installed clients:', results);

      // Initialize with all apps unchecked (user must explicitly select)
      const newSelected = new Map<string, boolean>();
      for (const app of results) {
        newSelected.set(app.app, false);
      }
      selectedClients = newSelected;
    } catch (error) {
      logger.error('Failed to detect installed clients:', error);
      uiStore.showError('Failed to detect installed clients');
    } finally {
      isDetecting = false;
    }
  }

  // Check if a server uses a standardized transport compatible with other MCP clients
  // Mirrors the backend logic in client_install.rs for consistency
  function isSupportedTransport(server: ServerInfo): boolean {
    const transportType = server.config.transport_config?.type;
    const config = server.config.transport_config;

    logger.debug(`[isSupportedTransport] "${server.config.name}": type="${transportType}"`, config);

    // Explicitly check the type field
    if (transportType === 'tcp' || transportType === 'unix') {
      logger.debug(`  → false (TurboMCP extension: ${transportType})`);
      return false; // These are TurboMCP extensions, not standard
    }

    if (transportType === 'stdio' || transportType === 'http' || transportType === 'webSocket') {
      logger.debug(`  → true (standard MCP transport: ${transportType})`);
      return true; // These are standard MCP transports
    }

    // Fallback: if type is unclear, infer from transport config structure
    // (defensive check in case data structure differs)
    if (!config) {
      logger.debug(`  → false (no config)`);
      return false;
    }

    // Check by transport type discriminator
    if ('command' in config) {
      logger.debug(`  → true (has command, inferred stdio)`);
      return true; // stdio
    }
    if ('url' in config) {
      logger.debug(`  → true (has url, inferred http/webSocket)`);
      return true; // http or webSocket
    }
    if ('host' in config) {
      logger.debug(`  → false (has host, inferred tcp)`);
      return false; // tcp (unsupported)
    }
    if ('path' in config) {
      logger.debug(`  → false (has path, inferred unix socket)`);
      return false; // unix (unsupported)
    }

    // Unknown transport - fail safe to unsupported
    logger.debug(`  → false (unknown transport structure)`);
    return false;
  }

  // Get all servers available in current mode (regardless of selection state)
  // This determines what CAN be selected in this mode
  function getAvailableServersInMode(): ServerInfo[] {
    if (serverSelectionMode === 'all') {
      return servers;
    }

    if (serverSelectionMode === 'profiles') {
      if (selectedProfiles.size === 0) {
        // No profiles selected yet - nothing available
        return [];
      }
      // Servers from selected profiles (deduplicated)
      const allProfileServerIds = new Set<string>();
      for (const profileId of selectedProfiles) {
        const profileServers = localProfileServerMap.get(profileId) || new Set();
        for (const serverId of profileServers) {
          allProfileServerIds.add(serverId);
        }
      }
      return servers.filter((s) => allProfileServerIds.has(s.id));
    }

    if (serverSelectionMode === 'individual') {
      // Individual mode: all servers are "available" for selection
      // (but user might not have selected any yet)
      return servers;
    }

    return [];
  }

  // Get all selected servers (before filtering by transport support)
  // This is what the user has actually CHECKED/CHOSEN
  function getAllSelectedServers(): ServerInfo[] {
    const available = getAvailableServersInMode();

    if (serverSelectionMode === 'all' || serverSelectionMode === 'profiles') {
      // In these modes, everything available is automatically "selected" (checked)
      return available;
    }

    if (serverSelectionMode === 'individual') {
      // In individual mode, only user-checked servers are selected
      return available.filter((s) => selectedServers.has(s.id));
    }

    return [];
  }

  // Get all selected servers for clipboard copy (NO FILTERING - includes all transports)
  function getAllSelectedServersForCopy(): ServerInfo[] {
    return getAllSelectedServers();
  }

  // Build server list to install (only supported transports)
  function getServersToInstall(): ServerInfo[] {
    return getAllSelectedServers().filter((s) => isSupportedTransport(s));
  }

  // Get list of unsupported servers for display (warnings)
  function getUnsupportedServers(): ServerInfo[] {
    return getAllSelectedServers().filter((s) => !isSupportedTransport(s));
  }

  // Get total count of selected servers (for UI display)
  const totalSelectedCount = $derived(() => {
    return getAllSelectedServers().length;
  });

  // Get count of supported servers only (for client install)
  const supportedSelectedCount = $derived(() => {
    return getServersToInstall().length;
  });

  // Check if there are ANY supported servers available in current mode
  function hasAnySupportedServersAvailable(): boolean {
    const available = getAvailableServersInMode();
    return available.some((s) => isSupportedTransport(s));
  }

  // Convert server to MCP config format (same logic as ServerManagement)
  function serverToMcpConfig(server: ServerInfo): Record<string, any> {
    const config = server.config;
    const mcpConfig: any = {};

    if (config.transport_config?.type === 'stdio') {
      mcpConfig.command = config.transport_config.command;
      if (config.transport_config.args && config.transport_config.args.length > 0) {
        mcpConfig.args = config.transport_config.args;
      }
      if (config.environment_variables && Object.keys(config.environment_variables).length > 0) {
        mcpConfig.env = config.environment_variables;
      }
    } else if (config.transport_config?.type === 'http') {
      // HTTP transport: use URL exactly as configured by user
      mcpConfig.url = config.transport_config.url;
      if (config.transport_config.headers && Object.keys(config.transport_config.headers).length > 0) {
        mcpConfig.headers = config.transport_config.headers;
      }
    } else if (config.transport_config?.type === 'webSocket') {
      // WebSocket transport: use URL exactly as configured by user
      mcpConfig.url = config.transport_config.url;
      if (config.transport_config.headers && Object.keys(config.transport_config.headers).length > 0) {
        mcpConfig.headers = config.transport_config.headers;
      }
    } else if (config.transport_config?.type === 'tcp') {
      mcpConfig.host = config.transport_config.host;
      mcpConfig.port = config.transport_config.port;
      mcpConfig.transport = 'tcp';
    } else if (config.transport_config?.type === 'unix') {
      mcpConfig.path = config.transport_config.path;
      mcpConfig.transport = 'unix';
    }

    return mcpConfig;
  }

  // Copy selected servers to clipboard (ALL servers, including unsupported transports)
  async function handleCopyToClipboard(format: 'full' | 'keys-only' = 'full') {
    const serversToCopy = getAllSelectedServersForCopy();
    if (serversToCopy.length === 0) {
      uiStore.showError('No servers selected to copy');
      return;
    }

    try {
      // Build MCP config object - includes ALL transports (stdio, http, webSocket, tcp, unix)
      const mcpServersConfig: Record<string, any> = {};
      for (const server of serversToCopy) {
        mcpServersConfig[server.config.name] = serverToMcpConfig(server);
      }

      let jsonString: string;
      if (format === 'keys-only') {
        // Keys-only: Just the raw server configs (ready to paste into existing mcpServers object)
        // Format each server entry as "name": {...} and join with commas
        const entries = Object.entries(mcpServersConfig).map(([name, config]) => {
          return `  ${JSON.stringify(name)}: ${JSON.stringify(config, null, 2).split('\n').join('\n  ')}`;
        });
        jsonString = entries.join(',\n');
      } else {
        // Full format: Wrapped in { mcpServers: { ... } }
        const fullConfig = {
          mcpServers: mcpServersConfig
        };
        jsonString = JSON.stringify(fullConfig, null, 2);
      }

      await navigator.clipboard.writeText(jsonString);

      const formatLabel = format === 'keys-only' ? ' (keys only)' : '';
      uiStore.showSuccess(`Copied ${serversToCopy.length} server${serversToCopy.length !== 1 ? 's' : ''}${formatLabel} to clipboard`);
      logger.debug('📋 Copied servers to clipboard:', jsonString);

      // Close modal after copying
      showCopyDropdown = false;
      handleClose();
    } catch (error) {
      logger.error('Failed to copy to clipboard:', error);
      uiStore.showError('Failed to copy to clipboard');
    }
  }

  // Perform installation
  async function handleInstall() {
    if (selectedClientsArray.length === 0) {
      uiStore.showError('Please select at least one client application');
      return;
    }

    const serversToInstall = getServersToInstall();
    if (serversToInstall.length === 0) {
      uiStore.showError('Please select at least one server');
      return;
    }

    isInstalling = true;
    installResults = {};

    try {
      // Convert servers to MCP format
      const installPayload = serversToInstall.map((server) => ({
        name: server.config.name,
        config: serverToMcpConfig(server),
      }));

      logger.info('Installation payload being sent to backend:', JSON.stringify(installPayload, null, 2));

      // Install to each selected client
      for (const appName of selectedClientsArray) {
        try {
          const result = await invoke('install_servers_to_client', {
            appName,
            servers: installPayload,
          });
          installResults[appName] = { ...(result as object), status: 'success' };
          logger.info(`Successfully installed to ${appName}:`, result);
        } catch (error) {
          installResults[appName] = {
            app: appName,
            status: 'error',
            message: String(error),
            success: false,
          };
          logger.error(`Failed to install to ${appName}:`, error);
        }
      }

      // Show summary
      const successCount = Object.values(installResults).filter(
        (r: any) => r.status === 'success'
      ).length;
      const totalApps = selectedClientsArray.length;

      if (successCount === totalApps) {
        uiStore.showSuccess(
          `Successfully installed servers to ${successCount}/${totalApps} applications`
        );
      } else {
        uiStore.showWarning(
          `Installed servers to ${successCount}/${totalApps} applications (${totalApps - successCount} failed)`
        );
      }

      // Move to results step
      step = 3 as any;
    } catch (error) {
      logger.error('Installation error:', error);
      uiStore.showError('Installation process failed');
    } finally {
      isInstalling = false;
    }
  }

  // Initialize on modal open
  $effect(() => {
    if (isOpen) {
      // Debug: log server structure to help diagnose transport detection issues
      logger.debug('InstallClientModal opened with servers:', servers);
      servers.forEach((s) => {
        logger.debug(`Server "${s.config.name}":`, {
          transportType: s.config.transport_config?.type,
          config: s.config.transport_config,
          supported: isSupportedTransport(s),
        });
      });

      if (detectedApps.length === 0) {
        detectClients();
      }
    }
  });
</script>

{#if isOpen}
  <div
    class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
    onclick={(e) => {
      if (e.target === e.currentTarget) {
        showCopyDropdown = false;
      }
    }}
  >
    <div
      class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-hidden flex flex-col"
      onclick={() => showCopyDropdown = false}
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">
          Export Server Configurations
        </h2>
        <button
          onclick={handleClose}
          class="p-2 text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
        >
          <X size={20} />
        </button>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6">
        {#if step === 1}
          <!-- Step 1: Select Servers -->
          <div class="space-y-4">
            <h3 class="font-medium text-gray-900 dark:text-white mb-4">Step 1: Select Servers</h3>

            <div class="space-y-3">
              <label class="flex items-center gap-3 p-3 border-2 rounded-lg cursor-pointer transition-all {serverSelectionMode === 'all'
                ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                : 'border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700/50'}">
                <input
                  type="radio"
                  name="selection-mode"
                  value="all"
                  bind:group={serverSelectionMode}
                  class="w-4 h-4 accent-blue-600"
                />
                <div>
                  <div class="font-medium text-gray-900 dark:text-white">All Servers</div>
                  <div class="text-sm text-gray-500 dark:text-gray-400">
                    {servers.length} total ({servers.filter((s) => isSupportedTransport(s)).length} client-compatible)
                  </div>
                </div>
              </label>

              <label class="flex items-center gap-3 p-3 border-2 rounded-lg cursor-pointer transition-all {serverSelectionMode === 'profiles'
                ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                : 'border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700/50'}">
                <input
                  type="radio"
                  name="selection-mode"
                  value="profiles"
                  bind:group={serverSelectionMode}
                  class="w-4 h-4 accent-blue-600"
                />
                <div class="flex-1">
                  <div class="font-medium text-gray-900 dark:text-white">Choose Profiles</div>
                  <div class="text-sm text-gray-500 dark:text-gray-400">
                    {profiles.length} profile{profiles.length !== 1 ? 's' : ''} available
                    {selectedProfiles.size > 0 ? `(${selectedProfiles.size} selected)` : ''}
                  </div>
                </div>
              </label>

              {#if serverSelectionMode === 'profiles'}
                <div class="ml-7 space-y-2 max-h-48 overflow-y-auto p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg border border-gray-200 dark:border-gray-700">
                  {#each profiles as profile}
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={selectedProfiles.has(profile.id)}
                        onchange={(e) => {
                          const newSet = new Set(selectedProfiles);
                          if (e.currentTarget.checked) {
                            newSet.add(profile.id);
                          } else {
                            newSet.delete(profile.id);
                          }
                          selectedProfiles = newSet;
                        }}
                        class="w-4 h-4"
                      />
                      <span class="text-sm text-gray-900 dark:text-white">
                        {profile.name}
                        <span class="text-gray-500 dark:text-gray-400 text-xs">
                          ({localProfileServerMap.get(profile.id)?.size || 0} servers)
                        </span>
                      </span>
                    </label>
                  {/each}
                </div>
              {/if}

              <label class="flex items-center gap-3 p-3 border-2 rounded-lg cursor-pointer transition-all {serverSelectionMode === 'individual'
                ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                : 'border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700/50'}">
                <input
                  type="radio"
                  name="selection-mode"
                  value="individual"
                  bind:group={serverSelectionMode}
                  class="w-4 h-4 accent-blue-600"
                />
                <div>
                  <div class="font-medium text-gray-900 dark:text-white">Individual Servers</div>
                  <div class="text-sm text-gray-500 dark:text-gray-400">
                    {selectedServers.size} selected ({servers.filter((s) => isSupportedTransport(s)).length} of {servers.length} client-compatible)
                  </div>
                </div>
              </label>

              {#if serverSelectionMode === 'individual'}
                <div class="ml-7 space-y-2 max-h-64 overflow-y-auto p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg border border-gray-200 dark:border-gray-700">
                  {#each servers as server}
                    {@const supported = isSupportedTransport(server)}
                    <div class="group">
                      <label class="flex items-center gap-2 cursor-pointer">
                        <input
                          type="checkbox"
                          checked={selectedServers.has(server.id)}
                          onchange={(e) => {
                            const newSet = new Set(selectedServers);
                            if (e.currentTarget.checked) {
                              newSet.add(server.id);
                            } else {
                              newSet.delete(server.id);
                            }
                            selectedServers = newSet;
                          }}
                          class="w-4 h-4"
                        />
                        <span class="text-sm text-gray-900 dark:text-white">
                          {server.config.name}
                          {#if !supported}
                            <span class="text-xs text-yellow-600 dark:text-yellow-400 ml-1">⚠️</span>
                          {/if}
                        </span>
                      </label>
                      {#if !supported}
                        <div class="ml-6 text-xs text-gray-500 dark:text-gray-400 mt-1 p-2 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-700 rounded">
                          ⚠️ Uses {server.config.transport_config?.type} (non-standard transport, cannot be installed to clients)
                        </div>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
            </div>

            {#if totalSelectedCount() > 0}
              <div class="mt-6 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                <p class="text-sm text-blue-900 dark:text-blue-200">
                  <span class="font-medium">📋 Selected:</span> {totalSelectedCount()} server{totalSelectedCount() !== 1 ? 's' : ''}
                  {#if supportedSelectedCount() < totalSelectedCount()}
                    <span class="text-xs text-yellow-700 dark:text-yellow-300">
                      ({supportedSelectedCount()} client-compatible, {getUnsupportedServers().length} clipboard-only)
                    </span>
                  {:else}
                    <span class="text-xs text-green-700 dark:text-green-300">
                      (all client-compatible)
                    </span>
                  {/if}
                </p>
              </div>
            {/if}

            {#if getUnsupportedServers().length > 0}
              <div class="mt-4 p-3 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg">
                <p class="text-sm text-yellow-900 dark:text-yellow-200">
                  <span class="font-medium">⚠️ {getUnsupportedServers().length} server{getUnsupportedServers().length !== 1 ? 's' : ''} cannot be installed to clients:</span>
                </p>
                <ul class="text-xs text-yellow-800 dark:text-yellow-300 mt-2 ml-4 space-y-1">
                  {#each getUnsupportedServers() as server}
                    <li>• {server.config.name} ({server.config.transport_config?.type})</li>
                  {/each}
                </ul>
                <p class="text-xs text-yellow-700 dark:text-yellow-400 mt-2">
                  These use TurboMCP-specific transports (TCP/Unix sockets) not supported by standard MCP clients.
                  <strong>They can still be copied to clipboard.</strong>
                </p>
              </div>
            {/if}
          </div>
        {/if}

        {#if step === 2}
          <!-- Step 2: Select Clients -->
          <div class="space-y-4">
            <h3 class="font-medium text-gray-900 dark:text-white mb-4">Step 2: Select Application(s)</h3>

            {#if isDetecting}
              <div class="flex items-center justify-center py-8">
                <Loader size={20} class="animate-spin text-gray-500 mr-2" />
                <span class="text-sm text-gray-600 dark:text-gray-400">Detecting installed applications...</span>
              </div>
            {:else if detectedApps.length === 0}
              <div class="p-4 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg">
                <p class="text-sm text-yellow-900 dark:text-yellow-200">No applications detected. Select manually:</p>
              </div>
            {/if}

            <div class="space-y-2 max-h-64 overflow-y-auto">
              {#each detectedApps as app}
                <label
                  class="flex items-center gap-3 p-3 border rounded-lg cursor-pointer transition-colors {selectedClients.get(
                    app.app
                  )
                    ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                    : app.installed
                      ? 'border-green-300 dark:border-green-700 hover:bg-gray-50 dark:hover:bg-gray-700/50'
                      : 'border-gray-300 dark:border-gray-600 opacity-60 hover:bg-gray-50 dark:hover:bg-gray-700/50'}"
                >
                  <input
                    type="checkbox"
                    checked={selectedClients.get(app.app) || false}
                    onchange={(e) => {
                      const newMap = new Map(selectedClients);
                      newMap.set(app.app, e.currentTarget.checked);
                      selectedClients = newMap;
                    }}
                    disabled={!app.installed}
                    class="w-4 h-4"
                  />
                  <div class="flex-1">
                    <div class="font-medium text-gray-900 dark:text-white">{app.app}</div>
                    <div class="text-xs text-gray-500 dark:text-gray-400 font-mono">
                      {app.config_path || 'Not found'}
                    </div>
                  </div>
                  <div>
                    {#if app.installed}
                      <CheckCircle size={16} class="text-green-600 dark:text-green-400" />
                    {:else}
                      <AlertCircle size={16} class="text-gray-400 dark:text-gray-600" />
                    {/if}
                  </div>
                </label>
              {/each}
            </div>

            <div class="mt-6 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
              <p class="text-sm text-blue-900 dark:text-blue-200">
                <span class="font-medium">Selected:</span> {selectedClientsArray.length}
                {selectedClientsArray.length === 1 ? 'application' : 'applications'}
              </p>
            </div>
          </div>
        {/if}

        {#if step === 3 as number}
          <!-- Step 3: Installation Results -->
          <div class="space-y-4">
            <h3 class="font-medium text-gray-900 dark:text-white mb-4">Installation Results</h3>

            <div class="space-y-2">
              {#each Object.entries(installResults) as [app, result]}
                <div
                  class="p-3 border rounded-lg {result.status === 'success'
                    ? 'border-green-300 dark:border-green-700 bg-green-50 dark:bg-green-900/20'
                    : 'border-red-300 dark:border-red-700 bg-red-50 dark:bg-red-900/20'}"
                >
                  <div class="flex items-start gap-3">
                    <div class="mt-0.5">
                      {#if result.status === 'success'}
                        <CheckCircle size={18} class="text-green-600 dark:text-green-400" />
                      {:else}
                        <AlertCircle size={18} class="text-red-600 dark:text-red-400" />
                      {/if}
                    </div>
                    <div class="flex-1 min-w-0">
                      <div class="font-medium text-gray-900 dark:text-white">{app}</div>
                      <div class="text-sm text-gray-600 dark:text-gray-300 mt-1">
                        {result.message}
                      </div>
                      {#if result.servers_added !== undefined || result.servers_updated !== undefined}
                        <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                          {#if result.servers_added > 0}
                            <span class="text-green-700 dark:text-green-300">+{result.servers_added} added</span>
                          {/if}
                          {#if result.servers_updated > 0}
                            <span class="text-blue-700 dark:text-blue-300 ml-2">~{result.servers_updated} updated</span>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  </div>
                </div>
              {/each}
            </div>

            <div class="mt-6 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
              <p class="text-sm text-blue-900 dark:text-blue-200">
                ✅ Installation complete! Your servers have been added to the selected applications.
              </p>
            </div>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between p-6 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900 relative">
        <div class="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
          {#if (step as number) !== 3}
            <span>Step {step} of 2</span>
          {/if}
        </div>

        <div class="flex gap-3 relative z-10">
          {#if (step as number) !== 3}
            <button
              onclick={() => {
                if (step === 1) {
                  handleClose();
                } else {
                  step = 1 as any;
                }
              }}
              class="px-4 py-2 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
            >
              {step === 1 ? 'Cancel' : 'Back'}
            </button>

            {#if step === 1}
              <!-- Step 1: Two options - Copy to Clipboard OR Add to Client -->
              <div class="relative" onclick={(e) => e.stopPropagation()}>
                <div class="flex">
                  <button
                    onclick={() => handleCopyToClipboard('full')}
                    disabled={totalSelectedCount() === 0}
                    class="px-4 py-2 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-l-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                    title={totalSelectedCount() === 0 ? 'No servers selected' : `Copy ${totalSelectedCount()} server${totalSelectedCount() !== 1 ? 's' : ''} (full format with mcpServers wrapper)`}
                  >
                    <Copy size={16} />
                    Copy to Clipboard
                  </button>
                  <button
                    onclick={(e) => {
                      e.stopPropagation();
                      showCopyDropdown = !showCopyDropdown;
                    }}
                    disabled={totalSelectedCount() === 0}
                    class="px-2 py-2 text-gray-700 dark:text-gray-300 border border-l-0 border-gray-300 dark:border-gray-600 rounded-r-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    title="Copy format options"
                  >
                    <ChevronDown size={16} />
                  </button>
                </div>

                {#if showCopyDropdown}
                  <div class="absolute left-0 bottom-full mb-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg z-20 min-w-[250px] overflow-hidden">
                    <button
                      onclick={(e) => {
                        e.stopPropagation();
                        handleCopyToClipboard('full');
                      }}
                      class="w-full text-left px-4 py-2.5 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors border-b border-gray-200 dark:border-gray-700 first:rounded-t-lg"
                    >
                      <div class="font-medium text-sm text-gray-900 dark:text-white">Full Format</div>
                      <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                        <code class="bg-gray-100 dark:bg-gray-700 px-1 rounded">{"{ \"mcpServers\": { ... } }"}</code>
                      </div>
                    </button>
                    <button
                      onclick={(e) => {
                        e.stopPropagation();
                        handleCopyToClipboard('keys-only');
                      }}
                      class="w-full text-left px-4 py-2.5 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors last:rounded-b-lg"
                    >
                      <div class="font-medium text-sm text-gray-900 dark:text-white">Keys Only</div>
                      <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                        Just server configs (paste into existing)
                      </div>
                    </button>
                  </div>
                {/if}
              </div>

              <button
                onclick={() => {
                  if (supportedSelectedCount() === 0) {
                    uiStore.showError('No client-compatible servers selected');
                    return;
                  }
                  step = 2 as any;
                }}
                disabled={supportedSelectedCount() === 0}
                class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                title={supportedSelectedCount() === 0
                  ? (totalSelectedCount() > 0
                    ? `Cannot install: ${getUnsupportedServers().map(s => `${s.config.name} (${s.config.transport_config?.type})`).join(', ')} - non-standard transport${getUnsupportedServers().length !== 1 ? 's' : ''} not supported by MCP clients`
                    : 'No servers selected')
                  : `Install ${supportedSelectedCount()} client-compatible server${supportedSelectedCount() !== 1 ? 's' : ''}`}
              >
                {#if totalSelectedCount() > supportedSelectedCount()}
                  Add to Client ({supportedSelectedCount()}/{totalSelectedCount()})
                {:else}
                  Add to Client
                {/if}
                <ChevronRight size={16} />
              </button>
            {:else}
              <!-- Step 2: Install button -->
              <button
                onclick={handleInstall}
                disabled={isInstalling || selectedClientsArray.length === 0}
                class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
              >
                {#if isInstalling}
                  <Loader size={16} class="animate-spin" />
                  Installing...
                {:else}
                  <Download size={16} />
                  Install
                {/if}
              </button>
            {/if}
          {:else}
            <button
              onclick={handleClose}
              class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              Done
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
