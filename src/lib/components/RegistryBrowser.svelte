<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import {
    Search,
    RefreshCw,
    Filter,
    Package,
    Download,
    ExternalLink,
    Info,
    CheckCircle,
    AlertCircle,
    Loader,
  } from 'lucide-svelte';
  import Button from './ui/Button.svelte';
  import ServerConfigModal from './ServerConfigModal.svelte';
  import { uiStore } from '$lib/stores/uiStore';

  // Types
  interface RegistryServer {
    name: string;
    image?: string;
    type: 'server' | 'remote';
    about?: {
      title?: string;
      description?: string;
      icon?: string;
    };
    meta?: {
      category?: string;
      tags?: string[];
    };
    source?: {
      project?: string;
    };
    config?: any;
    oauth?: any[];
  }

  interface ServerDisplayInfo {
    name: string;
    title: string;
    description: string;
    icon?: string;
    category: string;
    tags: string[];
    server_type: 'server' | 'remote';
    is_docker_built: boolean;
    is_remote: boolean;
    has_oauth: boolean;
    github_url?: string;
  }

  // State
  let servers: Record<string, RegistryServer> = {};
  let displayServers: ServerDisplayInfo[] = [];
  let filteredServers: ServerDisplayInfo[] = [];
  let categories: string[] = [];
  let selectedCategory = 'all';
  let searchQuery = '';
  let loading = false;
  let selectedServer: RegistryServer | null = null;
  let showConfigModal = false;

  // Fetch registry on mount
  onMount(async () => {
    await fetchRegistry();
  });

  async function fetchRegistry() {
    loading = true;
    try {
      servers = await invoke('fetch_registry_catalog');
      categories = await invoke('get_registry_categories', { servers });
      updateDisplay();
      uiStore.showSuccess('Registry loaded successfully');
    } catch (error) {
      console.error('Failed to fetch registry:', error);
      uiStore.showError(`Failed to fetch registry: ${error}`);
    } finally {
      loading = false;
    }
  }

  async function refreshRegistry() {
    loading = true;
    try {
      servers = await invoke('refresh_registry_catalog');
      categories = await invoke('get_registry_categories', { servers });
      updateDisplay();
      uiStore.showSuccess('Registry refreshed successfully');
    } catch (error) {
      console.error('Failed to refresh registry:', error);
      uiStore.showError(`Failed to refresh registry: ${error}`);
    } finally {
      loading = false;
    }
  }

  function updateDisplay() {
    // Convert servers to display info
    displayServers = Object.values(servers).map((server) => {
      const about = server.about || {};
      const meta = server.meta || {};
      const source = server.source || {};

      return {
        name: server.name,
        title: about.title || server.name,
        description: about.description || '',
        icon: about.icon,
        category: meta.category || 'other',
        tags: meta.tags || [],
        server_type: server.type,
        is_docker_built: server.image?.startsWith('mcp/') || false,
        is_remote: server.type === 'remote',
        has_oauth: !!(server.oauth && server.oauth.length > 0),
        github_url: source.project,
      };
    });

    applyFilters();
  }

  function applyFilters() {
    let filtered = displayServers;

    // Filter by category
    if (selectedCategory !== 'all') {
      filtered = filtered.filter((s) => s.category === selectedCategory);
    }

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (s) =>
          s.name.toLowerCase().includes(query) ||
          s.title.toLowerCase().includes(query) ||
          s.description.toLowerCase().includes(query) ||
          s.tags.some((tag) => tag.toLowerCase().includes(query))
      );
    }

    filteredServers = filtered;
  }

  function handleCategoryChange(category: string) {
    selectedCategory = category;
    applyFilters();
  }

  function handleSearch(event: Event) {
    searchQuery = (event.target as HTMLInputElement).value;
    applyFilters();
  }

  function openConfigModal(serverName: string) {
    selectedServer = servers[serverName] || null;
    showConfigModal = true;
  }

  function closeConfigModal() {
    showConfigModal = false;
    selectedServer = null;
  }

  function getCategoryIcon(category: string): typeof Package {
    // Map categories to icons - can be expanded
    return Package;
  }

  function formatServerCount(count: number): string {
    return `${count} server${count !== 1 ? 's' : ''}`;
  }
</script>

<div class="h-full flex flex-col bg-white dark:bg-gray-900">
  <!-- Header -->
  <div class="border-b border-gray-200 dark:border-gray-700 p-4">
    <div class="flex items-center justify-between mb-4">
      <div>
        <h2 class="text-2xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
          <Package size={24} />
          Docker MCP Registry
        </h2>
        <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
          Browse and configure {formatServerCount(displayServers.length)} from the official registry
        </p>
      </div>

      <Button
        variant="secondary"
        leftIcon={RefreshCw}
        onclick={refreshRegistry}
        disabled={loading}
        title="Refresh registry (clears cache)"
      >
        Refresh
      </Button>
    </div>

    <!-- Search Bar -->
    <div class="relative">
      <Search class="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400" size={20} />
      <input
        type="text"
        placeholder="Search servers, tags, or descriptions..."
        value={searchQuery}
        oninput={handleSearch}
        class="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
      />
    </div>
  </div>

  <!-- Category Filter -->
  <div class="border-b border-gray-200 dark:border-gray-700 p-4 overflow-x-auto">
    <div class="flex gap-2">
      <button
        onclick={() => handleCategoryChange('all')}
        class={`px-4 py-2 rounded-lg text-sm font-medium transition-colors whitespace-nowrap ${
          selectedCategory === 'all'
            ? 'bg-blue-600 text-white'
            : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700'
        }`}
      >
        All
      </button>
      {#each categories as category}
        <button
          onclick={() => handleCategoryChange(category)}
          class={`px-4 py-2 rounded-lg text-sm font-medium transition-colors whitespace-nowrap ${
            selectedCategory === category
              ? 'bg-blue-600 text-white'
              : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700'
          }`}
        >
          {category}
        </button>
      {/each}
    </div>
  </div>

  <!-- Server Grid -->
  <div class="flex-1 overflow-y-auto p-4">
    {#if loading}
      <div class="flex items-center justify-center h-full">
        <div class="flex flex-col items-center gap-3">
          <Loader class="animate-spin text-blue-600" size={48} />
          <p class="text-gray-600 dark:text-gray-400">Loading registry...</p>
        </div>
      </div>
    {:else if filteredServers.length === 0}
      <div class="flex items-center justify-center h-full">
        <div class="text-center">
          <Package class="mx-auto mb-4 text-gray-400" size={64} />
          <h3 class="text-xl font-semibold text-gray-900 dark:text-white mb-2">
            No servers found
          </h3>
          <p class="text-gray-600 dark:text-gray-400">
            Try adjusting your search or category filter
          </p>
        </div>
      </div>
    {:else}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
        {#each filteredServers as server}
          <div
            class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:shadow-lg transition-all cursor-pointer group"
            onclick={() => openConfigModal(server.name)}
          >
            <!-- Server Header -->
            <div class="flex items-start gap-3 mb-3">
              {#if server.icon}
                <img
                  src={server.icon}
                  alt={server.title}
                  class="w-10 h-10 rounded-lg object-cover"
                  onerror={(e) => {
                    const img = e.currentTarget as HTMLImageElement;
                    img.style.display = 'none';
                    const fallback = img.nextElementSibling as HTMLElement | null;
                    fallback?.classList.remove('hidden');
                  }}
                />
                <div class="hidden w-10 h-10 rounded-lg bg-gray-200 dark:bg-gray-700 flex items-center justify-center">
                  <Package class="text-gray-500" size={24} />
                </div>
              {:else}
                <div class="w-10 h-10 rounded-lg bg-gray-200 dark:bg-gray-700 flex items-center justify-center">
                  <Package class="text-gray-500" size={24} />
                </div>
              {/if}

              <div class="flex-1 min-w-0">
                <h3 class="font-semibold text-gray-900 dark:text-white group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors truncate">
                  {server.title}
                </h3>
                <p class="text-xs text-gray-500 dark:text-gray-400">{server.name}</p>
              </div>
            </div>

            <!-- Description -->
            <p class="text-sm text-gray-600 dark:text-gray-300 mb-3 line-clamp-2">
              {server.description || 'No description available'}
            </p>

            <!-- Tags -->
            {#if server.tags.length > 0}
              <div class="flex flex-wrap gap-1 mb-3">
                {#each server.tags.slice(0, 3) as tag}
                  <span class="px-2 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 text-xs rounded-full">
                    {tag}
                  </span>
                {/each}
                {#if server.tags.length > 3}
                  <span class="px-2 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 text-xs rounded-full">
                    +{server.tags.length - 3}
                  </span>
                {/if}
              </div>
            {/if}

            <!-- Server Info Badges -->
            <div class="flex items-center gap-2 text-xs">
              {#if server.is_docker_built}
                <span class="flex items-center gap-1 text-green-600 dark:text-green-400">
                  <CheckCircle size={14} />
                  Docker Built
                </span>
              {/if}
              {#if server.is_remote}
                <span class="flex items-center gap-1 text-blue-600 dark:text-blue-400">
                  <ExternalLink size={14} />
                  Remote
                </span>
              {/if}
              {#if server.has_oauth}
                <span class="flex items-center gap-1 text-orange-600 dark:text-orange-400">
                  <AlertCircle size={14} />
                  OAuth
                </span>
              {/if}
            </div>

            <!-- GitHub Link -->
            {#if server.github_url}
              <div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
                <a
                  href={server.github_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="text-xs text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1"
                  onclick={(e) => e.stopPropagation()}
                >
                  <ExternalLink size={12} />
                  View on GitHub
                </a>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<!-- Config Modal -->
{#if showConfigModal && selectedServer}
  <ServerConfigModal server={selectedServer} onClose={closeConfigModal} />
{/if}

<style>
  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
