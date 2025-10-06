<!--
  SamplingStepConfig - Composed component for configuring sampling operations in Collections

  Sampling = Client requests LLM completion from server
  Critical: auto_approve mode for automated testing (no HITL required)
-->
<script lang="ts">
  import type { ServerInfo } from '$lib/stores/serverStore';
  import type { SamplingOperation } from '$lib/types/collections';
  import { Info, Plus, Trash2 } from 'lucide-svelte';

  interface Props {
    operation: SamplingOperation;
    servers: ServerInfo[];
    availableVariables?: string[];
  }

  let { operation, servers, availableVariables = [] }: Props = $props();

  // Initialize messages if empty
  if (!operation.messages || operation.messages.length === 0) {
    operation.messages = [
      {
        role: 'user',
        content: 'Write a function to reverse a string'
      }
    ];
  }

  function addMessage() {
    operation.messages.push({
      role: 'user',
      content: ''
    });
  }

  function removeMessage(index: number) {
    if (operation.messages.length > 1) {
      operation.messages.splice(index, 1);
    }
  }
</script>

<div class="space-y-4">
  <!-- Server Selection -->
  <div>
    <label for="sampling-server" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
      Server
    </label>
    <select
      id="sampling-server"
      bind:value={operation.server_alias}
      class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
    >
      <option value="">Select a server...</option>
      {#each servers as server}
        <option value={server.id}>{server.config.name}</option>
      {/each}
    </select>
  </div>

  <!-- Messages Editor -->
  <div>
    <div class="flex items-center justify-between mb-2">
      <div class="flex items-center gap-2">
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
          Messages
        </label>
        <button
          type="button"
          class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
          title="Conversation history sent to the LLM. Use ${'{variable}'} syntax for interpolation."
        >
          <Info size={14} />
        </button>
      </div>
      <button
        type="button"
        onclick={addMessage}
        class="flex items-center gap-1 px-2 py-1 text-xs bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 hover:bg-blue-100 dark:hover:bg-blue-900/30 rounded border border-blue-200 dark:border-blue-800"
      >
        <Plus size={12} />
        <span>Add Message</span>
      </button>
    </div>

    <div class="space-y-2">
      {#each operation.messages as message, i}
        <div class="border border-gray-200 dark:border-gray-700 rounded-lg p-3 bg-gray-50 dark:bg-gray-800">
          <div class="flex items-center justify-between mb-2">
            <select
              bind:value={message.role}
              class="px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
            >
              <option value="user">User</option>
              <option value="assistant">Assistant</option>
              <option value="system">System</option>
            </select>

            <button
              type="button"
              onclick={() => removeMessage(i)}
              disabled={operation.messages.length === 1}
              class="p-1 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded disabled:opacity-50 disabled:cursor-not-allowed"
              title="Remove message"
            >
              <Trash2 size={14} />
            </button>
          </div>

          <textarea
            bind:value={message.content}
            placeholder="Message content... (use ${'{variable}'} for interpolation)"
            rows="3"
            class="w-full p-2 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          ></textarea>
        </div>
      {/each}
    </div>
  </div>

  <!-- Sampling Parameters -->
  <div class="grid grid-cols-2 gap-3">
    <div>
      <label for="max-tokens" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        Max Tokens
      </label>
      <input
        id="max-tokens"
        type="number"
        bind:value={operation.max_tokens}
        min="1"
        max="100000"
        placeholder="1000"
        class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>

    <div>
      <label for="temperature" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        Temperature
      </label>
      <input
        id="temperature"
        type="number"
        bind:value={operation.temperature}
        min="0"
        max="2"
        step="0.1"
        placeholder="0.7"
        class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>
  </div>

  <!-- Auto-approve (CRITICAL for testing) -->
  <div class="border-t border-gray-200 dark:border-gray-700 pt-3">
    <div class="flex items-start gap-3 p-3 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg">
      <Info size={16} class="text-yellow-600 dark:text-yellow-400 flex-shrink-0 mt-0.5" />
      <div class="flex-1">
        <label class="flex items-center gap-2 text-sm cursor-pointer">
          <input
            type="checkbox"
            bind:checked={operation.auto_approve}
            class="rounded border-gray-300 dark:border-gray-600 text-yellow-600 focus:ring-yellow-500"
          />
          <span class="font-medium text-yellow-700 dark:text-yellow-300">
            Auto-approve (HITL bypass for testing)
          </span>
        </label>
        <p class="text-xs text-yellow-600 dark:text-yellow-400 mt-1 ml-6">
          When enabled, sampling requests will automatically proceed without human approval.
          <strong>Use only for automated testing.</strong>
          In production, leave unchecked for human-in-the-loop workflows.
        </p>
      </div>
    </div>
  </div>

  <!-- Available variables hint -->
  {#if availableVariables.length > 0}
    <div class="p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
      <p class="text-sm text-blue-700 dark:text-blue-300 font-medium mb-2">
        Available Variables (click to insert):
      </p>
      <div class="flex flex-wrap gap-2">
        {#each availableVariables as varName}
          <code class="px-2 py-1 text-xs bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 rounded border border-blue-200 dark:border-blue-700">
            ${'{'}{varName}{'}'}
          </code>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Example message -->
  <div class="text-xs text-gray-500 dark:text-gray-400 border-t border-gray-200 dark:border-gray-700 pt-3">
    <p class="font-medium mb-1">Example with variable interpolation:</p>
    <code class="block p-2 bg-gray-100 dark:bg-gray-800 rounded">
      "Generate a summary of: ${'{'}{'{'}document_content{'}'}{'}'}"
    </code>
  </div>
</div>
