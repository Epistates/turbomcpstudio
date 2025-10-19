<!--
  Server Context Bar Component

  Enterprise-grade server selection UI that provides clear operational context.
  Shows which server the user is currently targeting for operations.

  Design Philosophy:
  - Always visible on operational tabs (Tools, Resources, Prompts, etc.)
  - Crystal clear "WHAT am I working with?" indicator
  - Quick server switching without leaving current tab
  - Profile context awareness
  - Connection state handling
-->
<script lang="ts">
  import { contextStore, serverHasCapability, type ServerContext } from '$lib/stores/contextStore';
  import { uiStore } from '$lib/stores/uiStore';
  import { profileStore } from '$lib/stores/profileStore';
  import {
    Server,
    ChevronDown,
    RefreshCw,
    Settings,
    AlertCircle,
    CheckCircle,
    Clock,
    Search,
    Folder,
    Zap
  } from 'lucide-svelte';

  // Props
  interface Props {
    requiredCapability?: 'tools' | 'resources' | 'prompts' | 'sampling' | 'elicitation' | null;
    mode?: 'selector' | 'filter';
    compact?: boolean;
  }

  let { requiredCapability = null, mode = 'selector', compact = false }: Props = $props();

  // Derived label based on mode
  const label = $derived(mode === 'filter' ? '🔍 Filter:' : '🎯 Testing:');

  // Dropdown state
  let isDropdownOpen = $state(false);
  let searchQuery = $state('');
  let dropdownRef: HTMLDivElement | undefined = $state();

  // Get the context and derive values from it
  const context = $derived($contextStore) as ServerContext;
  const selectedServer = $derived(context.selectedServer);
  const activeProfile = $derived(context.activeProfile);
  const isFromActiveProfile = $derived(context.isFromActiveProfile);
  const lastUpdated = $derived(context.lastUpdated);
  const availableServers = $derived(context.availableServers);

  // Apply capability filter
  const capableServers = $derived(
    requiredCapability
      ? availableServers.filter(s => serverHasCapability(s, requiredCapability))
      : availableServers
  );

  // Debug logging for capability filtering
  $effect(() => {
    console.log('[ServerContextBar] Capability filtering:', {
      requiredCapability,
      availableServers: availableServers.length,
      capableServers: capableServers.length,
      availableServerDetails: availableServers.map(s => ({
        name: s.config.name,
        status: s.status,
        hasCapabilities: !!s.capabilities,
        capabilities: s.capabilities ? Object.keys(s.capabilities) : []
      })),
      capableServerNames: capableServers.map(s => s.config.name)
    });
  });

  // Apply search filter
  const filteredServers = $derived(
    searchQuery.trim()
      ? capableServers.filter(s =>
          s.config.name.toLowerCase().includes(searchQuery.toLowerCase())
        )
      : capableServers
  );

  // Group servers by profile
  const serverGroups = $derived(() => {
    // Get current filtered servers
    const currentFilteredServers = filteredServers;
    const currentActiveProfile = activeProfile;

    if (!currentActiveProfile) {
      return [{ name: 'All Servers', servers: currentFilteredServers, isProfile: false }];
    }

    const profileStore$ = $profileStore;
    const profileServerIds = new Set(profileStore$.activeProfile?.servers?.map(ps => ps.server_id) || []);

    const profileServers = currentFilteredServers.filter(s => profileServerIds.has(s.id));
    const standaloneServers = currentFilteredServers.filter(s => !profileServerIds.has(s.id));

    const groups = [];

    if (profileServers.length > 0) {
      groups.push({
        name: `${currentActiveProfile.icon || '📁'} ${currentActiveProfile.name}`,
        servers: profileServers,
        isProfile: true,
      });
    }

    if (standaloneServers.length > 0) {
      groups.push({
        name: 'Other Servers',
        servers: standaloneServers,
        isProfile: false,
      });
    }

    return groups;
  });

  // Connection status styling
  const statusConfig = $derived(() => {
    switch (context.connectionStatus) {
      case 'connected':
        return { color: 'status-connected', icon: CheckCircle, text: 'Connected' };
      case 'connecting':
        return { color: 'status-connecting', icon: Clock, text: 'Connecting' };
      case 'error':
        return { color: 'status-error', icon: AlertCircle, text: 'Error' };
      default:
        return { color: 'status-disconnected', icon: Server, text: 'Disconnected' };
    }
  });

  // Actions
  function toggleDropdown() {
    isDropdownOpen = !isDropdownOpen;
    if (isDropdownOpen) {
      searchQuery = '';
    }
  }

  function selectServer(serverId: string) {
    contextStore.selectServer(serverId);
    isDropdownOpen = false;
  }

  function handleOpenServerManager() {
    isDropdownOpen = false;
    uiStore.setView('servers');
  }

  function handleReconnect() {
    if (selectedServer) {
      // Trigger reconnection logic
      // This will be handled by the server management
    }
  }

  // Close dropdown on outside click
  function handleClickOutside(event: MouseEvent) {
    if (dropdownRef && !dropdownRef.contains(event.target as Node)) {
      isDropdownOpen = false;
    }
  }

  $effect(() => {
    if (isDropdownOpen) {
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  // Capability badge helper
  function getCapabilityBadges(server: any) {
    const badges = [];
    if (server.capabilities?.tools) badges.push('T');
    if (server.capabilities?.resources) badges.push('R');
    if (server.capabilities?.prompts) badges.push('P');
    if (server.capabilities?.sampling) badges.push('S');
    if (server.capabilities?.elicitation) badges.push('E');
    return badges;
  }

  // Clear selection (for filter mode)
  function clearSelection() {
    contextStore.clearSelection();
    isDropdownOpen = false;
  }
</script>

<div class="server-context-bar" class:compact bind:this={dropdownRef}>
  {#if selectedServer}
    <!-- Selected Server Display -->
    <div class="server-display">
      <span class="server-label" class:compact>{label}</span>

      <button class="server-selector" onclick={toggleDropdown}>
        <span class="server-status {statusConfig().color}">
          <svelte:component this={statusConfig().icon} size={14} />
        </span>
        <span class="server-name">{selectedServer.config.name}</span>
        <span class="capability-badges">
          {#each getCapabilityBadges(selectedServer) as badge}
            <span class="capability-badge">{badge}</span>
          {/each}
        </span>
        <ChevronDown size={14} class="dropdown-icon" />
      </button>

      {#if !compact}
        <div class="server-actions">
          {#if mode === 'filter'}
            <button class="action-btn clear" onclick={clearSelection} title="Clear Filter">
              <span>Clear</span>
            </button>
          {/if}
          {#if context.connectionStatus === 'error'}
            <button class="action-btn reconnect" onclick={handleReconnect} title="Reconnect">
              <RefreshCw size={14} />
              <span>Retry</span>
            </button>
          {/if}
          <button class="action-btn settings" onclick={handleOpenServerManager} title="Server Manager">
            <Settings size={14} />
          </button>
        </div>
      {/if}
    </div>

    <!-- Profile Context -->
    {#if isFromActiveProfile && activeProfile && !compact}
      <div class="profile-context">
        <span class="profile-badge">
          <Zap size={12} />
          <span>from {activeProfile.name}</span>
        </span>
      </div>
    {/if}
  {:else}
    <!-- No Server Selected -->
    <div class="server-display empty">
      <AlertCircle size={16} class="warning-icon" />
      <span class="empty-text">No server selected</span>
      <button class="btn-select-server" onclick={toggleDropdown}>
        Select a server
        <ChevronDown size={14} />
      </button>
    </div>
  {/if}

  <!-- Server Dropdown -->
  {#if isDropdownOpen}
    <div class="server-dropdown">
      <!-- Search -->
      {#if filteredServers.length > 5}
        <div class="dropdown-search">
          <Search size={14} />
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search servers... (⌘K)"
            class="search-input"
          />
        </div>
      {/if}

      <!-- Server Groups -->
      <div class="dropdown-content">
        {#if filteredServers.length === 0}
          <div class="dropdown-empty">
            <Server size={32} />
            <p>No {requiredCapability || 'capable'} servers connected</p>
            <button class="btn-manage" onclick={handleOpenServerManager}>
              Go to Server Manager →
            </button>
          </div>
        {:else}
          {#each serverGroups() as group}
            <div class="server-group">
              <div class="group-header">
                {#if group.isProfile}
                  <Folder size={12} />
                {/if}
                <span>{group.name}</span>
                <span class="group-count">({group.servers.length})</span>
              </div>

              {#each group.servers as server}
                {@const isSelected = selectedServer && server.id === selectedServer.id}
                <button
                  class="server-item"
                  class:selected={isSelected}
                  onclick={() => selectServer(server.id)}
                  disabled={isSelected}
                >
                  <span class="server-item-status {getStatusColor(server.status)}">
                    {getStatusIcon(server.status)}
                  </span>
                  <div class="server-item-info">
                    <span class="server-item-name">{server.config.name}</span>
                    <span class="server-item-capabilities">
                      {#each getCapabilityBadges(server) as badge}
                        <span class="capability-badge-small">{badge}</span>
                      {/each}
                    </span>
                  </div>
                  {#if isSelected}
                    <span class="selected-indicator">✓</span>
                  {:else}
                    <span class="unselected-indicator">○</span>
                  {/if}
                </button>
              {/each}
            </div>
          {/each}
        {/if}
      </div>

      <!-- Footer -->
      <div class="dropdown-footer">
        <button class="footer-action" onclick={handleOpenServerManager}>
          ⚙️ Manage Servers & Profiles
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  /* Main Container */
  .server-context-bar {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-3);
    background: var(--mcp-surface-secondary);
    border-bottom: 1px solid var(--mcp-border-primary);
    position: relative;
    min-height: 56px;
  }

  .server-context-bar.compact {
    min-height: 44px;
    padding: var(--mcp-space-2);
  }

  /* Server Display */
  .server-display {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    flex: 1;
  }

  .server-display.empty {
    gap: var(--mcp-space-3);
  }

  .server-label {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-secondary);
    white-space: nowrap;
  }

  .server-label.compact {
    display: none;
  }

  /* Server Selector Button */
  .server-selector {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: var(--mcp-text-sm);
    min-width: 200px;
  }

  .server-selector:hover {
    background: var(--mcp-surface-hover);
    border-color: var(--mcp-border-hover);
  }

  .server-status {
    display: flex;
    align-items: center;
  }

  .server-name {
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-primary);
    flex: 1;
    text-align: left;
  }

  .capability-badges {
    display: flex;
    gap: var(--mcp-space-1);
  }

  .capability-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    font-size: 10px;
    font-weight: var(--mcp-font-bold);
    background: var(--mcp-primary-subtle);
    color: var(--mcp-primary);
    border-radius: var(--mcp-radius-sm);
  }

  .dropdown-icon {
    color: var(--mcp-text-tertiary);
  }

  /* Server Actions */
  .server-actions {
    display: flex;
    gap: var(--mcp-space-2);
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-1);
    padding: var(--mcp-space-2);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
  }

  .action-btn:hover {
    background: var(--mcp-surface-hover);
    border-color: var(--mcp-border-hover);
    color: var(--mcp-text-primary);
  }

  .action-btn.reconnect {
    border-color: var(--mcp-warning);
    color: var(--mcp-warning);
  }

  .action-btn.reconnect:hover {
    background: var(--mcp-warning-subtle);
  }

  /* Profile Context */
  .profile-context {
    display: flex;
    align-items: center;
  }

  .profile-badge {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-1);
    padding: var(--mcp-space-1) var(--mcp-space-2);
    background: var(--mcp-success-subtle);
    border: 1px solid var(--mcp-success);
    border-radius: var(--mcp-radius-md);
    font-size: var(--mcp-text-xs);
    color: var(--mcp-success);
  }

  /* Empty State */
  .empty-text {
    color: var(--mcp-text-secondary);
    font-size: var(--mcp-text-sm);
  }

  .warning-icon {
    color: var(--mcp-warning);
  }

  .btn-select-server {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: var(--mcp-primary);
    color: white;
    border: none;
    border-radius: var(--mcp-radius-md);
    cursor: pointer;
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    transition: all 0.2s ease;
  }

  .btn-select-server:hover {
    background: var(--mcp-primary-hover);
  }

  /* Dropdown */
  .server-dropdown {
    position: absolute;
    top: calc(100% + var(--mcp-space-1));
    left: var(--mcp-space-3);
    right: var(--mcp-space-3);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    box-shadow: var(--mcp-shadow-xl);
    z-index: 1000;
    max-height: 500px;
    display: flex;
    flex-direction: column;
  }

  .dropdown-search {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-3);
    border-bottom: 1px solid var(--mcp-border-primary);
  }

  .search-input {
    flex: 1;
    padding: var(--mcp-space-2);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    background: var(--mcp-surface-secondary);
    color: var(--mcp-text-primary);
    font-size: var(--mcp-text-sm);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--mcp-primary);
  }

  .dropdown-content {
    overflow-y: auto;
    max-height: 360px;
  }

  .dropdown-empty {
    padding: var(--mcp-space-8);
    text-align: center;
    color: var(--mcp-text-secondary);
  }

  .dropdown-empty p {
    margin: var(--mcp-space-3) 0;
  }

  .btn-manage {
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: var(--mcp-primary);
    color: white;
    border: none;
    border-radius: var(--mcp-radius-md);
    cursor: pointer;
    font-size: var(--mcp-text-sm);
  }

  /* Server Groups */
  .server-group {
    padding: var(--mcp-space-2) 0;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-2) var(--mcp-space-3);
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-bold);
    color: var(--mcp-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .group-count {
    color: var(--mcp-text-tertiary);
  }

  /* Server Items */
  .server-item {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    width: 100%;
    padding: var(--mcp-space-3);
    border: none;
    background: transparent;
    cursor: pointer;
    transition: background 0.2s ease;
    text-align: left;
  }

  .server-item:hover:not(:disabled) {
    background: var(--mcp-surface-hover);
  }

  .server-item.selected {
    background: var(--mcp-primary-subtle);
    cursor: default;
  }

  .server-item-status {
    font-size: 18px;
  }

  .server-item-info {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-1);
    flex: 1;
    min-width: 0;
  }

  .server-item-name {
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .server-item-capabilities {
    display: flex;
    gap: var(--mcp-space-1);
  }

  .capability-badge-small {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    font-size: 9px;
    font-weight: var(--mcp-font-bold);
    background: var(--mcp-surface-tertiary);
    color: var(--mcp-text-secondary);
    border-radius: var(--mcp-radius-sm);
  }

  .selected-indicator {
    color: var(--mcp-primary);
    font-weight: var(--mcp-font-bold);
  }

  .unselected-indicator {
    color: var(--mcp-text-tertiary);
    font-weight: var(--mcp-font-normal);
  }

  /* Dropdown Footer */
  .dropdown-footer {
    padding: var(--mcp-space-2);
    border-top: 1px solid var(--mcp-border-primary);
  }

  .footer-action {
    width: 100%;
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-secondary);
    font-size: var(--mcp-text-sm);
    text-align: left;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .footer-action:hover {
    background: var(--mcp-surface-hover);
    border-color: var(--mcp-border-primary);
    color: var(--mcp-text-primary);
  }

  /* Status Colors */
  .status-connected { color: var(--mcp-success); }
  .status-connecting { color: var(--mcp-warning); }
  .status-error { color: var(--mcp-error); }
  .status-disconnected { color: var(--mcp-text-tertiary); }
</style>

<script module lang="ts">
  function getStatusColor(status: string) {
    switch (status?.toLowerCase()) {
      case 'connected': return 'status-connected';
      case 'connecting': return 'status-connecting';
      case 'error': return 'status-error';
      default: return 'status-disconnected';
    }
  }

  function getStatusIcon(status: string) {
    switch (status?.toLowerCase()) {
      case 'connected': return '🟢';
      case 'connecting': return '🟡';
      case 'error': return '🔴';
      default: return '⚪';
    }
  }
</script>
