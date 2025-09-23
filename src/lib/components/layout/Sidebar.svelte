<!--
  MCP Studio Sidebar Component
  Enterprise navigation sidebar with server management
-->
<script lang="ts">
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore, type View } from '$lib/stores/uiStore';
  import { 
    Monitor, 
    Zap, 
    Database, 
    FileText, 
    MessageSquare, 
    Activity, 
    FolderOpen,
    Settings,
    Plus,
    ChevronRight,
    ChevronDown
  } from 'lucide-svelte';

  // Props using Svelte 5 runes
  const { isMobile = false, isSidebarCollapsed = false } = $props();

  let servers: ServerInfo[] = $state([]);
  let currentView: View = $state('dashboard');
  let expandedSections = $state(new Set(['navigation', 'servers']));

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

  function selectServer(serverId: string) {
    serverStore.selectServer(serverId);
    uiStore.setView('tools');
  }

  function addServer() {
    uiStore.openModal('addServer');
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

    // Debug log the transport object structure
    console.log('Transport object:', JSON.stringify(transport, null, 2));

    // Check for the serde tagged format first
    if (transport.type) {
      return transport.type.toUpperCase();
    }

    // Check for individual transport variants (if serde serializes them as object keys)
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
      <span>Servers</span>
      <div class="mcp-sidebar__section-actions">
        <span class="mcp-sidebar__server-count">{servers.length}</span>
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
        {#if servers.length === 0}
          <div class="mcp-sidebar__empty-state">
            <Database size={32} class="mcp-sidebar__empty-icon" />
            <p class="mcp-sidebar__empty-text">No servers connected</p>
            <button
              class="mcp-sidebar__empty-action"
              onclick={addServer}
            >
              Add your first server
            </button>
          </div>
        {:else}
          <div class="mcp-sidebar__server-list">
            {#each servers as server (server.id)}
              <button
                class="mcp-sidebar__server-item"
                onclick={() => {
                  console.log('Server object:', JSON.stringify(server, null, 2));
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
                      {getTransportType(server.config.transport)}
                    </span>
                    {#if server.metrics}
                      <span class="mcp-sidebar__server-messages">
                        {server.metrics.messages_sent + server.metrics.messages_received} msgs
                      </span>
                    {/if}
                  </div>
                </div>
              </button>
            {/each}
          </div>
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
    display: block;
    width: 100%;
    padding: var(--mcp-space-3);
    margin-bottom: var(--mcp-space-2);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
    text-align: left;
  }

  .mcp-sidebar__server-item:hover {
    background: var(--mcp-surface-tertiary);
    border-color: var(--mcp-border-secondary);
    box-shadow: var(--mcp-shadow-sm);
  }

  .mcp-sidebar__server-item:last-child {
    margin-bottom: 0;
  }

  .mcp-sidebar__server-info {
    width: 100%;
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
    background: var(--mcp-primary-900);
    color: var(--mcp-primary-300);
  }

  [data-theme="dark"] .mcp-sidebar__add-button:hover {
    background: var(--mcp-primary-800);
  }

  [data-theme="dark"] .nav-item-active {
    background: var(--mcp-primary-900);
    color: var(--mcp-primary-200);
  }
</style>