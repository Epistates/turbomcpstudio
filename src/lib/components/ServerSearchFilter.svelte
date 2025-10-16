<script lang="ts">
  import { Search, X, Filter, SortAsc, ChevronDown } from 'lucide-svelte';
  import { type ServerInfo } from '$lib/stores/serverStore';

  interface Props {
    servers: ServerInfo[];
    onFilterChange?: (filtered: ServerInfo[]) => void;
  }

  let { servers, onFilterChange }: Props = $props();

  // Filter state
  let searchQuery = $state('');
  let selectedStatuses = $state<Set<string>>(new Set());
  let selectedTransports = $state<Set<string>>(new Set());
  let sortBy = $state<'name' | 'status' | 'lastUsed' | 'responseTime'>('name');
  let sortDirection = $state<'asc' | 'desc'>('asc');
  let showFilters = $state(false);

  // Available filter options
  const statusOptions = ['connected', 'disconnected', 'error'];
  const transportOptions = ['stdio', 'http', 'websocket', 'tcp', 'unix'];

  // Debounced search
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;
  function handleSearchInput(value: string) {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      searchQuery = value;
    }, 300); // 300ms debounce
  }

  // Filtered and sorted servers
  const filteredServers = $derived(() => {
    let result = servers;

    // Search filter (fuzzy)
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(s =>
        s.config.name.toLowerCase().includes(query) ||
        s.config.description?.toLowerCase().includes(query) ||
        s.id.toLowerCase().includes(query)
      );
    }

    // Status filter
    if (selectedStatuses.size > 0) {
      result = result.filter(s => selectedStatuses.has(s.status));
    }

    // Transport filter
    if (selectedTransports.size > 0) {
      result = result.filter(s =>
        s.config.transport_config?.type &&
        selectedTransports.has(s.config.transport_config.type)
      );
    }

    // Sorting
    result = [...result].sort((a, b) => {
      let comparison = 0;

      switch (sortBy) {
        case 'name':
          comparison = a.config.name.localeCompare(b.config.name);
          break;
        case 'status':
          const statusOrder = { connected: 0, connecting: 1, disconnected: 2, error: 3 };
          comparison = statusOrder[a.status] - statusOrder[b.status];
          break;
        case 'lastUsed':
          const aTime = a.last_seen ? new Date(a.last_seen).getTime() : 0;
          const bTime = b.last_seen ? new Date(b.last_seen).getTime() : 0;
          comparison = bTime - aTime; // Most recent first
          break;
        case 'responseTime':
          comparison = (a.metrics?.avg_response_time_ms || Infinity) -
                      (b.metrics?.avg_response_time_ms || Infinity);
          break;
      }

      return sortDirection === 'asc' ? comparison : -comparison;
    });

    return result;
  });

  // Notify parent of filter changes
  $effect(() => {
    if (onFilterChange) {
      onFilterChange(filteredServers());
    }
  });

  function toggleStatus(status: string) {
    if (selectedStatuses.has(status)) {
      selectedStatuses.delete(status);
    } else {
      selectedStatuses.add(status);
    }
    selectedStatuses = new Set(selectedStatuses);
  }

  function toggleTransport(transport: string) {
    if (selectedTransports.has(transport)) {
      selectedTransports.delete(transport);
    } else {
      selectedTransports.add(transport);
    }
    selectedTransports = new Set(selectedTransports);
  }

  function clearFilters() {
    searchQuery = '';
    selectedStatuses.clear();
    selectedStatuses = new Set();
    selectedTransports.clear();
    selectedTransports = new Set();
  }

  function toggleSort(field: typeof sortBy) {
    if (sortBy === field) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = field;
      sortDirection = 'asc';
    }
  }

  const hasActiveFilters = $derived(
    searchQuery.trim() !== '' ||
    selectedStatuses.size > 0 ||
    selectedTransports.size > 0
  );
</script>

<div class="search-filter-container">
  <!-- Search Bar -->
  <div class="search-bar">
    <Search size={18} class="search-icon" />
    <input
      type="text"
      class="search-input"
      placeholder="Search servers by name, description, or ID..."
      value={searchQuery}
      oninput={(e) => handleSearchInput((e.target as HTMLInputElement).value)}
      onkeydown={(e) => {
        if (e.key === 'Escape') {
          searchQuery = '';
          (e.target as HTMLInputElement).value = '';
        }
      }}
    />
    {#if searchQuery}
      <button
        class="clear-search"
        onclick={() => {
          searchQuery = '';
          const input = document.querySelector('.search-input') as HTMLInputElement;
          if (input) input.value = '';
        }}
        aria-label="Clear search"
      >
        <X size={16} />
      </button>
    {/if}
  </div>

  <!-- Filter & Sort Controls -->
  <div class="controls">
    <button
      class="control-btn"
      class:active={showFilters}
      onclick={() => showFilters = !showFilters}
    >
      <Filter size={16} />
      <span>Filters</span>
      {#if hasActiveFilters}
        <span class="filter-badge">{selectedStatuses.size + selectedTransports.size}</span>
      {/if}
      <div class="chevron" class:rotated={showFilters}>
        <ChevronDown size={14} />
      </div>
    </button>

    <div class="sort-controls">
      <button class="control-btn" onclick={() => toggleSort('name')}>
        <SortAsc size={16} />
        <span>Name</span>
        {#if sortBy === 'name'}
          <span class="sort-indicator">{sortDirection === 'asc' ? '↑' : '↓'}</span>
        {/if}
      </button>
      <button class="control-btn" onclick={() => toggleSort('status')}>
        <span>Status</span>
        {#if sortBy === 'status'}
          <span class="sort-indicator">{sortDirection === 'asc' ? '↑' : '↓'}</span>
        {/if}
      </button>
      <button class="control-btn" onclick={() => toggleSort('responseTime')}>
        <span>Performance</span>
        {#if sortBy === 'responseTime'}
          <span class="sort-indicator">{sortDirection === 'asc' ? '↑' : '↓'}</span>
        {/if}
      </button>
    </div>

    {#if hasActiveFilters}
      <button class="clear-btn" onclick={clearFilters}>
        Clear Filters
      </button>
    {/if}
  </div>

  <!-- Filter Panel -->
  {#if showFilters}
    <div class="filter-panel">
      <!-- Status Filters -->
      <div class="filter-section">
        <h4>Status</h4>
        <div class="filter-chips">
          {#each statusOptions as status}
            <button
              class="filter-chip"
              class:active={selectedStatuses.has(status)}
              onclick={() => toggleStatus(status)}
            >
              <div class="status-dot status-{status}"></div>
              <span>{status}</span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Transport Filters -->
      <div class="filter-section">
        <h4>Transport</h4>
        <div class="filter-chips">
          {#each transportOptions as transport}
            <button
              class="filter-chip"
              class:active={selectedTransports.has(transport)}
              onclick={() => toggleTransport(transport)}
            >
              <span class="transport-badge">{transport.toUpperCase()}</span>
            </button>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  <!-- Results Summary -->
  <div class="results-summary">
    <span class="result-count">
      {filteredServers().length} of {servers.length} servers
      {#if hasActiveFilters}
        <span class="filtered-note">(filtered)</span>
      {/if}
    </span>
  </div>
</div>

<style>
  .search-filter-container {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-4);
    padding: var(--mcp-space-4);
    background: var(--mcp-surface-primary);
    border-bottom: 1px solid var(--mcp-border-primary);
  }

  /* Search Bar */
  .search-bar {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: var(--mcp-space-3);
    color: var(--mcp-text-tertiary);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: var(--mcp-space-3) var(--mcp-space-10) var(--mcp-space-3) var(--mcp-space-10);
    background: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    color: var(--mcp-text-primary);
    font-size: var(--mcp-text-sm);
    transition: all 0.2s ease;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--mcp-primary-500);
    box-shadow: 0 0 0 3px var(--mcp-primary-100);
  }

  .clear-search {
    position: absolute;
    right: var(--mcp-space-3);
    padding: var(--mcp-space-1);
    background: transparent;
    border: none;
    color: var(--mcp-text-tertiary);
    cursor: pointer;
    border-radius: var(--mcp-radius-sm);
    transition: all 0.2s ease;
  }

  .clear-search:hover {
    background: var(--mcp-surface-tertiary);
    color: var(--mcp-text-secondary);
  }

  /* Controls */
  .controls {
    display: flex;
    gap: var(--mcp-space-3);
    flex-wrap: wrap;
    align-items: center;
  }

  .control-btn {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-primary);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .control-btn:hover {
    background: var(--mcp-surface-tertiary);
    border-color: var(--mcp-primary-500);
  }

  .control-btn.active {
    background: var(--mcp-primary-100);
    border-color: var(--mcp-primary-500);
    color: var(--mcp-primary-700);
  }

  .filter-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    background: var(--mcp-primary-600);
    color: white;
    font-size: 11px;
    font-weight: var(--mcp-font-bold);
    border-radius: var(--mcp-radius-full);
  }

  .sort-indicator {
    font-size: 12px;
    color: var(--mcp-primary-600);
  }

  .chevron {
    transition: transform 0.2s ease;
  }

  .chevron.rotated {
    transform: rotate(180deg);
  }

  .sort-controls {
    display: flex;
    gap: var(--mcp-space-2);
  }

  .clear-btn {
    margin-left: auto;
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: transparent;
    border: none;
    color: var(--mcp-error-600);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .clear-btn:hover {
    color: var(--mcp-error-700);
    text-decoration: underline;
  }

  /* Filter Panel */
  .filter-panel {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-4);
    padding: var(--mcp-space-4);
    background: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
  }

  .filter-section h4 {
    margin: 0 0 var(--mcp-space-2) 0;
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .filter-chips {
    display: flex;
    gap: var(--mcp-space-2);
    flex-wrap: wrap;
  }

  .filter-chip {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-primary);
    font-size: var(--mcp-text-sm);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .filter-chip:hover {
    border-color: var(--mcp-primary-500);
    background: var(--mcp-surface-tertiary);
  }

  .filter-chip.active {
    background: var(--mcp-primary-100);
    border-color: var(--mcp-primary-500);
    color: var(--mcp-primary-700);
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .status-dot.status-connected {
    background: var(--mcp-success-500);
  }

  .status-dot.status-disconnected {
    background: var(--mcp-gray-400);
  }

  .status-dot.status-error {
    background: var(--mcp-error-500);
  }

  .transport-badge {
    font-size: 11px;
    font-weight: var(--mcp-font-bold);
  }

  /* Results Summary */
  .results-summary {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: var(--mcp-space-3);
    border-top: 1px solid var(--mcp-border-primary);
  }

  .result-count {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
  }

  .filtered-note {
    color: var(--mcp-primary-600);
    font-weight: var(--mcp-font-medium);
  }

  /* Responsive */
  @media (max-width: 768px) {
    .controls {
      flex-direction: column;
      align-items: stretch;
    }

    .sort-controls {
      width: 100%;
      justify-content: space-between;
    }

    .clear-btn {
      margin-left: 0;
    }
  }
</style>
