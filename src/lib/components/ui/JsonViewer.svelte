<!--
  JsonViewer - Production-grade JSON renderer with syntax highlighting

  Features:
  - Recursive collapsible tree view
  - Syntax highlighting for all JSON types
  - Copy to clipboard
  - Search/filter
  - Dark mode support
  - Type indicators
  - Compact and expanded modes
-->
<script lang="ts">
  import { Copy, ChevronRight, ChevronDown, Search } from 'lucide-svelte';

  let {
    data,
    expanded = true,
    showCopy = true,
    showSearch = false,
    maxHeight,
    title,
    embedded = false,
    searchTerm = $bindable('')
  } = $props<{
    data: any;
    expanded?: boolean;
    showCopy?: boolean;
    showSearch?: boolean;
    maxHeight?: string;
    title?: string;
    embedded?: boolean;
    searchTerm?: string;
  }>();

  let copyFeedback = $state<string | null>(null);
  let expandedPaths = $state(new Set<string>());

  // Initialize all paths as expanded if expanded=true
  $effect(() => {
    if (expanded) {
      expandedPaths = new Set<string>(['root']);
    }
  });

  function copyToClipboard(value: any) {
    const text = typeof value === 'string' ? value : JSON.stringify(value, null, 2);
    navigator.clipboard.writeText(text).then(() => {
      copyFeedback = 'Copied!';
      setTimeout(() => copyFeedback = null, 2000);
    });
  }

  function getType(value: any): string {
    if (value === null) return 'null';
    if (Array.isArray(value)) return 'array';
    return typeof value;
  }

  function matchesSearch(obj: any, term: string): boolean {
    if (!term) return true;
    const lowerTerm = term.toLowerCase();
    const str = JSON.stringify(obj).toLowerCase();
    return str.includes(lowerTerm);
  }

  function isPathExpanded(path: string): boolean {
    return expandedPaths.has(path) || expanded;
  }

  function togglePath(path: string) {
    if (expandedPaths.has(path)) {
      expandedPaths.delete(path);
    } else {
      expandedPaths.add(path);
    }
    expandedPaths = new Set(expandedPaths);
  }
</script>

{#if !embedded && (title || showSearch || showCopy)}
  <div class="json-viewer-header">
    {#if title}
      <h4 class="json-viewer-title">{title}</h4>
    {/if}

    <div class="json-viewer-actions">
      {#if showSearch}
        <div class="json-search">
          <Search size={14} />
          <input
            type="text"
            bind:value={searchTerm}
            placeholder="Search JSON..."
            class="json-search-input"
          />
        </div>
      {/if}

      {#if showCopy}
        <button
          onclick={() => copyToClipboard(data)}
          class="json-copy-btn"
          title="Copy JSON"
        >
          <Copy size={14} />
          {#if copyFeedback}
            <span class="copy-feedback">{copyFeedback}</span>
          {/if}
        </button>
      {/if}
    </div>
  </div>
{/if}

<div class="json-viewer {embedded ? 'json-viewer-embedded' : ''}" style:max-height={maxHeight}>
  {@render JsonNode({ data, path: "root", searchTerm })}
</div>

<!-- Recursive JSON Node Component -->
{#snippet JsonNode(props: { data: any, path: string, searchTerm: string })}
  {@const { data, path, searchTerm } = props}
  {@const type = getType(data)}
  {@const isExpandable = type === 'object' || type === 'array'}
  {@const matches = matchesSearch(data, searchTerm)}

  {#if !matches && searchTerm}
    <!-- Skip rendering if doesn't match search -->
  {:else if type === 'object' && data !== null}
    {@render JsonObject({ data, path, searchTerm })}
  {:else if type === 'array'}
    {@render JsonArray({ data, path, searchTerm })}
  {:else}
    {@render JsonPrimitive({ value: data, type })}
  {/if}
{/snippet}

<!-- Object Renderer -->
{#snippet JsonObject(props: { data: Record<string, any>, path: string, searchTerm: string })}
  {@const { data, path, searchTerm } = props}
  {@const keys = Object.keys(data)}
  {#if keys.length === 0}
    <span class="json-empty">&#123;&#125;</span>
  {:else}
    {#each keys as key, index}
      {@const value = data[key]}
      {@const valueType = getType(value)}
      {@const isExpandable = valueType === 'object' || valueType === 'array'}
      {@const itemPath = `${path}.${key}`}
      {@const itemExpanded = isPathExpanded(itemPath)}
      <div class="json-line">
        <div class="json-key-row">
          {#if isExpandable}
            <button
              class="json-expand-btn"
              onclick={() => togglePath(itemPath)}
            >
              {#if itemExpanded}
                <ChevronDown size={14} />
              {:else}
                <ChevronRight size={14} />
              {/if}
            </button>
          {:else}
            <span class="json-expand-placeholder"></span>
          {/if}

          <span class="json-key">"{key}"</span>
          <span class="json-colon">:</span>

          {#if !isExpandable}
            {@render JsonNode({ data: value, path: itemPath, searchTerm })}
          {:else if !itemExpanded}
            <span class="json-collapsed">
              {valueType === 'array' ? `[${value.length}]` : '{...}'}
            </span>
          {/if}

          <button
            class="json-copy-inline-btn"
            onclick={() => copyToClipboard(value)}
            title="Copy value"
          >
            <Copy size={12} />
          </button>
        </div>

        {#if isExpandable && itemExpanded}
          <div class="json-nested">
            {@render JsonNode({ data: value, path: itemPath, searchTerm })}
          </div>
        {/if}
      </div>
    {/each}
  {/if}
{/snippet}

<!-- Array Renderer -->
{#snippet JsonArray(props: { data: any[], path: string, searchTerm: string })}
  {@const { data, path, searchTerm } = props}
  {#if data.length === 0}
    <span class="json-empty">[]</span>
  {:else}
    {#each data as item, index}
      {@const valueType = getType(item)}
      {@const isExpandable = valueType === 'object' || valueType === 'array'}
      {@const itemPath = `${path}[${index}]`}
      {@const itemExpanded = isPathExpanded(itemPath)}
      <div class="json-line">
        <div class="json-key-row">
          {#if isExpandable}
            <button
              class="json-expand-btn"
              onclick={() => togglePath(itemPath)}
            >
              {#if itemExpanded}
                <ChevronDown size={14} />
              {:else}
                <ChevronRight size={14} />
              {/if}
            </button>
          {:else}
            <span class="json-expand-placeholder"></span>
          {/if}

          <span class="json-index">{index}</span>
          <span class="json-colon">:</span>

          {#if !isExpandable}
            {@render JsonNode({ data: item, path: itemPath, searchTerm })}
          {:else if !itemExpanded}
            <span class="json-collapsed">
              {valueType === 'array' ? `[${item.length}]` : '{...}'}
            </span>
          {/if}

          <button
            class="json-copy-inline-btn"
            onclick={() => copyToClipboard(item)}
            title="Copy value"
          >
            <Copy size={12} />
          </button>
        </div>

        {#if isExpandable && itemExpanded}
          <div class="json-nested">
            {@render JsonNode({ data: item, path: itemPath, searchTerm })}
          </div>
        {/if}
      </div>
    {/each}
  {/if}
{/snippet}

<!-- Primitive Value Renderer -->
{#snippet JsonPrimitive(props: { value: any, type: string })}
  {@const { value, type } = props}
  {#if type === 'string'}
    <span class="json-string">"{value}"</span>
  {:else if type === 'number'}
    <span class="json-number">{value}</span>
  {:else if type === 'boolean'}
    <span class="json-boolean">{value}</span>
  {:else if type === 'null'}
    <span class="json-null">null</span>
  {:else}
    <span class="json-undefined">undefined</span>
  {/if}
{/snippet}

<style>
  .json-viewer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--mcp-space-3) var(--mcp-space-4);
    border-bottom: 1px solid var(--mcp-border-primary);
    background: var(--mcp-surface-secondary);
  }

  .json-viewer-title {
    margin: 0;
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
  }

  .json-viewer-actions {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
  }

  .json-search {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-1-5) var(--mcp-space-3);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
  }

  .json-search-input {
    border: none;
    background: transparent;
    outline: none;
    color: var(--mcp-text-primary);
    font-size: var(--mcp-text-sm);
    width: 200px;
  }

  .json-copy-btn {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-1-5) var(--mcp-space-3);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-secondary);
    font-size: var(--mcp-text-sm);
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
  }

  .json-copy-btn:hover {
    background: var(--mcp-surface-tertiary);
    color: var(--mcp-text-primary);
  }

  .copy-feedback {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-success-600);
  }

  .json-viewer {
    padding: var(--mcp-space-4);
    background: var(--mcp-surface-primary);
    overflow: auto;
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-sm);
    line-height: 1.6;
  }

  .json-viewer-embedded {
    padding: 0;
    background: transparent;
    height: 100%;
  }

  .json-line {
    margin-bottom: var(--mcp-space-1);
  }

  .json-key-row {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: 2px 0;
  }

  .json-key-row:hover {
    background: var(--mcp-surface-secondary);
  }

  .json-key-row:hover .json-copy-inline-btn {
    opacity: 1;
  }

  .json-expand-btn {
    padding: 0;
    background: none;
    border: none;
    color: var(--mcp-text-tertiary);
    cursor: pointer;
    display: flex;
    align-items: center;
    transition: color var(--mcp-transition-fast);
  }

  .json-expand-btn:hover {
    color: var(--mcp-text-primary);
  }

  .json-expand-placeholder {
    width: 14px;
    display: inline-block;
  }

  .json-key {
    color: var(--mcp-primary-600);
    font-weight: var(--mcp-font-medium);
  }

  .json-index {
    color: var(--mcp-text-tertiary);
    font-weight: var(--mcp-font-medium);
  }

  .json-colon {
    color: var(--mcp-text-tertiary);
  }

  .json-string {
    color: var(--mcp-success-600);
  }

  .json-number {
    color: var(--mcp-warning-600);
  }

  .json-boolean {
    color: var(--mcp-primary-600);
    font-weight: var(--mcp-font-medium);
  }

  .json-null {
    color: var(--mcp-text-tertiary);
    font-style: italic;
  }

  .json-undefined {
    color: var(--mcp-text-tertiary);
    font-style: italic;
  }

  .json-collapsed {
    color: var(--mcp-text-tertiary);
    font-style: italic;
    font-size: var(--mcp-text-xs);
  }

  .json-empty {
    color: var(--mcp-text-tertiary);
  }

  .json-nested {
    margin-left: var(--mcp-space-6);
    border-left: 1px solid var(--mcp-border-primary);
    padding-left: var(--mcp-space-3);
  }

  .json-copy-inline-btn {
    opacity: 0;
    padding: var(--mcp-space-1);
    background: none;
    border: none;
    color: var(--mcp-text-tertiary);
    cursor: pointer;
    display: flex;
    align-items: center;
    transition: all var(--mcp-transition-fast);
    margin-left: auto;
  }

  .json-copy-inline-btn:hover {
    color: var(--mcp-text-primary);
    background: var(--mcp-surface-tertiary);
    border-radius: var(--mcp-radius-sm);
  }

  /* Dark mode enhancements */
  [data-theme="dark"] .json-key {
    color: var(--mcp-primary-400);
  }

  [data-theme="dark"] .json-string {
    color: var(--mcp-success-400);
  }

  [data-theme="dark"] .json-number {
    color: var(--mcp-warning-400);
  }

  [data-theme="dark"] .json-boolean {
    color: var(--mcp-primary-400);
  }
</style>
