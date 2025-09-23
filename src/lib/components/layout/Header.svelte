<!--
  MCP Studio Header Component
  Enterprise application header with navigation and controls
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { themeStore } from '$lib/stores/themeStore';
  import { uiStore } from '$lib/stores/uiStore';
  import { serverStore, getServerStatus } from '$lib/stores/serverStore';
  import Button from '../ui/Button.svelte';
  import { 
    Menu, 
    X, 
    Sun, 
    Moon, 
    Monitor, 
    Settings, 
    HelpCircle,
    Search,
    Bell,
    User
  } from 'lucide-svelte';

  // Props using Svelte 5 runes
  const { isMobile = false, isSidebarCollapsed = false, isMobileMenuOpen = false } = $props();

  const dispatch = createEventDispatcher<{
    'toggle-sidebar': void;
  }>();

  // Store subscriptions
  const theme = $derived($themeStore);
  const ui = $derived($uiStore);
  const servers = $derived($serverStore);

  // Connection status
  const connectedServers = $derived(servers.servers.filter(s => getServerStatus(s) === 'connected').length);
  const totalServers = $derived(servers.servers.length);
  const connectionStatus = $derived(connectedServers === 0 ? 'disconnected' :
                      connectedServers === totalServers ? 'connected' : 'partial');

  // Theme toggle
  function toggleTheme() {
    themeStore.toggleTheme();
  }

  function openSettings() {
    uiStore.setView('settings');
  }

  function openHelp() {
    // Open help/documentation
    window.open('https://github.com/anthropics/mcp-studio', '_blank');
  }

  // Search functionality
  let searchQuery = $state('');
  function handleSearch() {
    if (searchQuery.trim()) {
      // Implement global search
      console.log('Search:', searchQuery);
    }
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      handleSearch();
    }
    if (event.key === 'Escape') {
      searchQuery = '';
    }
  }
</script>

<header class="mcp-header">
  <div class="mcp-header__content">
    <!-- Left Section: Logo & Menu Toggle -->
    <div class="mcp-header__left">
      <button
        class="mcp-header__menu-toggle"
        class:mcp-header__menu-toggle--active={isMobileMenuOpen}
        aria-label={isMobileMenuOpen ? 'Close menu' : 'Open menu'}
        onclick={() => dispatch('toggle-sidebar')}
      >
        {#if isMobileMenuOpen}
          <X size={20} />
        {:else}
          <Menu size={20} />
        {/if}
      </button>

      <!-- Logo & Title -->
      <div class="mcp-header__logo">
        <div class="mcp-header__logo-icon">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
            <rect x="3" y="3" width="18" height="18" rx="3" stroke="currentColor" stroke-width="2"/>
            <path d="M8 12h8M12 8v8" stroke="currentColor" stroke-width="2"/>
          </svg>
        </div>
        <div class="mcp-header__title">
          <h1>MCP Studio</h1>
          {#if !isMobile}
            <span class="mcp-header__subtitle">Model Context Protocol</span>
          {/if}
        </div>
      </div>
    </div>

    <!-- Center Section: Search (Desktop only) -->
    {#if !isMobile}
      <div class="mcp-header__center">
        <div class="mcp-header__search">
          <Search size={16} class="mcp-header__search-icon" />
          <input
            type="text"
            placeholder="Search servers, tools, resources..."
            bind:value={searchQuery}
            onkeydown={handleSearchKeydown}
            class="mcp-header__search-input"
          />
        </div>
      </div>
    {/if}

    <!-- Right Section: Status & Controls -->
    <div class="mcp-header__right">
      <!-- Connection Status -->
      <div class="mcp-header__status">
        <div 
          class="mcp-header__status-indicator"
          class:mcp-header__status-indicator--connected={connectionStatus === 'connected'}
          class:mcp-header__status-indicator--partial={connectionStatus === 'partial'}
          class:mcp-header__status-indicator--disconnected={connectionStatus === 'disconnected'}
          title={`${connectedServers}/${totalServers} servers connected`}
        >
          <div class="mcp-header__status-dot"></div>
          {#if !isMobile}
            <span class="mcp-header__status-text">
              {connectedServers}/{totalServers}
            </span>
          {/if}
        </div>
      </div>

      <!-- Theme Toggle -->
      <Button
        variant="ghost"
        size="sm"
        leftIcon={theme.theme === 'dark' ? Sun : theme.theme === 'light' ? Moon : Monitor}
        onclick={toggleTheme}
        title="Toggle theme"
      >
        {#if !isMobile}
          {theme.theme === 'system' ? 'Auto' : theme.resolvedTheme === 'dark' ? 'Dark' : 'Light'}
        {/if}
      </Button>

      <!-- Notifications (placeholder) -->
      <button
        class="mcp-header__icon-button"
        title="Notifications"
        aria-label="Notifications"
      >
        <Bell size={18} />
      </button>

      <!-- Settings -->
      <Button
        variant="ghost"
        size="sm"
        leftIcon={Settings}
        onclick={openSettings}
        title="Settings"
      >
        {#if !isMobile}Settings{/if}
      </Button>

      <!-- Help -->
      <button
        class="mcp-header__icon-button"
        title="Help & Documentation"
        aria-label="Help"
        onclick={openHelp}
      >
        <HelpCircle size={18} />
      </button>
    </div>
  </div>
</header>

<style>
  .mcp-header {
    grid-area: header;
    height: var(--header-height);
    background: var(--mcp-surface-primary);
    border-bottom: 1px solid var(--mcp-border-primary);
    z-index: 30;
    display: flex;
    align-items: center;
  }

  .mcp-header__content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    height: 100%;
    padding: 0 var(--mcp-space-4);
    gap: var(--mcp-space-4);
  }

  /* Left Section */
  .mcp-header__left {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    min-width: 0;
  }

  .mcp-header__menu-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border: none;
    background: transparent;
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-secondary);
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
  }

  .mcp-header__menu-toggle:hover {
    background: var(--mcp-surface-secondary);
    color: var(--mcp-text-primary);
  }

  .mcp-header__menu-toggle--active {
    background: var(--mcp-primary-100);
    color: var(--mcp-primary-700);
  }

  .mcp-header__logo {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
  }

  .mcp-header__logo-icon {
    color: var(--mcp-primary-600);
    display: flex;
    align-items: center;
  }

  .mcp-header__title h1 {
    font-size: var(--mcp-text-lg);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    margin: 0;
    line-height: 1.2;
  }

  .mcp-header__subtitle {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    font-weight: var(--mcp-font-medium);
  }

  /* Center Section - Search */
  .mcp-header__center {
    flex: 1;
    max-width: 400px;
    margin: 0 auto;
  }

  .mcp-header__search {
    position: relative;
    display: flex;
    align-items: center;
  }

  .mcp-header__search-icon {
    position: absolute;
    left: var(--mcp-space-3);
    color: var(--mcp-text-tertiary);
    z-index: 1;
  }

  .mcp-header__search-input {
    width: 100%;
    height: 36px;
    padding: var(--mcp-space-2) var(--mcp-space-3) var(--mcp-space-2) 2.5rem;
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    background: var(--mcp-surface-secondary);
    color: var(--mcp-text-primary);
    font-size: var(--mcp-text-sm);
    transition: all var(--mcp-transition-fast);
  }

  .mcp-header__search-input:focus {
    outline: none;
    border-color: var(--mcp-primary-500);
    box-shadow: 0 0 0 2px var(--mcp-primary-100);
    background: var(--mcp-surface-primary);
  }

  .mcp-header__search-input::placeholder {
    color: var(--mcp-text-tertiary);
  }

  /* Right Section */
  .mcp-header__right {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
  }

  .mcp-header__status {
    margin-right: var(--mcp-space-2);
  }

  .mcp-header__status-indicator {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-1-5);
    padding: var(--mcp-space-1-5) var(--mcp-space-3);
    border-radius: var(--mcp-radius-full);
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-medium);
    cursor: pointer;
    transition: background-color var(--mcp-transition-fast);
  }

  .mcp-header__status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    transition: background-color var(--mcp-transition-fast);
  }

  .mcp-header__status-indicator--connected {
    background: var(--mcp-success-100);
    color: var(--mcp-success-700);
  }

  .mcp-header__status-indicator--connected .mcp-header__status-dot {
    background: var(--mcp-success-500);
  }

  .mcp-header__status-indicator--partial {
    background: var(--mcp-warning-100);
    color: var(--mcp-warning-700);
  }

  .mcp-header__status-indicator--partial .mcp-header__status-dot {
    background: var(--mcp-warning-500);
  }

  .mcp-header__status-indicator--disconnected {
    background: var(--mcp-gray-100);
    color: var(--mcp-gray-700);
  }

  .mcp-header__status-indicator--disconnected .mcp-header__status-dot {
    background: var(--mcp-gray-500);
  }

  .mcp-header__icon-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: none;
    background: transparent;
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-secondary);
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
  }

  .mcp-header__icon-button:hover {
    background: var(--mcp-surface-secondary);
    color: var(--mcp-text-primary);
  }

  /* Dark theme adjustments */
  [data-theme="dark"] .mcp-header__status-indicator--connected {
    background: var(--mcp-success-900);
    color: var(--mcp-success-300);
  }

  [data-theme="dark"] .mcp-header__status-indicator--partial {
    background: var(--mcp-warning-900);
    color: var(--mcp-warning-300);
  }

  [data-theme="dark"] .mcp-header__status-indicator--disconnected {
    background: var(--mcp-gray-800);
    color: var(--mcp-gray-300);
  }

  /* Mobile adjustments */
  @media (max-width: 767px) {
    .mcp-header__content {
      padding: 0 var(--mcp-space-3);
      gap: var(--mcp-space-2);
    }
    
    .mcp-header__title h1 {
      font-size: var(--mcp-text-base);
    }
    
    .mcp-header__right {
      gap: var(--mcp-space-1);
    }
  }

  /* High contrast mode */
  @media (prefers-contrast: high) {
    .mcp-header {
      border-bottom: 2px solid var(--mcp-border-primary);
    }
  }
</style>