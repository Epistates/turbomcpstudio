<!--
  ResourceStepConfig - Composed component for configuring resource operations in Collections

  Resources use URI templates (e.g., file:///path/to/{filename})
  Supports variable interpolation: file:///${folder}/${file}
-->
<script lang="ts">
  import type { ServerInfo } from '$lib/stores/serverStore';
  import type { ResourceOperation } from '$lib/types/collections';
  import { Info } from 'lucide-svelte';

  interface Props {
    operation: ResourceOperation;
    servers: ServerInfo[];
    availableVariables?: string[];
  }

  let { operation, servers, availableVariables = [] }: Props = $props();
</script>

<div class="space-y-4">
  <!-- Server Selection -->
  <div>
    <label for="resource-server" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
      Server
    </label>
    <select
      id="resource-server"
      bind:value={operation.server_alias}
      class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
    >
      <option value="">Select a server...</option>
      {#each servers as server}
        <option value={server.id}>{server.config.name}</option>
      {/each}
    </select>
  </div>

  <!-- Resource URI Input -->
  <div>
    <div class="flex items-center gap-2 mb-2">
      <label for="resource-uri" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
        Resource URI
      </label>
      <button
        type="button"
        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
        title="Use &#36;&#123;variable&#125; syntax to interpolate variables from previous steps"
      >
        <Info size={14} />
      </button>
    </div>

    <input
      id="resource-uri"
      type="text"
      bind:value={operation.resource_uri}
      placeholder="file:///path/to/resource or http://api.example.com/&#36;&#123;id&#125;"
      class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
    />

    <p class="mt-2 text-xs text-gray-600 dark:text-gray-400">
      Use URI templates with variable interpolation. Example: <code class="px-1 py-0.5 bg-gray-200 dark:bg-gray-700 rounded">file:///&#123;folder&#125;/&#123;file&#125;</code>
    </p>
  </div>

  <!-- Available variables hint -->
  {#if availableVariables.length > 0}
    <div class="p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
      <p class="text-sm text-blue-700 dark:text-blue-300 font-medium mb-2">
        Available Variables:
      </p>
      <div class="flex flex-wrap gap-2">
        {#each availableVariables as varName}
          <button
            type="button"
            onclick={() => {
              // Insert variable at cursor position
              const cursorPos = operation.resource_uri.length;
              operation.resource_uri =
                operation.resource_uri.slice(0, cursorPos) +
                '${' + varName + '}' +
                operation.resource_uri.slice(cursorPos);
            }}
            class="px-2 py-1 text-xs bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 rounded border border-blue-200 dark:border-blue-700 hover:bg-blue-200 dark:hover:bg-blue-900/60"
            title="Click to insert"
          >
            &#36;&#123;{varName}&#125;
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Common URI patterns -->
  <div class="border-t border-gray-200 dark:border-gray-700 pt-3">
    <p class="text-xs text-gray-600 dark:text-gray-400 mb-2">
      Common patterns:
    </p>
    <div class="space-y-1 text-xs">
      <button
        type="button"
        onclick={() => operation.resource_uri = 'file:///path/to/file.txt'}
        class="block text-left w-full px-2 py-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
      >
        <code class="text-gray-700 dark:text-gray-300">file:///path/to/file.txt</code>
        <span class="text-gray-500 ml-2">- Local file</span>
      </button>
      <button
        type="button"
        onclick={() => operation.resource_uri = 'http://api.example.com/v1/resource'}
        class="block text-left w-full px-2 py-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
      >
        <code class="text-gray-700 dark:text-gray-300">http://api.example.com/v1/resource</code>
        <span class="text-gray-500 ml-2">- HTTP endpoint</span>
      </button>
      <button
        type="button"
        onclick={() => operation.resource_uri = 'db://database/table/&#36;&#123;id&#125;'}
        class="block text-left w-full px-2 py-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
      >
        <code class="text-gray-700 dark:text-gray-300">db://database/table/&#36;&#123;id&#125;</code>
        <span class="text-gray-500 ml-2">- Database with variable</span>
      </button>
    </div>
  </div>
</div>
