<script lang="ts">
  import { proxyStore, type CreateProxyInput, type ServerSpec } from '$lib/stores/proxyStore';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher<{ created: { id: string } }>();

  let form = $state({
    name: '',
    description: '',
    backend_type: 'stdio',
    frontend_type: 'http',
    backend_config: {
      command: '',
      args: [] as string[],
    } as Record<string, unknown>,
  });

  let loading = $state(false);
  let error = $state<string | null>(null);
  let serverSpec = $state<ServerSpec | null>(null);
  let showIntrospection = $state(false);
  let newArg = $state('');

  function updateBackendType(type: string) {
    form.backend_type = type;
    form.backend_config = {};

    switch (type) {
      case 'stdio':
        form.backend_config = { command: '', args: [] };
        break;
      case 'http':
        form.backend_config = { url: '' };
        break;
      case 'tcp':
        form.backend_config = { host: 'localhost', port: 9000 };
        break;
      case 'websocket':
        form.backend_config = { url: '' };
        break;
    }
  }

  function addArg() {
    if (newArg.trim()) {
      const args = form.backend_config.args as string[] || [];
      args.push(newArg.trim());
      form.backend_config.args = args;
      newArg = '';
    }
  }

  function removeArg(index: number) {
    const args = form.backend_config.args as string[];
    args.splice(index, 1);
    form.backend_config.args = args;
  }

  async function handleIntrospect() {
    if (!form.name.trim()) {
      error = 'Please enter a proxy name first';
      return;
    }

    loading = true;
    error = null;
    showIntrospection = true;

    try {
      serverSpec = await proxyStore.introspectBackend(
        form.backend_type,
        form.backend_config,
        30
      );
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to introspect backend';
      serverSpec = null;
    } finally {
      loading = false;
    }
  }

  async function handleSubmit() {
    if (!form.name.trim()) {
      error = 'Please enter a proxy name';
      return;
    }

    loading = true;
    error = null;

    try {
      const input: CreateProxyInput = {
        name: form.name,
        description: form.description || undefined,
        backend_type: form.backend_type,
        backend_config: form.backend_config,
        frontend_type: form.frontend_type,
      };

      const proxyId = await proxyStore.createProxy(input);
      dispatch('created', { id: proxyId });
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to create proxy';
    } finally {
      loading = false;
    }
  }
</script>

<div class="proxy-creator">
  <div class="creator-content">
    <h2 class="creator-title">Create New Proxy</h2>

    {#if error}
      <div class="error-box">
        {error}
      </div>
    {/if}

    <form on:submit|preventDefault={handleSubmit} class="creator-form">
      <!-- Basic Information -->
      <div class="form-section">
        <h3 class="section-title">Proxy Configuration</h3>

        <div class="form-group">
          <label for="name">Proxy Name *</label>
          <input
            id="name"
            type="text"
            placeholder="e.g., My Python Server"
            bind:value={form.name}
            required
          />
          <p class="form-hint">A descriptive name for your proxy</p>
        </div>

        <div class="form-group">
          <label for="description">Description</label>
          <textarea
            id="description"
            placeholder="Optional description of this proxy"
            bind:value={form.description}
            rows="2"
          />
        </div>
      </div>

      <!-- Backend Configuration -->
      <div class="form-section">
        <h3 class="section-title">Backend Server</h3>

        <div class="form-group">
          <label>Backend Type *</label>
          <div class="radio-group">
            {#each ['stdio', 'http', 'tcp', 'websocket'] as type}
              <label class="radio-label">
                <input
                  type="radio"
                  name="backend_type"
                  value={type}
                  checked={form.backend_type === type}
                  on:change={() => updateBackendType(type)}
                />
                <span class="radio-text">{type.toUpperCase()}</span>
              </label>
            {/each}
          </div>
        </div>

        <!-- STDIO Backend -->
        {#if form.backend_type === 'stdio'}
          <div class="form-group">
            <label for="command">Command *</label>
            <input
              id="command"
              type="text"
              placeholder="e.g., python"
              bind:value={form.backend_config.command}
              required
            />
            <p class="form-hint">Command to execute (e.g., python, node, deno)</p>
          </div>

          <div class="form-group">
            <label>Arguments</label>
            <div class="arg-input">
              <input
                type="text"
                placeholder="e.g., server.py"
                bind:value={newArg}
                on:keydown={(e) => e.key === 'Enter' && (e.preventDefault(), addArg())}
              />
              <button type="button" on:click={addArg} class="btn-small">Add</button>
            </div>

            {#if Array.isArray(form.backend_config.args) && form.backend_config.args.length > 0}
              <div class="args-list">
                {#each form.backend_config.args as arg, i}
                  <div class="arg-item">
                    <span>{arg}</span>
                    <button
                      type="button"
                      on:click={() => removeArg(i)}
                      class="btn-remove"
                    >
                      ✕
                    </button>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        <!-- HTTP Backend -->
        {#if form.backend_type === 'http'}
          <div class="form-group">
            <label for="http-url">Server URL *</label>
            <input
              id="http-url"
              type="url"
              placeholder="https://api.example.com"
              bind:value={form.backend_config.url}
              required
            />
          </div>
        {/if}

        <!-- TCP Backend -->
        {#if form.backend_type === 'tcp'}
          <div class="form-row">
            <div class="form-group">
              <label for="tcp-host">Host *</label>
              <input
                id="tcp-host"
                type="text"
                placeholder="localhost"
                bind:value={form.backend_config.host}
                required
              />
            </div>
            <div class="form-group">
              <label for="tcp-port">Port *</label>
              <input
                id="tcp-port"
                type="number"
                placeholder="9000"
                bind:value={form.backend_config.port}
                required
              />
            </div>
          </div>
        {/if}

        <!-- WebSocket Backend -->
        {#if form.backend_type === 'websocket'}
          <div class="form-group">
            <label for="ws-url">WebSocket URL *</label>
            <input
              id="ws-url"
              type="url"
              placeholder="ws://localhost:8000"
              bind:value={form.backend_config.url}
              required
            />
          </div>
        {/if}
      </div>

      <!-- Frontend Configuration -->
      <div class="form-section">
        <h3 class="section-title">Frontend Exposure</h3>

        <div class="form-group">
          <label>Frontend Type *</label>
          <div class="radio-group">
            {#each ['http', 'websocket', 'tcp'] as type}
              <label class="radio-label">
                <input
                  type="radio"
                  name="frontend_type"
                  value={type}
                  bind:group={form.frontend_type}
                />
                <span class="radio-text">{type.toUpperCase()}</span>
              </label>
            {/each}
          </div>
          <p class="form-hint">How clients will connect to this proxy</p>
        </div>
      </div>

      <!-- Form Actions -->
      <div class="form-actions">
        <button
          type="button"
          on:click={handleIntrospect}
          disabled={loading}
          class="btn-secondary"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
            />
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
            />
          </svg>
          Introspect Backend
        </button>

        <button
          type="submit"
          disabled={loading || !form.name}
          class="btn-primary"
        >
          {#if loading}
            <svg class="spinner" viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2" />
            </svg>
          {:else}
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 4v16m8-8H4"
              />
            </svg>
          {/if}
          Create Proxy
        </button>
      </div>
    </form>

    <!-- Introspection Results -->
    {#if showIntrospection && serverSpec}
      <div class="introspection-results">
        <h3 class="section-title">Server Capabilities</h3>

        {#if serverSpec.tools.length > 0}
          <div class="capability-section">
            <h4 class="capability-title">Tools ({serverSpec.tools.length})</h4>
            <div class="tool-list">
              {#each serverSpec.tools.slice(0, 5) as tool}
                <div class="tool-item">
                  <span class="tool-name">{tool.name}</span>
                  {#if tool.description}
                    <span class="tool-desc">{tool.description}</span>
                  {/if}
                </div>
              {/each}
              {#if serverSpec.tools.length > 5}
                <div class="more-count">+{serverSpec.tools.length - 5} more</div>
              {/if}
            </div>
          </div>
        {/if}

        {#if serverSpec.resources.length > 0}
          <div class="capability-section">
            <h4 class="capability-title">Resources ({serverSpec.resources.length})</h4>
            <div class="resource-list">
              {#each serverSpec.resources.slice(0, 5) as resource}
                <div class="resource-item">{resource.uri}</div>
              {/each}
              {#if serverSpec.resources.length > 5}
                <div class="more-count">+{serverSpec.resources.length - 5} more</div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .proxy-creator {
    max-width: 600px;
  }

  .creator-content {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .creator-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-text);
    margin: 0;
  }

  .error-box {
    padding: 1rem;
    background: #fee;
    border: 1px solid #fcc;
    border-radius: 0.375rem;
    color: #c00;
    font-size: 0.875rem;
  }

  .creator-form {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .form-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    border: 1px solid var(--color-border);
    border-radius: 0.5rem;
    background: var(--color-bg-secondary);
  }

  .section-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text);
    margin: 0 0 0.5rem 0;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .form-group label {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-text);
  }

  .form-group input,
  .form-group textarea {
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: var(--color-bg);
    color: var(--color-text);
    font-size: 0.875rem;
    font-family: inherit;
    transition: border-color 0.2s;
  }

  .form-group input:focus,
  .form-group textarea:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.1);
  }

  .form-hint {
    font-size: 0.75rem;
    color: var(--color-text-secondary);
    margin: 0;
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .radio-group {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    font-size: 0.875rem;
  }

  .radio-label input[type='radio'] {
    cursor: pointer;
  }

  .radio-text {
    color: var(--color-text);
  }

  .arg-input {
    display: flex;
    gap: 0.5rem;
  }

  .arg-input input {
    flex: 1;
  }

  .btn-small {
    padding: 0.5rem 1rem;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.875rem;
    transition: opacity 0.2s;
  }

  .btn-small:hover {
    opacity: 0.9;
  }

  .args-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .arg-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.75rem;
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: 0.25rem;
    font-size: 0.875rem;
    font-family: monospace;
  }

  .btn-remove {
    padding: 0;
    width: 20px;
    height: 20px;
    border: none;
    background: transparent;
    color: var(--color-error);
    cursor: pointer;
    font-size: 0.875rem;
    transition: color 0.2s;
  }

  .btn-remove:hover {
    color: var(--color-error-dark, #c00);
  }

  .form-actions {
    display: flex;
    gap: 1rem;
    padding: 1rem;
    border-top: 1px solid var(--color-border);
    background: var(--color-bg-secondary);
    border-radius: 0.5rem;
  }

  .btn-primary,
  .btn-secondary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.875rem;
    font-weight: 500;
    transition: all 0.2s;
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
    flex: 1;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-secondary {
    background: transparent;
    border: 1px solid var(--color-border);
    color: var(--color-text);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--color-bg);
  }

  .btn-primary:disabled,
  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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

  .introspection-results {
    padding: 1rem;
    border: 1px solid var(--color-border);
    border-radius: 0.5rem;
    background: var(--color-bg-secondary);
  }

  .capability-section {
    margin-top: 1rem;
  }

  .capability-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--color-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 0 0 0.75rem 0;
  }

  .tool-list,
  .resource-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .tool-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.5rem;
    background: var(--color-bg);
    border-radius: 0.25rem;
    font-size: 0.875rem;
  }

  .tool-name {
    font-family: monospace;
    font-weight: 500;
    color: var(--color-primary);
  }

  .tool-desc {
    font-size: 0.8rem;
    color: var(--color-text-secondary);
  }

  .resource-item {
    padding: 0.5rem;
    background: var(--color-bg);
    border-radius: 0.25rem;
    font-size: 0.875rem;
    font-family: monospace;
    color: var(--color-text-secondary);
    word-break: break-all;
  }

  .more-count {
    padding: 0.5rem;
    font-size: 0.8rem;
    color: var(--color-text-secondary);
    font-style: italic;
  }
</style>
