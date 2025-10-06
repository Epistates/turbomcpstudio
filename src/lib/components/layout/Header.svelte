<!--
  MCP Studio Header Component
  Enterprise application header with navigation and controls
-->
<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
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
    User,
    Zap,
    Database,
    FileText,
    Layers3
  } from 'lucide-svelte';

  // Props using Svelte 5 runes
  const { isMobile = false, isSidebarCollapsed = false, isMobileMenuOpen = false } = $props();

  const dispatch = createEventDispatcher<{
    'toggle-sidebar': void;
  }>();

  // Store subscriptions
  const theme = $themeStore;
  const ui = $uiStore;
  const servers = $serverStore;

  // Connection status
  const connectedServers = $derived(servers.servers.filter((s: any) => getServerStatus(s) === 'connected').length);
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

  // Search functionality with global store
  import { searchStore, categorizedResults, type SearchResult } from '$lib/stores/searchStore';

  let searchState = $derived($searchStore);
  let categories = $derived($categorizedResults);

  // Handle keyboard shortcuts (Cmd+K / Ctrl+K)
  onMount(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        searchStore.openGlobalSearch();
      }
    };
    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });

  async function handleSearchInput(value: string) {
    await searchStore.setGlobalQuery(value);
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      searchStore.selectNext();
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      searchStore.selectPrevious();
    } else if (event.key === 'Enter') {
      event.preventDefault();
      const selected = searchStore.getSelectedResult();
      if (selected) {
        navigateToResult(selected);
      }
    } else if (event.key === 'Escape') {
      searchStore.closeGlobalSearch();
    }
  }

  function navigateToResult(result: SearchResult) {
    // Handle navigation suggestions (smart category navigation)
    if (result.metadata?.isNavSuggestion) {
      const targetView = result.metadata.navigateTo || result.type + 's';

      // Map type to view name
      const viewMap: Record<string, any> = {
        'server': 'dashboard',
        'tool': 'tools',
        'resource': 'resources',
        'prompt': 'prompts',
        'collection': 'collections',
        'protocol': 'protocol'
      };

      const view = result.metadata.navigateTo || viewMap[result.type] || result.type + 's';
      uiStore.setView(view as any);
      searchStore.closeGlobalSearch();
      return;
    }

    // Navigate to the appropriate view based on result type
    switch (result.type) {
      case 'server':
        serverStore.selectServer(result.id);
        uiStore.setView('dashboard');
        break;
      case 'tool':
        if (result.metadata?.serverId) {
          serverStore.selectServer(result.metadata.serverId);
        }
        uiStore.setView('tools');
        break;
      case 'resource':
        if (result.metadata?.serverId) {
          serverStore.selectServer(result.metadata.serverId);
        }
        uiStore.setView('resources');
        break;
      case 'prompt':
        if (result.metadata?.serverId) {
          serverStore.selectServer(result.metadata.serverId);
        }
        uiStore.setView('prompts');
        break;
      case 'collection':
        uiStore.setView('collections');
        break;
    }
    searchStore.closeGlobalSearch();
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
        <div class="relative w-full max-w-md">
          <div class="absolute inset-y-0 left-0 flex items-center pl-3 pointer-events-none">
            <Search size={16} class="text-gray-400 dark:text-gray-500" />
          </div>
          <input
            type="text"
            placeholder="Search servers, tools, resources... (⌘K)"
            value={searchState.globalQuery}
            oninput={(e) => handleSearchInput(e.currentTarget.value)}
            onkeydown={handleSearchKeydown}
            onfocus={() => searchStore.openGlobalSearch()}
            onblur={() => setTimeout(() => searchStore.closeGlobalSearch(), 200)}
            class="w-full h-9 pl-10 pr-4 py-2 bg-gray-100 dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-sm text-gray-900 dark:text-gray-100 placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400 focus:border-transparent transition-all"
          />

          {#if searchState.isGlobalSearchOpen && searchState.globalQuery}
            <div class="absolute top-full left-0 right-0 mt-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl max-h-96 overflow-y-auto z-50">
              {#if searchState.results.length > 0}
                <!-- Header -->
                <div class="p-3 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                  <p class="text-xs font-medium text-gray-500 dark:text-gray-400">
                    {searchState.results.length} {searchState.results.length === 1 ? 'result' : 'results'} for "{searchState.globalQuery}"
                  </p>
                  <div class="flex items-center gap-2 text-xs text-gray-400 dark:text-gray-500">
                    <span>↑↓ navigate</span>
                    <span>↵ select</span>
                    <span>esc close</span>
                  </div>
                </div>

                <!-- Categorized Results -->
                {#each Object.entries(categories) as [category, results]}
                  <div class="py-2">
                    <div class="px-3 py-1">
                      <h4 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">{category}</h4>
                    </div>
                    {#each results as result, index}
                      {@const globalIndex = searchState.results.indexOf(result)}
                      {@const isSelected = globalIndex === searchState.selectedIndex}
                      <button
                        onclick={() => navigateToResult(result)}
                        class={`w-full text-left px-3 py-2 flex items-start gap-3 transition-colors ${
                          isSelected
                            ? 'bg-blue-50 dark:bg-blue-900/20 border-l-2 border-blue-500 dark:border-blue-400'
                            : 'hover:bg-gray-50 dark:hover:bg-gray-700/50 border-l-2 border-transparent'
                        }`}
                      >
                        <!-- Icon based on type -->
                        <div class={`flex-shrink-0 w-8 h-8 rounded-lg flex items-center justify-center ${
                          result.type === 'server' ? 'bg-blue-100 dark:bg-blue-900/50 text-blue-600 dark:text-blue-400' :
                          result.type === 'tool' ? 'bg-purple-100 dark:bg-purple-900/50 text-purple-600 dark:text-purple-400' :
                          result.type === 'resource' ? 'bg-green-100 dark:bg-green-900/50 text-green-600 dark:text-green-400' :
                          result.type === 'prompt' ? 'bg-orange-100 dark:bg-orange-900/50 text-orange-600 dark:text-orange-400' :
                          'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400'
                        }`}>
                          {#if result.type === 'server'}
                            <Monitor size={16} />
                          {:else if result.type === 'tool'}
                            <Zap size={16} />
                          {:else if result.type === 'resource'}
                            <Database size={16} />
                          {:else if result.type === 'prompt'}
                            <FileText size={16} />
                          {:else}
                            <Layers3 size={16} />
                          {/if}
                        </div>

                        <!-- Content -->
                        <div class="flex-1 min-w-0">
                          <div class="flex items-center gap-2">
                            <span class="font-medium text-sm text-gray-900 dark:text-gray-100 truncate">{result.title}</span>
                            {#if result.metadata?.serverName}
                              <span class="text-xs text-gray-500 dark:text-gray-400 truncate">· {result.metadata.serverName}</span>
                            {/if}
                          </div>
                          {#if result.description}
                            <p class="text-xs text-gray-600 dark:text-gray-400 truncate mt-0.5">{result.description}</p>
                          {/if}
                        </div>

                        <!-- Score badge (debug) -->
                        {#if isSelected}
                          <div class="flex-shrink-0 px-2 py-1 bg-blue-100 dark:bg-blue-900/30 text-xs font-medium text-blue-700 dark:text-blue-300 rounded">
                            {result.score}%
                          </div>
                        {/if}
                      </button>
                    {/each}
                  </div>
                {/each}
              {:else}
                <!-- Empty state -->
                <div class="p-8 text-center">
                  <Search size={32} class="mx-auto text-gray-300 dark:text-gray-600 mb-3" />
                  <p class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-1">No results found</p>
                  <p class="text-xs text-gray-500 dark:text-gray-400">Try a different search term</p>
                </div>
              {/if}
            </div>
          {/if}
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

  /* Search styles now use Tailwind classes directly in the template */

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