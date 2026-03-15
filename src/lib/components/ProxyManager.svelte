<script lang="ts">
  import { onMount } from 'svelte';
  import { proxyStore, proxies, activeProxyId, runningProxies, proxyError } from '$lib/stores/proxyStore';
  import ProxyList from './proxy/ProxyList.svelte';
  import ProxyCreator from './proxy/ProxyCreator.svelte';
  import ProxyMonitor from './proxy/ProxyMonitor.svelte';

  let view = $state<'list' | 'create' | 'monitor'>('list');
  let selectedProxyId = $state<string | null>(null);
  let showError = $state(false);

  onMount(async () => {
    await proxyStore.loadProxies();
  });

  function handleSelectProxy(detail: { id: string }) {
    selectedProxyId = detail.id;
    view = 'monitor';
  }

  function handleProxyCreated(detail: { id: string }) {
    selectedProxyId = detail.id;
    view = 'monitor';
  }

  function handleBackToList() {
    view = 'list';
    selectedProxyId = null;
  }

  $effect(() => {
    if ($proxyError) {
      showError = true;
      setTimeout(() => {
        showError = false;
        proxyStore.clearError();
      }, 5000);
    }
  });
</script>

<div class="proxy-manager">
  <header class="proxy-header">
    <div class="flex items-center justify-between gap-4">
      <div class="flex items-center gap-3">
        <h1 class="text-2xl font-bold">MCP Proxies</h1>
        <span class="text-sm text-gray-500">
          {$proxies.length} configured • {$runningProxies.length} running
        </span>
      </div>

      <nav class="flex gap-2">
        <button
          type="button"
          class="btn"
          class:active={view === 'list'}
          onclick={() => handleBackToList()}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 6h16M4 12h16M4 18h16"
            />
          </svg>
          <span>All Proxies</span>
        </button>
        <button
          type="button"
          class="btn btn-primary"
          class:active={view === 'create'}
          onclick={() => (view = 'create')}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 4v16m8-8H4"
            />
          </svg>
          <span>New Proxy</span>
        </button>
      </nav>
    </div>
  </header>

  {#if showError && $proxyError}
    <div class="error-banner">
      <div class="flex items-start gap-3">
        <svg class="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
          <path
            fill-rule="evenodd"
            d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
            clip-rule="evenodd"
          />
        </svg>
        <div>
          <p class="font-medium text-red-800">{$proxyError}</p>
        </div>
      </div>
    </div>
  {/if}

  <main class="proxy-content">
    {#if view === 'list'}
      <ProxyList onselect={handleSelectProxy} />
    {:else if view === 'create'}
      <ProxyCreator oncreated={handleProxyCreated} />
    {:else if view === 'monitor' && selectedProxyId}
      <ProxyMonitor {selectedProxyId} onclose={handleBackToList} />
    {/if}
  </main>
</div>

<style>
  .proxy-manager {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 0;
    background: var(--color-bg);
  }

  .proxy-header {
    padding: 1.5rem;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-secondary);
  }

  .error-banner {
    padding: 1rem 1.5rem;
    background: #fee;
    border-bottom: 1px solid var(--color-border);
  }

  .proxy-content {
    flex: 1;
    overflow: auto;
    padding: 1.5rem;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: 0.375rem;
    border: 1px solid var(--color-border);
    background: transparent;
    cursor: pointer;
    font-size: 0.875rem;
    font-weight: 500;
    transition: all 0.2s;
  }

  .btn:hover {
    background: var(--color-bg-hover);
  }

  .btn.active {
    background: var(--color-primary);
    color: white;
    border-color: var(--color-primary);
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
    border-color: var(--color-primary);
  }

  .btn-primary:hover {
    opacity: 0.9;
  }
</style>
