<script lang="ts">
  import { proxies, proxyStore } from '$lib/stores/proxyStore';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher<{ select: { id: string } }>();

  let loading = $state<Record<string, boolean>>({});

  async function handleStart(proxyId: string, e: Event) {
    e.stopPropagation();
    loading[proxyId] = true;
    try {
      await proxyStore.startProxy(proxyId);
    } finally {
      loading[proxyId] = false;
    }
  }

  async function handleStop(proxyId: string, e: Event) {
    e.stopPropagation();
    loading[proxyId] = true;
    try {
      await proxyStore.stopProxy(proxyId);
    } finally {
      loading[proxyId] = false;
    }
  }

  async function handleDelete(proxyId: string, e: Event) {
    e.stopPropagation();
    if (!confirm('Are you sure you want to delete this proxy?')) {
      return;
    }
    try {
      await proxyStore.deleteProxy(proxyId);
    } catch (err) {
      console.error('Failed to delete proxy:', err);
    }
  }

  function handleSelect(proxyId: string) {
    dispatch('select', { id: proxyId });
  }
</script>

<div class="proxy-list">
  {#if $proxies.length === 0}
    <div class="empty-state">
      <svg class="empty-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
        />
      </svg>
      <h3 class="empty-title">No proxies yet</h3>
      <p class="empty-description">Create your first proxy to expose MCP servers across transports.</p>
    </div>
  {:else}
    <div class="proxies-grid">
      {#each $proxies as proxy (proxy.id.value)}
        <div
          class="proxy-card"
          role="button"
          tabindex="0"
          on:click={() => handleSelect(proxy.id.value)}
          on:keydown={(e) => e.key === 'Enter' && handleSelect(proxy.id.value)}
        >
          <div class="card-header">
            <div class="flex items-center gap-2 flex-1">
              <div
                class="status-indicator"
                class:running={proxy.running}
                title={proxy.running ? 'Running' : 'Stopped'}
              />
              <h3 class="card-title">{proxy.name}</h3>
            </div>

            <div class="card-actions">
              {#if proxy.running}
                <button
                  class="action-btn stop"
                  on:click={(e) => handleStop(proxy.id.value, e)}
                  disabled={loading[proxy.id.value]}
                  title="Stop proxy"
                >
                  {#if loading[proxy.id.value]}
                    <svg class="spinner" viewBox="0 0 24 24">
                      <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2" />
                    </svg>
                  {:else}
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 4h12v12H6z" />
                    </svg>
                  {/if}
                </button>
              {:else}
                <button
                  class="action-btn start"
                  on:click={(e) => handleStart(proxy.id.value, e)}
                  disabled={loading[proxy.id.value]}
                  title="Start proxy"
                >
                  {#if loading[proxy.id.value]}
                    <svg class="spinner" viewBox="0 0 24 24">
                      <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2" />
                    </svg>
                  {:else}
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
                      />
                    </svg>
                  {/if}
                </button>
              {/if}

              <button
                class="action-btn delete"
                on:click={(e) => handleDelete(proxy.id.value, e)}
                title="Delete proxy"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                  />
                </svg>
              </button>
            </div>
          </div>

          {#if proxy.description}
            <p class="card-description">{proxy.description}</p>
          {/if}

          <div class="card-details">
            <div class="detail-item">
              <span class="detail-label">Frontend:</span>
              <span class="detail-value">{proxy.frontend_type}</span>
            </div>

            {#if proxy.frontend_url}
              <div class="detail-item">
                <span class="detail-label">URL:</span>
                <span class="detail-value code">{proxy.frontend_url}</span>
              </div>
            {/if}

            {#if proxy.uptime_seconds !== undefined && proxy.running}
              <div class="detail-item">
                <span class="detail-label">Uptime:</span>
                <span class="detail-value">{Math.floor(proxy.uptime_seconds / 60)}m</span>
              </div>
            {/if}

            {#if proxy.total_requests !== undefined && proxy.running}
              <div class="detail-item">
                <span class="detail-label">Requests:</span>
                <span class="detail-value">{proxy.total_requests}</span>
              </div>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .proxy-list {
    display: flex;
    flex-direction: column;
    gap: 0;
    height: 100%;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    flex: 1;
    text-align: center;
    color: var(--color-text-secondary);
  }

  .empty-icon {
    width: 64px;
    height: 64px;
    opacity: 0.5;
  }

  .empty-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--color-text);
  }

  .empty-description {
    max-width: 300px;
  }

  .proxies-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
    gap: 1rem;
  }

  .proxy-card {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem;
    border: 1px solid var(--color-border);
    border-radius: 0.5rem;
    background: var(--color-bg-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .proxy-card:hover {
    border-color: var(--color-primary);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .status-indicator {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--color-error);
    transition: background 0.2s;
  }

  .status-indicator.running {
    background: var(--color-success);
    box-shadow: 0 0 8px var(--color-success);
  }

  .card-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text);
    margin: 0;
  }

  .card-actions {
    display: flex;
    gap: 0.5rem;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: transparent;
    cursor: pointer;
    transition: all 0.2s;
  }

  .action-btn:hover:not(:disabled) {
    border-color: var(--color-primary);
    background: var(--color-primary);
    color: white;
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-btn.start {
    color: var(--color-success);
  }

  .action-btn.stop {
    color: var(--color-warning);
  }

  .action-btn.delete {
    color: var(--color-error);
  }

  .action-btn:hover.start,
  .action-btn:hover.stop,
  .action-btn:hover.delete {
    color: white;
  }

  .spinner {
    width: 16px;
    height: 16px;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .card-description {
    font-size: 0.875rem;
    color: var(--color-text-secondary);
    margin: 0;
  }

  .card-details {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--color-border);
  }

  .detail-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .detail-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--color-text-secondary);
    letter-spacing: 0.5px;
  }

  .detail-value {
    font-size: 0.875rem;
    color: var(--color-text);
    font-weight: 500;
  }

  .detail-value.code {
    font-family: monospace;
    font-size: 0.8rem;
    word-break: break-all;
  }
</style>
