<!--
  KeyValuePairInput - Reusable component for managing key-value pairs

  Use cases:
  - HTTP headers
  - Environment variables
  - Query parameters
  - Custom metadata

  Features:
  - Add/remove pairs dynamically
  - Customizable placeholders
  - Optional labels
  - Clean validation
-->
<script lang="ts">
  import { X } from 'lucide-svelte';

  let {
    pairs = $bindable([{ key: '', value: '' }]),
    label,
    keyPlaceholder = 'Key',
    valuePlaceholder = 'Value',
    addButtonText = '+ Add Pair',
    onUpdate
  } = $props<{
    pairs?: Array<{ key: string; value: string }>;
    label?: string;
    keyPlaceholder?: string;
    valuePlaceholder?: string;
    addButtonText?: string;
    onUpdate?: () => void;
  }>();

  function addPair() {
    pairs = [...pairs, { key: '', value: '' }];
    onUpdate?.();
  }

  function removePair(index: number) {
    pairs = pairs.filter((_: any, i: number) => i !== index);
    onUpdate?.();
  }

  function handleInput() {
    onUpdate?.();
  }
</script>

{#if label}
  <div>
    <label class="form-label">{label}</label>
    <div class="space-y-2">
      {#each pairs as pair, index}
        <div class="flex items-center space-x-2">
          <input
            type="text"
            bind:value={pair.key}
            oninput={handleInput}
            placeholder={keyPlaceholder}
            class="form-input flex-1"
          />
          <input
            type="text"
            bind:value={pair.value}
            oninput={handleInput}
            placeholder={valuePlaceholder}
            class="form-input flex-1"
          />
          <button
            onclick={() => removePair(index)}
            class="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition-colors"
            title="Remove pair"
          >
            <X size={16} />
          </button>
        </div>
      {/each}
      <button
        onclick={addPair}
        class="text-sm text-mcp-primary-600 hover:text-mcp-primary-700 dark:text-mcp-primary-400 dark:hover:text-mcp-primary-300 transition-colors"
      >
        {addButtonText}
      </button>
    </div>
  </div>
{:else}
  <!-- No label version for inline usage -->
  <div class="space-y-2">
    {#each pairs as pair, index}
      <div class="flex items-center space-x-2">
        <input
          type="text"
          bind:value={pair.key}
          oninput={handleInput}
          placeholder={keyPlaceholder}
          class="form-input flex-1"
        />
        <input
          type="text"
          bind:value={pair.value}
          oninput={handleInput}
          placeholder={valuePlaceholder}
          class="form-input flex-1"
        />
        <button
          onclick={() => removePair(index)}
          class="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition-colors"
          title="Remove pair"
        >
          <X size={16} />
        </button>
      </div>
    {/each}
    <button
      onclick={addPair}
      class="text-sm text-mcp-primary-600 hover:text-mcp-primary-700 dark:text-mcp-primary-400 dark:hover:text-mcp-primary-300 transition-colors"
    >
      {addButtonText}
    </button>
  </div>
{/if}
