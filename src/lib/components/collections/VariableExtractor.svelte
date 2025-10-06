<!--
  VariableExtractor - Configure variable extraction from step results

  Allows users to extract values from responses using JSONPath and store them
  as variables for use in subsequent steps.

  Example:
    - Extract $.result.auth_token and save as "token"
    - Use ${token} in next step's parameters
-->
<script lang="ts">
  import { Plus, Trash2, Info } from 'lucide-svelte';
  import type { VariableExtract } from '$lib/types/collections';

  interface Props {
    extracts: VariableExtract[];
    availableVariables?: string[];
  }

  let { extracts = $bindable([]), availableVariables = [] }: Props = $props();

  function addExtract() {
    extracts.push({
      source: 'response',
      path: '$.result',
      variable_name: 'extracted_value'
    });
  }

  function removeExtract(index: number) {
    extracts.splice(index, 1);
  }
</script>

<div class="border-t border-gray-200 dark:border-gray-700 pt-4 mt-4">
  <!-- Header -->
  <div class="flex items-center justify-between mb-3">
    <div class="flex items-center gap-2">
      <h4 class="text-sm font-semibold text-gray-900 dark:text-gray-100">
        Variable Extraction
      </h4>
      <button
        type="button"
        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
        title="Extract values from this step's response to use in later steps"
      >
        <Info size={14} />
      </button>
    </div>
    <button
      type="button"
      onclick={addExtract}
      class="flex items-center gap-1 px-2 py-1 text-sm bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 hover:bg-blue-100 dark:hover:bg-blue-900/30 rounded border border-blue-200 dark:border-blue-800"
    >
      <Plus size={14} />
      <span>Add</span>
    </button>
  </div>

  <!-- Info message when no extracts -->
  {#if extracts.length === 0}
    <div class="p-3 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
      <p class="text-sm text-gray-600 dark:text-gray-400">
        Extract values from this step's response to use in later steps.
        <br />
        <span class="text-xs mt-1 inline-block">
          Example: Extract <code class="px-1 py-0.5 bg-gray-200 dark:bg-gray-700 rounded">$.result.token</code> and use as <code class="px-1 py-0.5 bg-gray-200 dark:bg-gray-700 rounded">${'{token}'}</code> in next step
        </span>
      </p>
    </div>
  {/if}

  <!-- Extract list -->
  {#each extracts as extract, i}
    <div class="flex gap-2 mb-2 items-start p-3 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
      <!-- Source dropdown -->
      <div class="flex-shrink-0" style="width: 120px;">
        <label for="extract-source-{i}" class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
          Source
        </label>
        <select
          id="extract-source-{i}"
          bind:value={extract.source}
          class="w-full p-2 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="response">Response</option>
          <option value="status">Status</option>
          <option value="timing">Timing</option>
          <option value="error">Error</option>
        </select>
      </div>

      <!-- JSONPath input -->
      <div class="flex-1">
        <label for="extract-path-{i}" class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
          JSONPath
        </label>
        <input
          id="extract-path-{i}"
          type="text"
          bind:value={extract.path}
          placeholder="$.result.token"
          class="w-full p-2 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 font-mono focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      <!-- Variable name input -->
      <div class="flex-1">
        <label for="extract-var-{i}" class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
          Variable Name
        </label>
        <input
          id="extract-var-{i}"
          type="text"
          bind:value={extract.variable_name}
          placeholder="auth_token"
          class="w-full p-2 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      <!-- Delete button -->
      <div class="flex-shrink-0" style="padding-top: 20px;">
        <button
          type="button"
          onclick={() => removeExtract(i)}
          class="p-2 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded"
          title="Remove extraction"
        >
          <Trash2 size={14} />
        </button>
      </div>
    </div>
  {/each}

  <!-- Available variables info -->
  {#if availableVariables.length > 0 && extracts.length > 0}
    <div class="mt-3 p-2 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded text-xs">
      <span class="text-blue-700 dark:text-blue-300 font-medium">Available in later steps:</span>
      <span class="text-blue-600 dark:text-blue-400 ml-2">
        {#each availableVariables as varName, idx}
          {#if idx > 0}, {/if}
          <code class="px-1 py-0.5 bg-blue-100 dark:bg-blue-900/40 rounded">${'{' + varName + '}'}</code>
        {/each}
      </span>
    </div>
  {/if}
</div>
