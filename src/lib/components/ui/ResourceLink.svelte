<script lang="ts">
  import { createLogger } from '$lib/utils/logger';
  import { ExternalLink, FileText, Image, File, Loader2, AlertCircle } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';

  // Initialize scoped logger
  const logger = createLogger('ResourceLink');

  /**
   * ResourceLink Component - June 2025 MCP Spec
   *
   * Displays resource_link content type with click-to-load functionality.
   * Resource links are ephemeral - not in resources/list, returned in tool results.
   */

  interface ResourceLinkData {
    uri: string;
    name?: string;
    description?: string;
    mimeType?: string;
  }

  interface ResourceContent {
    uri: string;
    mimeType?: string;
    text?: string;
    blob?: string;
  }

  // Props
  const {
    uri,
    name,
    description,
    mimeType,
    serverId
  }: {
    uri: string;
    name?: string;
    description?: string;
    mimeType?: string;
    serverId?: string;
  } = $props();

  // State
  let loading = $state(false);
  let error = $state<string | null>(null);
  let loaded = $state(false);
  let content = $state<ResourceContent | null>(null);

  // Derived
  const displayName = $derived(name || uri.split('/').pop() || uri);
  const resourceType = $derived(() => {
    if (mimeType?.startsWith('image/')) return 'image';
    if (mimeType?.startsWith('text/')) return 'text';
    if (uri.match(/\.(png|jpg|jpeg|gif|svg|webp)$/i)) return 'image';
    if (uri.match(/\.(txt|md|json|xml|html|css|js|ts)$/i)) return 'text';
    return 'file';
  });

  function getIcon() {
    const type = resourceType();
    switch (type) {
      case 'image': return Image;
      case 'text': return FileText;
      default: return File;
    }
  }

  async function loadResource() {
    if (loaded || loading) return;

    loading = true;
    error = null;

    try {
      // Try to load the resource via MCP read_resource command
      if (serverId) {
        const result = await invoke<ResourceContent>('read_resource', {
          serverId,
          uri
        });
        content = result;
        loaded = true;
      } else {
        // Fallback: if it's a web URL, we could fetch it
        // For now, just show error
        error = 'Server ID required to load resource';
      }
    } catch (e) {
      error = String(e);
      logger.error('Failed to load resource:', e);
    } finally {
      loading = false;
    }
  }

  function openExternal() {
    // For web URLs, open in browser
    if (uri.startsWith('http://') || uri.startsWith('https://')) {
      invoke('open_url', { url: uri }).catch(e => logger.error('Failed to open URL', e));
    }
  }
</script>

<div class="resource-link">
  {#if !loaded}
    <!-- Collapsed View - Click to Load -->
    <button
      onclick={loadResource}
      disabled={loading}
      class="resource-link__trigger"
    >
      <div class="resource-link__icon">
        {#if loading}
          <Loader2 size={16} class="animate-spin text-blue-600 dark:text-blue-400" />
        {:else}
          {@const Icon = getIcon()}
          <Icon size={16} class="text-blue-600 dark:text-blue-400" />
        {/if}
      </div>

      <div class="resource-link__info">
        <div class="resource-link__name">
          {displayName}
        </div>
        {#if description}
          <div class="resource-link__description">
            {description}
          </div>
        {/if}
        <div class="resource-link__uri">
          {uri}
        </div>
      </div>

      <div class="resource-link__action">
        {#if loading}
          <span class="text-xs text-blue-600 dark:text-blue-400">Loading...</span>
        {:else}
          <ExternalLink size={14} class="text-blue-600 dark:text-blue-400" />
        {/if}
      </div>
    </button>

    {#if error}
      <div class="resource-link__error">
        <AlertCircle size={14} />
        <span>{error}</span>
      </div>
    {/if}
  {:else if content}
    {@const Icon = getIcon()}
    <!-- Expanded View - Show Content -->
    <div class="resource-link__loaded">
      <div class="resource-link__header">
        <div class="flex items-center gap-2">
          <Icon size={16} class="text-blue-600 dark:text-blue-400" />
          <span class="text-sm font-semibold text-blue-900 dark:text-blue-100">
            {displayName}
          </span>
        </div>
        <button
          onclick={() => loaded = false}
          class="text-xs text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200"
        >
          Collapse
        </button>
      </div>

      {#if description}
        <div class="text-xs text-blue-700 dark:text-blue-300 mb-2">
          {description}
        </div>
      {/if}

      <div class="resource-link__content">
        {#if resourceType() === 'image'}
          {#if content.blob}
            <img
              src={`data:${content.mimeType || 'image/png'};base64,${content.blob}`}
              alt={displayName}
              class="max-w-full rounded"
            />
          {:else}
            <p class="text-sm text-gray-500">Image data not available</p>
          {/if}
        {:else if resourceType() === 'text'}
          {#if content.text}
            <pre class="resource-link__text">{content.text}</pre>
          {:else}
            <p class="text-sm text-gray-500">Text content not available</p>
          {/if}
        {:else}
          <div class="text-sm text-gray-700 dark:text-gray-300">
            <p class="mb-2">Resource loaded successfully.</p>
            {#if content.text}
              <pre class="resource-link__text">{content.text}</pre>
            {:else if content.blob}
              <p class="text-xs text-gray-500">Binary content ({content.blob.length} bytes)</p>
            {/if}
          </div>
        {/if}
      </div>

      <div class="resource-link__footer">
        <span class="text-xs text-gray-600 dark:text-gray-400 font-mono">
          {uri}
        </span>
        {#if uri.startsWith('http://') || uri.startsWith('https://')}
          <button
            onclick={openExternal}
            class="text-xs text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1"
          >
            Open in browser
            <ExternalLink size={12} />
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .resource-link {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .resource-link__trigger {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.75rem;
    background: rgb(239 246 255 / 1); /* blue-50 */
    border: 1px solid rgb(191 219 254 / 1); /* blue-200 */
    border-radius: 0.5rem;
    text-align: left;
    transition: all 0.2s;
    cursor: pointer;
  }

  .resource-link__trigger:hover:not(:disabled) {
    background: rgb(219 234 254 / 1); /* blue-100 */
    border-color: rgb(147 197 253 / 1); /* blue-300 */
  }

  .resource-link__trigger:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  [data-theme="dark"] .resource-link__trigger {
    background: rgb(30 58 138 / 0.2); /* blue-900/20 */
    border-color: rgb(30 64 175 / 1); /* blue-800 */
  }

  [data-theme="dark"] .resource-link__trigger:hover:not(:disabled) {
    background: rgb(30 58 138 / 0.3);
    border-color: rgb(37 99 235 / 1); /* blue-600 */
  }

  .resource-link__icon {
    flex-shrink: 0;
  }

  .resource-link__info {
    flex: 1;
    min-width: 0;
  }

  .resource-link__name {
    font-size: 0.875rem;
    font-weight: 600;
    color: rgb(30 58 138 / 1); /* blue-900 */
    margin-bottom: 0.125rem;
  }

  [data-theme="dark"] .resource-link__name {
    color: rgb(191 219 254 / 1); /* blue-200 */
  }

  .resource-link__description {
    font-size: 0.75rem;
    color: rgb(29 78 216 / 1); /* blue-700 */
    margin-bottom: 0.25rem;
  }

  [data-theme="dark"] .resource-link__description {
    color: rgb(147 197 253 / 1); /* blue-300 */
  }

  .resource-link__uri {
    font-size: 0.75rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    color: rgb(107 114 128 / 1); /* gray-500 */
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  [data-theme="dark"] .resource-link__uri {
    color: rgb(156 163 175 / 1); /* gray-400 */
  }

  .resource-link__action {
    flex-shrink: 0;
  }

  .resource-link__error {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: rgb(254 242 242 / 1); /* red-50 */
    border: 1px solid rgb(254 202 202 / 1); /* red-200 */
    border-radius: 0.375rem;
    font-size: 0.75rem;
    color: rgb(153 27 27 / 1); /* red-900 */
  }

  [data-theme="dark"] .resource-link__error {
    background: rgb(127 29 29 / 0.2); /* red-900/20 */
    border-color: rgb(153 27 27 / 1); /* red-900 */
    color: rgb(254 202 202 / 1); /* red-200 */
  }

  .resource-link__loaded {
    padding: 0.75rem;
    background: rgb(239 246 255 / 1); /* blue-50 */
    border: 1px solid rgb(191 219 254 / 1); /* blue-200 */
    border-radius: 0.5rem;
  }

  [data-theme="dark"] .resource-link__loaded {
    background: rgb(30 58 138 / 0.2); /* blue-900/20 */
    border-color: rgb(30 64 175 / 1); /* blue-800 */
  }

  .resource-link__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }

  .resource-link__content {
    margin: 0.75rem 0;
    padding: 0.75rem;
    background: rgb(255 255 255 / 1);
    border: 1px solid rgb(229 231 235 / 1); /* gray-200 */
    border-radius: 0.375rem;
  }

  [data-theme="dark"] .resource-link__content {
    background: rgb(31 41 55 / 1); /* gray-800 */
    border-color: rgb(75 85 99 / 1); /* gray-600 */
  }

  .resource-link__text {
    font-size: 0.875rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    color: rgb(17 24 39 / 1); /* gray-900 */
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
  }

  [data-theme="dark"] .resource-link__text {
    color: rgb(243 244 246 / 1); /* gray-100 */
  }

  .resource-link__footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid rgb(219 234 254 / 1); /* blue-100 */
  }

  [data-theme="dark"] .resource-link__footer {
    border-color: rgb(30 64 175 / 1); /* blue-800 */
  }
</style>
