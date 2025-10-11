<!--
  MCP Studio Sidebar Component
  Enterprise navigation sidebar with server management
-->
<script lang="ts">
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore, type View } from '$lib/stores/uiStore';
  import { profileStore } from '$lib/stores/profileStore';
  import { getServerCapabilitySummary, serverSupportsCapability, type McpCapability } from '$lib/utils/serverCapabilities';
  import {
    Monitor,
    Server,
    Zap,
    Database,
    FileText,
    MessageSquare,
    Activity,
    FolderOpen,
    Layers3,
    Settings,
    Plus,
    ChevronRight,
    ChevronDown,
    Play,
    Square,
    Network
  } from 'lucide-svelte';

  // Props using Svelte 5 runes
  const { isMobile = false, isSidebarCollapsed = false } = $props();

  // Reactive store access using Svelte 5 patterns
  const serverState = $derived($serverStore);
  const uiState = $derived($uiStore);
  const profileState = $derived($profileStore);

  // ✅ FIXED: Convert Map to array for UI compatibility
  const servers = $derived(
    serverState.servers instanceof Map
      ? Array.from(serverState.servers.values())
      : []
  );
  const selectedServerId = $derived(serverState.selectedServerId);
  const currentView = $derived(uiState.currentView);
  const activeProfile = $derived(profileState.activeProfile);

  let expandedSections = $state(new Set(['navigation', 'servers']));

  const navigationItems = [
    { id: 'dashboard', label: 'Dashboard', icon: Monitor },
    { id: 'servers', label: 'Server Manager', icon: Server },
    { id: 'tools', label: 'Tools', icon: Zap },
    { id: 'resources', label: 'Resources', icon: Database },
    { id: 'prompts', label: 'Prompts', icon: FileText },
    { id: 'sampling', label: 'Sampling', icon: MessageSquare },
    { id: 'elicitation', label: 'Elicitation', icon: Activity },
    { id: 'protocol', label: 'Protocol', icon: Network },
    { id: 'collections', label: 'Collections', icon: Layers3 },
  ];

  function navigateTo(view: View) {
    uiStore.setView(view);
  }

  // Map views to their required capabilities
  const viewCapabilityMap: Record<View, McpCapability | null> = {
    'dashboard': null,        // Dashboard supports all servers
    'servers': null,          // Server Manager supports all servers (manages profiles)
    'tools': 'tools',
    'resources': 'resources',
    'prompts': 'prompts',
    'sampling': 'sampling',
    'elicitation': 'elicitation',
    'protocol': null,         // Protocol inspector supports all servers
    'collections': null,      // Collections supports all servers
    'settings': null          // Settings doesn't require MCP capability
  };

  function selectServer(serverId: string) {
    const server = servers.find(s => s.id === serverId);
    if (!server) return;

    const requiredCapability = viewCapabilityMap[currentView];

    // If current view doesn't require a specific capability (like dashboard), always allow selection
    if (!requiredCapability) {
      serverStore.selectServer(serverId);
      return;
    }

    // Check if server supports the current view's capability
    if (serverSupportsCapability(server, requiredCapability)) {
      serverStore.selectServer(serverId);
    } else {
      // Server doesn't support current view - find a compatible view and switch
      const compatibleViews: View[] = [];

      if (serverSupportsCapability(server, 'tools')) compatibleViews.push('tools');
      if (serverSupportsCapability(server, 'resources')) compatibleViews.push('resources');
      if (serverSupportsCapability(server, 'prompts')) compatibleViews.push('prompts');
      if (serverSupportsCapability(server, 'sampling')) compatibleViews.push('sampling');
      if (serverSupportsCapability(server, 'elicitation')) compatibleViews.push('elicitation');

      if (compatibleViews.length > 0) {
        // Switch to first compatible view and select server
        uiStore.setView(compatibleViews[0]);
        serverStore.selectServer(serverId);
      } else {
        // No compatible views - just go to dashboard
        uiStore.setView('dashboard');
        serverStore.selectServer(serverId);
      }
    }
  }

  function addServer() {
    uiStore.openModal('addServer');
  }

  async function toggleConnection(server: ServerInfo, event: Event) {
    event.stopPropagation();

    try {
      if (server.status?.toLowerCase() === 'connected') {
        await serverStore.disconnectServer(server.id);
        uiStore.showSuccess(`Disconnected from ${server.config.name}`);
      } else {
        await serverStore.connectServer(server.config);
        uiStore.showSuccess(`Connected to ${server.config.name}`);
      }
    } catch (error) {
      uiStore.showError(`Failed to ${server.status?.toLowerCase() === 'connected' ? 'disconnect from' : 'connect to'} ${server.config.name}: ${error}`);
    }
  }

  function openServerConfig(server: ServerInfo, event: Event) {
    event.stopPropagation();
    serverStore.selectServer(server.id);
    uiStore.openModal('serverConfig');
  }

  function getStatusColor(status: string) {
    switch (status?.toLowerCase()) {
      case 'connected': return 'status-connected';
      case 'connecting': return 'status-connecting';
      case 'error': return 'status-error';
      default: return 'status-disconnected';
    }
  }

  function getTransportType(transport: any): string {
    if (!transport) return 'UNKNOWN';

    // Rust enum with serde(tag = "type", rename_all = "camelCase") serializes as:
    // { "type": "stdio", "command": "...", ... }
    if (transport.type) {
      return transport.type.toUpperCase();
    }

    // Fallback checks for other possible formats
    if (transport.stdio) return 'STDIO';
    if (transport.http) return 'HTTP';
    if (transport.websocket) return 'WEBSOCKET';
    if (transport.tcp) return 'TCP';
    if (transport.unix) return 'UNIX';

    return 'UNKNOWN';
  }

  function toggleSection(section: string) {
    if (expandedSections.has(section)) {
      expandedSections.delete(section);
    } else {
      expandedSections.add(section);
    }
    expandedSections = new Set(expandedSections);
  }


  // Profile-aware server display logic
  const shouldShowProfileView = $derived(activeProfile?.profile != null);

  const displayServers = $derived(() => {
    if (shouldShowProfileView && activeProfile) {
      // Show only active profile servers
      const profileServerIds = new Set(activeProfile.servers.map(ps => ps.server_id));
      return servers.filter(s => profileServerIds.has(s.id));
    } else {
      // Show all servers
      return servers || [];
    }
  });

  // Group servers when no profile is active
  const groupedServers = $derived(() => {
    if (shouldShowProfileView) return { grouped: [], ungrouped: [] };

    const grouped: ServerInfo[] = [];
    const ungrouped: ServerInfo[] = [];

    servers.forEach(server => {
      // Check if server is in any profile
      const isInProfile = profileState.profiles.some(profile =>
        profile.server_count > 0 // Simplified check - would need to query profile servers
      );
      // For now, show all as ungrouped since we don't have profile membership info here
      ungrouped.push(server);
    });

    return { grouped, ungrouped };
  });

  // Connection count for active profile
  const profileConnectionStatus = $derived(() => {
    if (!shouldShowProfileView || !activeProfile) return null;

    const profileServers = displayServers();
    const connectedCount = profileServers.filter(s => s.status === 'connected').length;

    return {
      connected: connectedCount,
      total: profileServers.length
    };
  });
</script>

<div class="mcp-sidebar-content">
  <!-- Navigation Section -->
  <nav class="mcp-sidebar__section" aria-label="Main navigation">
    <button 
      class="mcp-sidebar__section-header"
      onclick={() => toggleSection('navigation')}
      aria-expanded={expandedSections.has('navigation')}
    >
      {#if expandedSections.has('navigation')}
        <ChevronDown size={16} />
      {:else}
        <ChevronRight size={16} />
      {/if}
      <span>Navigation</span>
    </button>
    
    {#if expandedSections.has('navigation')}
      <div class="mcp-sidebar__nav-items">
        {#each navigationItems as item}
          <button
            class="nav-item"
            class:nav-item-active={currentView === item.id}
            class:nav-item-inactive={currentView !== item.id}
            onclick={() => navigateTo(item.id as View)}
            aria-current={currentView === item.id ? 'page' : undefined}
          >
            {#if item.icon}
              {@const IconComponent = item.icon}
              <IconComponent size={16} class="nav-item__icon" />
            {/if}
            <span class="nav-item__label">{item.label}</span>
          </button>
        {/each}
      </div>
    {/if}
  </nav>

  <!-- Servers Section -->
  <div class="mcp-sidebar__section">
    <button
      class="mcp-sidebar__section-header"
      onclick={() => toggleSection('servers')}
      aria-expanded={expandedSections.has('servers')}
    >
      {#if expandedSections.has('servers')}
        <ChevronDown size={16} />
      {:else}
        <ChevronRight size={16} />
      {/if}
      <span>{shouldShowProfileView && activeProfile?.profile ? `⚡ ${activeProfile.profile.name}` : 'Servers'}</span>
      <div class="mcp-sidebar__section-actions">
        {#if shouldShowProfileView && profileConnectionStatus()}
          {@const status = profileConnectionStatus()}
          {#if status}
            <span class="mcp-sidebar__server-count text-green-600 dark:text-green-400">
              {status.connected}/{status.total}
            </span>
          {/if}
        {:else}
          <span class="mcp-sidebar__server-count">{servers.length}</span>
        {/if}
        <div
          class="mcp-sidebar__add-button"
          onclick={(e) => { e.stopPropagation(); addServer(); }}
          title="Add Server"
          aria-label="Add new server"
          role="button"
          tabindex="0"
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); addServer(); } }}
        >
          <Plus size={14} />
        </div>
      </div>
    </button>

    {#if expandedSections.has('servers')}
      <div class="mcp-sidebar__servers">
        {#if displayServers().length === 0}
          <div class="mcp-sidebar__empty-state">
            <Database size={32} class="mcp-sidebar__empty-icon" />
            <p class="mcp-sidebar__empty-text">No servers configured</p>
            <button
              class="mcp-sidebar__empty-action"
              onclick={addServer}
            >
              Add your first server
            </button>
          </div>
        {:else}
          <div class="mcp-sidebar__server-list">
            {#each displayServers() as server (server.id)}
              <div class="mcp-sidebar__server-item {selectedServerId === server.id ? 'mcp-sidebar__server-item--selected border-mcp-primary-300 bg-mcp-primary-50 ring-2 ring-mcp-primary-200 dark:border-mcp-primary-600 dark:bg-mcp-primary-900/30 dark:ring-mcp-primary-700' : ''}">
                <button
                  class="mcp-sidebar__server-main"
                  onclick={() => {
                    selectServer(server.id);
                  }}
                  title={server.config.description || server.config.name}
                >
                  <div class="mcp-sidebar__server-info">
                    <div class="mcp-sidebar__server-header">
                      <div class="mcp-sidebar__server-status">
                        <div class="mcp-sidebar__status-dot {getStatusColor(server.status)}"></div>
                        <h3 class="mcp-sidebar__server-name">
                          {server.config.name || 'Unnamed Server'}
                        </h3>
                      </div>
                    </div>

                    {#if server.config.description}
                      <p class="mcp-sidebar__server-description">
                        {server.config.description}
                      </p>
                    {/if}

                    <div class="mcp-sidebar__server-meta">
                      <span class="mcp-sidebar__server-transport">
                        {getTransportType(server.config.transport_config)}
                      </span>
                      {#if server.metrics}
                        <span class="mcp-sidebar__server-messages">
                          {server.metrics.requests_sent + server.metrics.responses_received} msgs
                        </span>
                      {/if}
                    </div>

                    {#if server.capabilities}
                      <div class="mcp-sidebar__server-capabilities">
                        {#if server.capabilities.tools}
                          <span class="capability-badge capability-badge--tools" title="Tools">T</span>
                        {/if}
                        {#if server.capabilities.resources}
                          <span class="capability-badge capability-badge--resources" title="Resources">R</span>
                        {/if}
                        {#if server.capabilities.prompts}
                          <span class="capability-badge capability-badge--prompts" title="Prompts">P</span>
                        {/if}
                        {#if server.capabilities.sampling}
                          <span class="capability-badge capability-badge--sampling" title="Sampling">S</span>
                        {/if}
                        {#if server.capabilities.elicitation}
                          <span class="capability-badge capability-badge--elicitation" title="Elicitation">E</span>
                        {/if}
                      </div>
                    {/if}
                  </div>
                </button>

                <div class="mcp-sidebar__server-actions">
                  <button
                    class="mcp-sidebar__action-button {server.status?.toLowerCase() === 'connected' ? 'mcp-sidebar__action-button--disconnect' : 'mcp-sidebar__action-button--connect'}"
                    onclick={(e) => toggleConnection(server, e)}
                    title={server.status?.toLowerCase() === 'connected' ? 'Disconnect' : 'Connect'}
                  >
                    {#if server.status?.toLowerCase() === 'connected'}
                      <Square size={14} />
                    {:else}
                      <Play size={14} />
                    {/if}
                  </button>

                  <button
                    class="mcp-sidebar__action-button mcp-sidebar__action-button--settings"
                    onclick={(e) => openServerConfig(server, e)}
                    title="Settings"
                  >
                    <Settings size={14} />
                  </button>
                </div>
              </div>
            {/each}
          </div>

          <!-- Profile Quick Actions (when profile active) -->
          {#if shouldShowProfileView && activeProfile}
            <div class="mcp-sidebar__profile-actions">
              <button
                class="mcp-sidebar__profile-action"
                onclick={async () => await profileStore.deactivateProfile()}
                title="Deactivate profile"
              >
                <Square size={14} />
                <span>Deactivate Profile</span>
              </button>
              <button
                class="mcp-sidebar__profile-action"
                onclick={() => uiStore.setView('servers')}
                title="Manage profiles"
              >
                <FolderOpen size={14} />
                <span>Manage Profiles</span>
              </button>
            </div>
          {/if}
        {/if}
      </div>
    {/if}
  </div>

  <!-- Settings Link -->
  <div class="mcp-sidebar__footer">
    <button
      class="nav-item"
      class:nav-item-active={currentView === 'settings'}
      class:nav-item-inactive={currentView !== 'settings'}
      onclick={() => navigateTo('settings')}
    >
      <Settings size={16} class="nav-item__icon" />
      <span class="nav-item__label">Settings</span>
    </button>
  </div>
</div>

<style>
  .mcp-sidebar-content {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--mcp-surface-secondary);
    overflow: hidden;
  }

  /* Section Styles */
  .mcp-sidebar__section {
    border-bottom: 1px solid var(--mcp-border-primary);
  }

  .mcp-sidebar__section:last-child {
    border-bottom: none;
  }

  .mcp-sidebar__section-header {
    display: flex;
    align-items: center;
    width: 100%;
    padding: var(--mcp-space-3) var(--mcp-space-4);
    background: transparent;
    border: none;
    color: var(--mcp-text-secondary);
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-semibold);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: color var(--mcp-transition-fast);
    gap: var(--mcp-space-2);
  }

  .mcp-sidebar__section-header:hover {
    color: var(--mcp-text-primary);
  }

  .mcp-sidebar__section-actions {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    margin-left: auto;
  }

  .mcp-sidebar__server-count {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    background: var(--mcp-surface-tertiary);
    padding: 2px 6px;
    border-radius: var(--mcp-radius-full);
    min-width: 20px;
    text-align: center;
  }

  .mcp-sidebar__add-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: none;
    background: var(--mcp-primary-100);
    color: var(--mcp-primary-700);
    border-radius: var(--mcp-radius-sm);
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
  }

  .mcp-sidebar__add-button:hover {
    background: var(--mcp-primary-200);
  }

  /* Navigation Items */
  .mcp-sidebar__nav-items {
    padding: var(--mcp-space-2) var(--mcp-space-2) var(--mcp-space-3);
  }

  .nav-item {
    display: flex;
    align-items: center;
    width: 100%;
    padding: var(--mcp-space-2-5) var(--mcp-space-3);
    margin-bottom: var(--mcp-space-1);
    background: transparent;
    border: none;
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-secondary);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    text-align: left;
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
    gap: var(--mcp-space-3);
  }

  .nav-item:hover {
    background: var(--mcp-surface-tertiary);
    color: var(--mcp-text-primary);
  }

  .nav-item-active {
    background: var(--mcp-primary-100);
    color: var(--mcp-primary-700);
    font-weight: var(--mcp-font-semibold);
  }

  .nav-item__icon {
    flex-shrink: 0;
  }

  .nav-item__label {
    flex: 1;
    min-width: 0;
  }

  /* Server List */
  .mcp-sidebar__servers {
    max-height: 400px;
    overflow-y: auto;
  }

  .mcp-sidebar__server-list {
    padding: var(--mcp-space-2);
  }

  .mcp-sidebar__server-item {
    display: flex;
    width: 100%;
    margin-bottom: var(--mcp-space-2);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    transition: all var(--mcp-transition-fast);
    overflow: hidden;
  }

  .mcp-sidebar__server-item:hover {
    background: var(--mcp-surface-tertiary);
    border-color: var(--mcp-border-secondary);
    box-shadow: var(--mcp-shadow-sm);
  }

  .mcp-sidebar__server-item:last-child {
    margin-bottom: 0;
  }


  .mcp-sidebar__server-main {
    flex: 1;
    background: transparent;
    border: none;
    padding: var(--mcp-space-3);
    cursor: pointer;
    text-align: left;
    min-width: 0;
  }

  .mcp-sidebar__server-actions {
    display: flex;
    flex-direction: column;
    background: var(--mcp-surface-secondary);
    border-left: 1px solid var(--mcp-border-primary);
    width: 32px;
    flex-shrink: 0;
  }

  .mcp-sidebar__action-button {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    padding: var(--mcp-space-2);
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
  }

  .mcp-sidebar__action-button:hover {
    background: var(--mcp-surface-tertiary);
  }

  .mcp-sidebar__action-button--connect {
    color: var(--mcp-success-600);
  }

  .mcp-sidebar__action-button--connect:hover {
    background: var(--mcp-success-50);
    color: var(--mcp-success-700);
  }

  .mcp-sidebar__action-button--disconnect {
    color: var(--mcp-error-600);
  }

  .mcp-sidebar__action-button--disconnect:hover {
    background: var(--mcp-error-50);
    color: var(--mcp-error-700);
  }

  .mcp-sidebar__action-button--settings {
    color: var(--mcp-text-tertiary);
    border-top: 1px solid var(--mcp-border-primary);
  }

  .mcp-sidebar__action-button--settings:hover {
    background: var(--mcp-surface-tertiary);
    color: var(--mcp-text-secondary);
  }

  .mcp-sidebar__server-info {
    width: 100%;
    min-width: 0; /* Allow flex shrinking */
  }

  .mcp-sidebar__server-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--mcp-space-1-5);
  }

  .mcp-sidebar__server-status {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    min-width: 0;
    flex: 1;
  }

  .mcp-sidebar__status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .mcp-sidebar__status-dot.status-connected {
    background: var(--mcp-success-500);
  }

  .mcp-sidebar__status-dot.status-connecting {
    background: var(--mcp-warning-500);
  }

  .mcp-sidebar__status-dot.status-error {
    background: var(--mcp-error-500);
  }

  .mcp-sidebar__status-dot.status-disconnected {
    background: var(--mcp-gray-400);
  }

  .mcp-sidebar__server-name {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    margin: 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mcp-sidebar__server-description {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    margin: 0 0 var(--mcp-space-1-5) 0;
    line-height: var(--mcp-leading-normal);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mcp-sidebar__server-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    gap: var(--mcp-space-2);
  }

  .mcp-sidebar__server-transport {
    font-weight: var(--mcp-font-medium);
    text-transform: uppercase;
  }

  /* Empty State */
  .mcp-sidebar__empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--mcp-space-8) var(--mcp-space-4);
    text-align: center;
  }

  .mcp-sidebar__empty-icon {
    color: var(--mcp-text-tertiary);
    margin-bottom: var(--mcp-space-3);
  }

  .mcp-sidebar__empty-text {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
    margin: 0 0 var(--mcp-space-3) 0;
  }

  .mcp-sidebar__empty-action {
    background: transparent;
    border: none;
    color: var(--mcp-primary-600);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    cursor: pointer;
    text-decoration: underline;
    transition: color var(--mcp-transition-fast);
  }

  .mcp-sidebar__empty-action:hover {
    color: var(--mcp-primary-700);
  }

  /* Footer */
  /* Profile Actions */
  .mcp-sidebar__profile-actions {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-3);
    border-top: 1px solid var(--mcp-border-secondary);
    margin-top: var(--mcp-space-2);
  }

  .mcp-sidebar__profile-action {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-primary);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .mcp-sidebar__profile-action:hover {
    background: var(--mcp-surface-hover);
    border-color: var(--mcp-border-hover);
  }

  .mcp-sidebar__footer {
    margin-top: auto;
    padding: var(--mcp-space-4);
    border-top: 1px solid var(--mcp-border-primary);
  }

  /* Scrollbar */
  .mcp-sidebar__servers::-webkit-scrollbar {
    width: 4px;
  }

  .mcp-sidebar__servers::-webkit-scrollbar-track {
    background: transparent;
  }

  .mcp-sidebar__servers::-webkit-scrollbar-thumb {
    background: var(--mcp-border-primary);
    border-radius: 2px;
  }

  .mcp-sidebar__servers::-webkit-scrollbar-thumb:hover {
    background: var(--mcp-border-secondary);
  }

  /* Dark theme adjustments */
  [data-theme="dark"] .mcp-sidebar__add-button {
    background: var(--mcp-surface-tertiary);
    color: var(--mcp-primary-300);
  }

  [data-theme="dark"] .mcp-sidebar__add-button:hover {
    background: var(--mcp-surface-elevated);
  }

  [data-theme="dark"] .nav-item-active {
    background: var(--mcp-primary-800);
    color: var(--mcp-primary-100);
    font-weight: var(--mcp-font-semibold);
    border-left: 3px solid var(--mcp-primary-400);
  }

  [data-theme="dark"] .mcp-sidebar__server-item:hover {
    background: rgba(12, 74, 110, 0.2); /* mcp-primary-900/20 */
    border-color: var(--mcp-primary-600);
  }

  /* Capability badges */
  .mcp-sidebar__server-capabilities {
    display: flex;
    gap: 4px;
    margin-top: var(--mcp-space-2);
  }

  .capability-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: var(--mcp-radius-sm);
    font-size: 10px;
    font-weight: var(--mcp-font-bold);
    flex-shrink: 0;
  }

  .capability-badge--tools {
    background: var(--mcp-primary-100);
    color: var(--mcp-primary-700);
  }

  .capability-badge--resources {
    background: var(--mcp-success-100);
    color: var(--mcp-success-700);
  }

  .capability-badge--prompts {
    background: rgb(243 232 255); /* purple-100 */
    color: rgb(126 34 206); /* purple-700 */
  }

  .capability-badge--sampling {
    background: rgb(255 237 213); /* orange-100 */
    color: rgb(194 65 12); /* orange-700 */
  }

  .capability-badge--elicitation {
    background: rgb(252 231 243); /* pink-100 */
    color: rgb(190 24 93); /* pink-700 */
  }

  [data-theme="dark"] .capability-badge--tools {
    background: rgba(59, 130, 246, 0.2);
    color: rgb(147, 197, 253);
  }

  [data-theme="dark"] .capability-badge--resources {
    background: rgba(34, 197, 94, 0.2);
    color: rgb(134, 239, 172);
  }

  [data-theme="dark"] .capability-badge--prompts {
    background: rgba(168, 85, 247, 0.2);
    color: rgb(216, 180, 254);
  }

  [data-theme="dark"] .capability-badge--sampling {
    background: rgba(249, 115, 22, 0.2);
    color: rgb(253, 186, 116);
  }

  [data-theme="dark"] .capability-badge--elicitation {
    background: rgba(236, 72, 153, 0.2);
    color: rgb(249, 168, 212);
  }

</style>