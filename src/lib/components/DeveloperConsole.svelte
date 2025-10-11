<script lang="ts">
  import { logStore, type LogEntry, type LogLevel } from '$lib/stores/logStore';
  import { uiStore } from '$lib/stores/uiStore';
  import { X, ChevronDown, ChevronUp, Download, Trash2, Filter, Search } from 'lucide-svelte';
  import { save } from '@tauri-apps/plugin-dialog';
  import { writeTextFile } from '@tauri-apps/plugin-fs';
  import { createLogger } from '$lib/utils/logger';

  // Initialize scoped logger
  const logger = createLogger('DeveloperConsole');

  const { isopen, onclose } = $props<{
    isopen: boolean;
    onclose: () => void;
  }>();

  const logState = $derived($logStore);
  const filteredLogs = $derived(logStore.getFilteredLogs(logState));

  let searchQuery = $state('');
  let selectedLevels = $state<Set<LogLevel>>(new Set(['debug', 'info', 'warn', 'error']));
  let showFilters = $state(false);
  let autoScroll = $state(true);

  // Available log sources
  const sources = $derived(() => {
    const sourceSet = new Set<string>();
    logState.logs.forEach((log) => sourceSet.add(log.source));
    return Array.from(sourceSet).sort();
  });

  let selectedSources = $state<Set<string>>(new Set());

  // Apply filters
  $effect(() => {
    logStore.setLevelFilter(selectedLevels);
    logStore.setSourceFilter(selectedSources);
    logStore.setSearchQuery(searchQuery);
  });

  function handleClearLogs() {
    if (confirm('Clear all logs?')) {
      logStore.clear();
    }
  }

  async function handleDownloadLogs() {
    try {
      // Generate default filename with timestamp
      const timestamp = new Date().toISOString().replace(/:/g, '-').replace(/\..+/, '');
      const defaultFilename = `mcp-studio-logs-${timestamp}.json`;

      logger.debug('📁 Opening save dialog...');

      // Show native save dialog
      const filePath = await save({
        defaultPath: defaultFilename,
        filters: [
          {
            name: 'JSON Files',
            extensions: ['json']
          },
          {
            name: 'All Files',
            extensions: ['*']
          }
        ],
        title: 'Save Logs'
      });

      // User cancelled the dialog
      if (!filePath) {
        logger.debug('❌ Log download cancelled by user');
        return;
      }

      logger.debug(`💾 Writing logs to: ${filePath}`);

      // Export logs and write to file
      const logsJson = logStore.exportLogs(logState);
      await writeTextFile(filePath, logsJson);

      const logCount = logState.logs.length;
      logger.info(`✅ Successfully saved ${logCount} log(s) to: ${filePath}`);
      uiStore.showSuccess(`Saved ${logCount} log(s) to ${filePath.split('/').pop() || 'file'}`);
    } catch (error) {
      logger.error('❌ Failed to save logs:', error);
      uiStore.showError(`Failed to save logs: ${error}`);
    }
  }

  function toggleLevel(level: LogLevel) {
    if (selectedLevels.has(level)) {
      selectedLevels.delete(level);
    } else {
      selectedLevels.add(level);
    }
    selectedLevels = new Set(selectedLevels); // Trigger reactivity
  }

  function toggleSource(source: string) {
    if (selectedSources.has(source)) {
      selectedSources.delete(source);
    } else {
      selectedSources.add(source);
    }
    selectedSources = new Set(selectedSources); // Trigger reactivity
  }

  function getLevelColor(level: LogLevel): string {
    switch (level) {
      case 'debug':
        return 'text-gray-400';
      case 'info':
        return 'text-blue-400';
      case 'warn':
        return 'text-yellow-400';
      case 'error':
        return 'text-red-400';
    }
  }

  function getLevelBg(level: LogLevel): string {
    switch (level) {
      case 'debug':
        return 'bg-gray-900';
      case 'info':
        return 'bg-blue-900/20';
      case 'warn':
        return 'bg-yellow-900/20';
      case 'error':
        return 'bg-red-900/20';
    }
  }

  function formatTimestamp(timestamp: string): string {
    const date = new Date(timestamp);
    return date.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit', fractionalSecondDigits: 3 });
  }
</script>

{#if isopen}
  <div
    class="fixed bottom-8 left-0 right-0 bg-gray-900 dark:bg-black border-t border-gray-700 dark:border-gray-800 flex flex-col shadow-2xl z-40"
    style="height: 400px;"
  >
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-2 bg-gray-800 dark:bg-gray-950 border-b border-gray-700 dark:border-gray-800">
      <div class="flex items-center gap-3">
        <h3 class="text-sm font-semibold text-white">Developer Console</h3>
        <span class="text-xs text-gray-400">
          {filteredLogs.length} / {logState.logs.length} logs
        </span>
      </div>

      <div class="flex items-center gap-2">
        <!-- Search -->
        <div class="relative">
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search logs..."
            class="w-48 px-2 py-1 text-xs bg-gray-700 dark:bg-gray-900 text-white border border-gray-600 rounded focus:outline-none focus:border-blue-500"
          />
          <Search size={12} class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400" />
        </div>

        <!-- Filter Toggle -->
        <button
          onclick={() => (showFilters = !showFilters)}
          class="p-1 hover:bg-gray-700 dark:hover:bg-gray-800 rounded text-gray-300 transition-colors"
          title="Toggle Filters"
        >
          <Filter size={14} />
        </button>

        <!-- Auto Scroll Toggle -->
        <button
          onclick={() => (autoScroll = !autoScroll)}
          class="px-2 py-1 text-xs rounded transition-colors {autoScroll
            ? 'bg-blue-600 text-white'
            : 'bg-gray-700 text-gray-300 hover:bg-gray-600'}"
          title="Auto-scroll to new logs"
        >
          Auto
        </button>

        <!-- Download -->
        <button
          onclick={handleDownloadLogs}
          class="p-1 hover:bg-gray-700 dark:hover:bg-gray-800 rounded text-gray-300 transition-colors"
          title="Download Logs"
        >
          <Download size={14} />
        </button>

        <!-- Clear -->
        <button
          onclick={handleClearLogs}
          class="p-1 hover:bg-gray-700 dark:hover:bg-gray-800 rounded text-red-400 transition-colors"
          title="Clear Logs"
        >
          <Trash2 size={14} />
        </button>

        <!-- Close -->
        <button
          onclick={onclose}
          class="p-1 hover:bg-gray-700 dark:hover:bg-gray-800 rounded text-gray-300 transition-colors"
          title="Close Console"
        >
          <X size={14} />
        </button>
      </div>
    </div>

    <!-- Filters Panel -->
    {#if showFilters}
      <div class="px-4 py-2 bg-gray-800 dark:bg-gray-950 border-b border-gray-700 dark:border-gray-800 flex gap-4">
        <!-- Level Filters -->
        <div class="flex items-center gap-2">
          <span class="text-xs text-gray-400">Levels:</span>
          {#each ['debug', 'info', 'warn', 'error'] as level}
            <button
              onclick={() => toggleLevel(level as LogLevel)}
              class="px-2 py-0.5 text-xs rounded transition-colors {selectedLevels.has(level as LogLevel)
                ? 'bg-blue-600 text-white'
                : 'bg-gray-700 text-gray-300 hover:bg-gray-600'}"
            >
              {level}
            </button>
          {/each}
        </div>

        <!-- Source Filters -->
        {#if sources().length > 0}
          <div class="flex items-center gap-2">
            <span class="text-xs text-gray-400">Sources:</span>
            {#each sources() as source}
              <button
                onclick={() => toggleSource(source)}
                class="px-2 py-0.5 text-xs rounded transition-colors {selectedSources.has(source)
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'}"
              >
                {source}
              </button>
            {/each}
            {#if selectedSources.size > 0}
              <button
                onclick={() => (selectedSources = new Set())}
                class="px-2 py-0.5 text-xs rounded bg-gray-700 text-gray-300 hover:bg-gray-600"
              >
                Clear
              </button>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Logs Container -->
    <div class="flex-1 overflow-y-auto p-2 font-mono text-xs">
      {#if filteredLogs.length === 0}
        <div class="flex items-center justify-center h-full text-gray-500">
          No logs to display
        </div>
      {:else}
        {#each filteredLogs as log (log.id)}
          <div class="flex items-start gap-2 py-1 px-2 hover:bg-gray-800 dark:hover:bg-gray-900 rounded {getLevelBg(log.level)}">
            <!-- Timestamp -->
            <span class="text-gray-500 shrink-0">{formatTimestamp(log.timestamp)}</span>

            <!-- Level -->
            <span class="shrink-0 w-12 {getLevelColor(log.level)} font-semibold uppercase">
              {log.level}
            </span>

            <!-- Source -->
            <span class="text-gray-400 shrink-0 w-24 truncate" title={log.source}>
              [{log.source}]
            </span>

            <!-- Message -->
            <span class="flex-1 text-gray-200 break-all">{log.message}</span>

            <!-- Details (if any) -->
            {#if log.details}
              <button
                class="text-blue-400 hover:text-blue-300 shrink-0"
                title="View details"
                onclick={() => console.log('Log details:', log.details)}
              >
                ⋯
              </button>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  </div>
{/if}
