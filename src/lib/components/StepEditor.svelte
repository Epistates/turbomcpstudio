<!--
  Workflow Step Editor Component
  Comprehensive editor for configuring MCP workflow steps
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { createLogger } from '$lib/utils/logger';

  import { invoke } from '@tauri-apps/api/core';

  import type { WorkflowStep, McpOperation, VariableExtract, Assertion } from '$lib/types/collections';

  import type { ServerInfo, ToolDefinition } from '$lib/stores/serverStore';

  import { serverStore } from '$lib/stores/serverStore';

  import {
    Save,
    X,
    Plus,
    Trash2,
    Zap,
    Database,
    MessageSquare,
    FileText,
    Activity,
    Settings,
    CheckCircle,
    AlertCircle
  } from 'lucide-svelte';

  // Initialize scoped logger
  const logger = createLogger('StepEditor');

  // Props
  const {
    step,
    servers,
    onSave,
    onCancel
  } = $props<{
    step: WorkflowStep;
    servers: ServerInfo[];
    onSave: (step: WorkflowStep) => void;
    onCancel: () => void;
  }>();

  // Local state for editing
  let editedStep = $state<WorkflowStep>(JSON.parse(JSON.stringify(step)));
  let availableTools = $state<ToolDefinition[]>([]);
  let availablePrompts = $state<any[]>([]);
  let loadingTools = $state(false);
  let loadingPrompts = $state(false);

  // Get available servers based on operation type
  const availableServers = $derived(() => {
    // Filter to only connected servers
    return servers.filter((s: any) => s.status?.toLowerCase() === 'connected');
  });

  // Load tools when tool operation server changes
  $effect(() => {
    if (editedStep.operation.type === 'tool') {
      const operation = editedStep.operation as any;
      if (operation.server_alias) {
        loadTools(operation.server_alias);
      } else {
        availableTools = [];
      }
    }
  });

  // Load prompts when prompt operation server changes
  $effect(() => {
    if (editedStep.operation.type === 'prompt') {
      const operation = editedStep.operation as any;
      if (operation.server_alias) {
        loadPrompts(operation.server_alias);
      } else {
        availablePrompts = [];
      }
    }
  });

  async function loadTools(serverId: string) {
    loadingTools = true;
    try {
      availableTools = await serverStore.listTools(serverId);
    } catch (error) {
      logger.error('Failed to load tools:', error);
      availableTools = [];
    } finally {
      loadingTools = false;
    }
  }

  async function loadPrompts(serverId: string) {
    loadingPrompts = true;
    try {
      availablePrompts = await invoke<any[]>('list_prompts', { serverId });
    } catch (error) {
      logger.error('Failed to load prompts:', error);
      availablePrompts = [];
    } finally {
      loadingPrompts = false;
    }
  }

  function addVariableExtract() {
    editedStep.variable_extracts.push({
      source: 'response',
      path: '',
      variable_name: ''
    });
    editedStep = { ...editedStep };
  }

  function removeVariableExtract(index: number) {
    editedStep.variable_extracts.splice(index, 1);
    editedStep = { ...editedStep };
  }

  function addAssertion() {
    editedStep.assertions.push({
      id: crypto.randomUUID(),
      name: '',
      type: 'response_status',
      condition: {
        operator: 'equals',
        expected_value: ''
      },
      severity: 'error',
      continue_on_failure: false
    });
    editedStep = { ...editedStep };
  }

  function removeAssertion(index: number) {
    editedStep.assertions.splice(index, 1);
    editedStep = { ...editedStep };
  }

  function handleSave() {
    onSave(editedStep);
  }
</script>

<div class="step-editor">
  <!-- Header -->
  <div class="step-editor__header">
    <h3 class="step-editor__title">Edit Workflow Step</h3>
    <div class="step-editor__actions">
      <button onclick={handleSave} class="btn-primary text-sm">
        <Save size={14} class="mr-1" />
        Save
      </button>
      <button onclick={onCancel} class="btn-secondary text-sm">
        <X size={14} class="mr-1" />
        Cancel
      </button>
    </div>
  </div>

  <!-- Scrollable Content -->
  <div class="step-editor__content">
    <!-- Basic Information -->
    <section class="step-editor__section">
      <h4 class="step-editor__section-title">Basic Information</h4>

      <div class="form-group">
        <label class="form-label" for="step-name">Step Name *</label>
        <input
          id="step-name"
          type="text"
          bind:value={editedStep.name}
          placeholder="e.g., Fetch User Data"
          class="form-input"
          required
        />
      </div>

      <div class="form-group">
        <label class="form-label" for="step-description">Description</label>
        <textarea
          id="step-description"
          bind:value={editedStep.description}
          placeholder="Describe what this step does..."
          class="form-input h-20 resize-none"
        ></textarea>
      </div>

      <div class="form-group-row">
        <label class="form-checkbox-label">
          <input type="checkbox" bind:checked={editedStep.enabled} class="form-checkbox" />
          <span>Enabled</span>
        </label>

        <label class="form-checkbox-label">
          <input type="checkbox" bind:checked={editedStep.continue_on_error} class="form-checkbox" />
          <span>Continue on Error</span>
        </label>
      </div>

      <div class="form-group">
        <label class="form-label" for="step-timeout">Timeout (ms)</label>
        <input
          id="step-timeout"
          type="number"
          bind:value={editedStep.timeout_ms}
          placeholder="30000"
          class="form-input"
          min="0"
        />
      </div>
    </section>

    <!-- Operation Configuration -->
    <section class="step-editor__section">
      <h4 class="step-editor__section-title">Operation Configuration</h4>

      {#if editedStep.operation.type === 'tool'}
        {@const operation = editedStep.operation as any}
        <div class="form-group">
          <label class="form-label" for="step-tool-server">Server</label>
          <select id="step-tool-server" bind:value={operation.server_alias} class="form-select">
            <option value="">Select a server...</option>
            {#each availableServers() as server}
              <option value={server.id}>{server.config.name}</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label class="form-label" for="step-tool-name">Tool</label>
          <select id="step-tool-name" bind:value={operation.tool_name} class="form-select" disabled={!operation.server_alias || loadingTools}>
            <option value="">{loadingTools ? 'Loading tools...' : 'Select a tool...'}</option>
            {#each availableTools as tool}
              <option value={tool.name}>{tool.name}</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label class="form-label" for="step-tool-params">Parameters (JSON)</label>
          <textarea
            id="step-tool-params"
            bind:value={operation.parameters}
            placeholder="&#123;&quot;param&quot;: &quot;value&quot;, &quot;user_id&quot;: &quot;$&#123;user_id&#125;&quot;&#125;"
            class="form-input font-mono text-sm h-32"
          ></textarea>
          <p class="form-help">Use $&#123;variable&#125; syntax for variable interpolation</p>
        </div>

      {:else if editedStep.operation.type === 'resource'}
        {@const operation = editedStep.operation as any}
        <div class="form-group">
          <label class="form-label" for="step-resource-server">Server</label>
          <select id="step-resource-server" bind:value={operation.server_alias} class="form-select">
            <option value="">Select a server...</option>
            {#each availableServers() as server}
              <option value={server.id}>{server.config.name}</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label class="form-label" for="step-resource-uri">Resource URI</label>
          <input
            id="step-resource-uri"
            type="text"
            bind:value={operation.resource_uri}
            placeholder="file:///path/to/resource or $&#123;resource_path&#125;"
            class="form-input font-mono"
          />
          <p class="form-help">Use $&#123;variable&#125; syntax for dynamic URIs</p>
        </div>

      {:else if editedStep.operation.type === 'prompt'}
        {@const operation = editedStep.operation as any}
        <div class="form-group">
          <label class="form-label" for="step-prompt-server">Server</label>
          <select id="step-prompt-server" bind:value={operation.server_alias} class="form-select">
            <option value="">Select a server...</option>
            {#each availableServers() as server}
              <option value={server.id}>{server.config.name}</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label class="form-label" for="step-prompt-name">Prompt</label>
          <select id="step-prompt-name" bind:value={operation.prompt_name} class="form-select" disabled={!operation.server_alias || loadingPrompts}>
            <option value="">{loadingPrompts ? 'Loading prompts...' : 'Select a prompt...'}</option>
            {#each availablePrompts as prompt}
              <option value={prompt.name}>{prompt.name}</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label class="form-label" for="step-prompt-args">Arguments (JSON)</label>
          <textarea
            id="step-prompt-args"
            bind:value={operation.parameters}
            placeholder="&#123;&quot;arg1&quot;: &quot;value&quot;, &quot;context&quot;: &quot;$&#123;context&#125;&quot;&#125;"
            class="form-input font-mono text-sm h-32"
          ></textarea>
          <p class="form-help">Use $&#123;variable&#125; syntax for variable interpolation</p>
        </div>

      {:else if editedStep.operation.type === 'sampling'}
        {@const operation = editedStep.operation as any}
        <div class="form-group">
          <label class="form-label" for="step-sampling-server">Server</label>
          <select id="step-sampling-server" bind:value={operation.server_alias} class="form-select">
            <option value="">Select a server...</option>
            {#each availableServers() as server}
              <option value={server.id}>{server.config.name}</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label class="form-label" for="step-sampling-messages">Messages (JSON)</label>
          <textarea
            id="step-sampling-messages"
            bind:value={operation.messages}
            placeholder="[&#123;&quot;role&quot;: &quot;user&quot;, &quot;content&quot;: &quot;Analyze $&#123;data&#125;&quot;&#125;]"
            class="form-input font-mono text-sm h-32"
          ></textarea>
        </div>

        <div class="form-group-row">
          <div class="form-group">
            <label class="form-label" for="step-sampling-max-tokens">Max Tokens</label>
            <input
              id="step-sampling-max-tokens"
              type="number"
              bind:value={operation.max_tokens}
              class="form-input"
              min="1"
            />
          </div>

          <div class="form-group">
            <label class="form-label" for="step-sampling-temperature">Temperature</label>
            <input
              id="step-sampling-temperature"
              type="number"
              bind:value={operation.temperature}
              class="form-input"
              min="0"
              max="2"
              step="0.1"
            />
          </div>
        </div>

        <label class="form-checkbox-label">
          <input type="checkbox" bind:checked={operation.auto_approve} class="form-checkbox" />
          <span>Auto-approve (for automated testing)</span>
        </label>
      {/if}
    </section>

    <!-- Variable Extraction -->
    <section class="step-editor__section">
      <div class="step-editor__section-header">
        <h4 class="step-editor__section-title">Variable Extraction</h4>
        <button onclick={addVariableExtract} class="btn-secondary text-xs">
          <Plus size={12} class="mr-1" />
          Add Variable
        </button>
      </div>

      {#if editedStep.variable_extracts.length === 0}
        <p class="form-help">No variables extracted. Extract values to use in later steps.</p>
      {:else}
        <div class="variable-extracts">
          {#each editedStep.variable_extracts as extract, index}
            <div class="variable-extract-item">
              <div class="form-group-row">
                <div class="form-group flex-1">
                  <label class="form-label text-xs" for="step-extract-var-{index}">Variable Name</label>
                  <input
                    id="step-extract-var-{index}"
                    type="text"
                    bind:value={extract.variable_name}
                    placeholder="user_id"
                    class="form-input text-sm"
                  />
                </div>

                <div class="form-group flex-1">
                  <label class="form-label text-xs" for="step-extract-path-{index}">JSON Path</label>
                  <input
                    id="step-extract-path-{index}"
                    type="text"
                    bind:value={extract.path}
                    placeholder="$.response.user.id"
                    class="form-input font-mono text-sm"
                  />
                </div>

                <button
                  onclick={() => removeVariableExtract(index)}
                  class="btn-secondary text-xs mt-auto"
                  title="Remove variable"
                >
                  <Trash2 size={12} />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <!-- Assertions -->
    <section class="step-editor__section">
      <div class="step-editor__section-header">
        <h4 class="step-editor__section-title">Assertions</h4>
        <button onclick={addAssertion} class="btn-secondary text-xs">
          <Plus size={12} class="mr-1" />
          Add Assertion
        </button>
      </div>

      {#if editedStep.assertions.length === 0}
        <p class="form-help">No assertions configured. Add assertions to validate step results.</p>
      {:else}
        <div class="assertions">
          {#each editedStep.assertions as assertion, index}
            <div class="assertion-item">
              <div class="form-group">
                <label class="form-label text-xs" for="step-assertion-name-{index}">Assertion Name</label>
                <input
                  id="step-assertion-name-{index}"
                  type="text"
                  bind:value={assertion.name}
                  placeholder="Status code should be 200"
                  class="form-input text-sm"
                />
              </div>

              <div class="form-group-row">
                <div class="form-group flex-1">
                  <label class="form-label text-xs" for="step-assertion-type-{index}">Type</label>
                  <select id="step-assertion-type-{index}" bind:value={assertion.type} class="form-select text-sm">
                    <option value="response_status">Response Status</option>
                    <option value="response_contains">Response Contains</option>
                    <option value="response_equals">Response Equals</option>
                    <option value="response_json_path">JSON Path</option>
                    <option value="timing">Timing</option>
                  </select>
                </div>

                <div class="form-group flex-1">
                  <label class="form-label text-xs" for="step-assertion-operator-{index}">Operator</label>
                  <select id="step-assertion-operator-{index}" bind:value={assertion.condition.operator} class="form-select text-sm">
                    <option value="equals">Equals</option>
                    <option value="not_equals">Not Equals</option>
                    <option value="contains">Contains</option>
                    <option value="not_contains">Not Contains</option>
                    <option value="greater_than">Greater Than</option>
                    <option value="less_than">Less Than</option>
                  </select>
                </div>

                <button
                  onclick={() => removeAssertion(index)}
                  class="btn-secondary text-xs mt-auto"
                  title="Remove assertion"
                >
                  <Trash2 size={12} />
                </button>
              </div>

              <div class="form-group">
                <label class="form-label text-xs" for="step-assertion-value-{index}">Expected Value</label>
                <input
                  id="step-assertion-value-{index}"
                  type="text"
                  bind:value={assertion.condition.expected_value}
                  placeholder="200"
                  class="form-input text-sm"
                />
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .step-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--mcp-surface-primary);
  }

  .step-editor__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--mcp-space-4);
    border-bottom: 1px solid var(--mcp-border-primary);
    background: var(--mcp-surface-secondary);
  }

  .step-editor__title {
    font-size: var(--mcp-text-lg);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    margin: 0;
  }

  .step-editor__actions {
    display: flex;
    gap: var(--mcp-space-2);
  }

  .step-editor__content {
    flex: 1;
    overflow-y: auto;
    padding: var(--mcp-space-4);
  }

  .step-editor__section {
    margin-bottom: var(--mcp-space-6);
    padding: var(--mcp-space-4);
    background: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
  }

  .step-editor__section-title {
    font-size: var(--mcp-text-base);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    margin: 0 0 var(--mcp-space-3) 0;
  }

  .step-editor__section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--mcp-space-3);
  }

  .form-group {
    margin-bottom: var(--mcp-space-3);
  }

  .form-group:last-child {
    margin-bottom: 0;
  }

  .form-group-row {
    display: flex;
    gap: var(--mcp-space-3);
    margin-bottom: var(--mcp-space-3);
  }

  .form-label {
    display: block;
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-secondary);
    margin-bottom: var(--mcp-space-1-5);
  }

  .form-input,
  .form-select {
    width: 100%;
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-primary);
    font-size: var(--mcp-text-sm);
    transition: all var(--mcp-transition-fast);
  }

  .form-input:focus,
  .form-select:focus {
    outline: none;
    border-color: var(--mcp-primary-500);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .form-checkbox {
    width: 16px;
    height: 16px;
    margin-right: var(--mcp-space-2);
  }

  .form-checkbox-label {
    display: flex;
    align-items: center;
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
    cursor: pointer;
  }

  .form-help {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    margin-top: var(--mcp-space-1);
  }

  .variable-extracts,
  .assertions {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-3);
  }

  .variable-extract-item,
  .assertion-item {
    padding: var(--mcp-space-3);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
  }

  /* Scrollbar */
  .step-editor__content::-webkit-scrollbar {
    width: 6px;
  }

  .step-editor__content::-webkit-scrollbar-track {
    background: transparent;
  }

  .step-editor__content::-webkit-scrollbar-thumb {
    background: var(--mcp-border-primary);
    border-radius: 3px;
  }

  .step-editor__content::-webkit-scrollbar-thumb:hover {
    background: var(--mcp-border-secondary);
  }

  /* Dark theme adjustments */
  [data-theme="dark"] .form-input:focus,
  [data-theme="dark"] .form-select:focus {
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.2);
  }
</style>
