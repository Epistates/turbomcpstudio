<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import {
    X,
    ExternalLink,
    Package,
    Copy,
    Download,
    CheckCircle,
    AlertCircle,
    Info,
    Key,
    Settings,
    FileJson,
  } from 'lucide-svelte';
  import Button from './ui/Button.svelte';
  import SchemaFormGenerator from './SchemaFormGenerator.svelte';
  import { uiStore } from '$lib/stores/uiStore';
  import { serverStore } from '$lib/stores/serverStore';

  // Props
  let { server, onClose, onServerAdded }: {
    server: any;
    onClose?: () => void;
    onServerAdded?: () => void;
  } = $props();

  // Available external clients
  const externalClients = [
    { id: 'claude-desktop', name: 'Claude Desktop', description: 'Anthropic Claude Desktop app' },
    { id: 'claude-code', name: 'Claude Code', description: 'Claude CLI tool' },
    { id: 'lmstudio', name: 'LM Studio', description: 'LM Studio desktop app' },
    { id: 'cursor', name: 'Cursor', description: 'Cursor AI IDE' },
    { id: 'cline', name: 'Cline', description: 'VS Code extension' },
  ];

  // State (must use $state() in runes mode for reactivity!)
  let currentStep = $state<1 | 2 | 3>(1); // 1: Info, 2: Choose Action, 3: Configure + Execute
  let parameterValues = $state<Record<string, any>>({});
  let secretValues = $state<Record<string, string>>({});
  let validationErrors = $state<Record<string, string>>({});

  // Step 2 state - action choice (default to turbomcp)
  let actionChoice = $state<'turbomcp' | 'export' | null>('turbomcp');

  // Step 3 state - export clients
  let selectedClients = $state<Set<string>>(new Set());
  let generatedConfigs = $state<Map<string, { config: any; json: string; notes: string[] }> >(new Map());

  // Extract configuration schema
  const config = $derived(server?.config || {});
  const parameters = $derived(config?.parameters || {});
  const secrets = $derived(config?.secrets || {});

  // Handle close
  function handleClose() {
    if (onClose) {
      onClose();
    }
  }

  // Initialize default values
  $effect(() => {
    if (server && parameters?.properties) {
      Object.entries(parameters.properties).forEach(([name, prop]: [string, any]) => {
        if (prop?.default !== undefined && !(name in parameterValues)) {
          parameterValues[name] = prop.default;
        }
      });
    }
  });

  function handleParameterChange(name: string, value: any) {
    parameterValues[name] = value;
    parameterValues = { ...parameterValues };

    if (validationErrors[name]) {
      delete validationErrors[name];
      validationErrors = { ...validationErrors };
    }
  }

  function handleSecretChange(name: string, value: string) {
    secretValues[name] = value;
    secretValues = { ...secretValues };

    if (validationErrors[name]) {
      delete validationErrors[name];
      validationErrors = { ...validationErrors };
    }
  }

  function validateConfiguration(): boolean {
    validationErrors = {};

    const required = parameters.required || [];
    for (const fieldName of required) {
      if (!parameterValues[fieldName]) {
        validationErrors[fieldName] = 'This field is required';
      }
    }

    for (const secret of secrets) {
      if (secret.required && !secretValues[secret.name]) {
        validationErrors[secret.name] = 'This secret is required';
      }
    }

    validationErrors = { ...validationErrors };
    return Object.keys(validationErrors).length === 0;
  }

  function proceedToStep3() {
    if (!actionChoice) {
      uiStore.showError('Please choose an action');
      return;
    }
    currentStep = 3;
  }

  async function executeAction() {
    if (!validateConfiguration()) {
      uiStore.showError('Please fill in all required fields');
      return;
    }

    if (actionChoice === 'turbomcp') {
      await addToServerManager();
    }
    // For export, configs are already generated when clients are selected
  }

  async function generateConfigsForClients() {
    generatedConfigs.clear();

    const userConfig = {
      parameters: parameterValues,
      secrets: secretValues,
    };

    for (const clientId of selectedClients) {
      try {
        const result = await invoke('generate_client_config', {
          server,
          userConfig,
          clientType: clientId,
        }) as { config_json: string; notes?: string[] };

        generatedConfigs.set(clientId, {
          config: result,
          json: result.config_json,
          notes: result.notes || [],
        });
      } catch (error) {
        console.error(`Failed to generate config for ${clientId}:`, error);
        uiStore.showError(`Failed to generate config for ${clientId}: ${error}`);
      }
    }

    generatedConfigs = new Map(generatedConfigs);
  }

  function toggleClient(clientId: string) {
    if (selectedClients.has(clientId)) {
      selectedClients.delete(clientId);
    } else {
      selectedClients.add(clientId);
    }
    selectedClients = new Set(selectedClients);

    if (selectedClients.size > 0) {
      generateConfigsForClients();
    }
  }

  async function copyToClipboard(clientId: string) {
    const configData = generatedConfigs.get(clientId);
    if (!configData) return;

    try {
      await writeText(configData.json);
      uiStore.showSuccess(`${getClientName(clientId)} configuration copied to clipboard`);
    } catch (error) {
      console.error('Failed to copy:', error);
      uiStore.showError('Failed to copy to clipboard');
    }
  }

  async function exportToFile(clientId: string) {
    const configData = generatedConfigs.get(clientId);
    if (!configData) return;

    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        defaultPath: `${server.name}-${clientId}-config.json`,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });

      if (filePath) {
        const { writeTextFile } = await import('@tauri-apps/plugin-fs');
        await writeTextFile(filePath, configData.json);
        uiStore.showSuccess('Configuration exported successfully');
      }
    } catch (error) {
      console.error('Failed to export:', error);
      uiStore.showError('Failed to export configuration');
    }
  }

  async function addToServerManager() {
    try {
      const userConfig = {
        parameters: parameterValues,
        secrets: secretValues,
      };

      // Generate TurboMCP config
      const result = await invoke('generate_client_config', {
        server,
        userConfig,
        clientType: 'turbomcp',
      }) as { config_json: string; notes?: string[] };

      const config = JSON.parse(result.config_json);

      // Call backend to add server
      await invoke('add_server_from_registry', { config });

      // Refresh the server list to show the newly added server
      await serverStore.loadServers();

      uiStore.showSuccess(`${server.name} added to Server Manager successfully!`);

      // Call the callback if provided (closes RegistryBrowser)
      onServerAdded?.();

      onClose();
    } catch (error) {
      console.error('Failed to add server:', error);
      uiStore.showError(`Failed to add server: ${error}`);
    }
  }

  function getClientName(clientId: string): string {
    return externalClients.find((c) => c.id === clientId)?.name || clientId;
  }
</script>

{#if server}
<div
  class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
  onclick={handleClose}
  role="dialog"
  aria-modal="true"
  onkeydown={(e) => e.key === 'Escape' && handleClose()}
>
  <div
    class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] flex flex-col"
    onclick={(e) => e.stopPropagation()}
    role="document"
  >
    <div class="flex items-start justify-between p-6 border-b border-gray-200 dark:border-gray-700">
      <div class="flex items-start gap-4 flex-1">
        {#if server?.about?.icon}
          <img
            src={server.about.icon}
            alt={server.about.title}
            class="w-12 h-12 rounded-lg object-cover"
            onerror={(e) => {
              const img = e.currentTarget as HTMLImageElement;
              img.style.display = 'none';
              const fallback = img.nextElementSibling as HTMLElement | null;
              fallback?.classList.remove('hidden');
            }}
          />
          <div class="hidden w-12 h-12 rounded-lg bg-gray-200 dark:bg-gray-700 flex items-center justify-center">
            <Package class="text-gray-500" size={28} />
          </div>
        {:else}
          <div class="w-12 h-12 rounded-lg bg-gray-200 dark:bg-gray-700 flex items-center justify-center">
            <Package class="text-gray-500" size={28} />
          </div>
        {/if}

        <div class="flex-1">
          <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
            {server.about?.title || server.name}
          </h2>
          <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
            {server.about?.description || 'Configure this server'}
          </p>

          <div class="flex items-center gap-3 mt-2">
            {#if server.type === 'remote'}
              <span class="flex items-center gap-1 text-xs text-blue-600 dark:text-blue-400">
                <ExternalLink size={14} />
                Remote Server
              </span>
            {:else if server.image?.startsWith('mcp/')}
              <span class="flex items-center gap-1 text-xs text-green-600 dark:text-green-400">
                <CheckCircle size={14} />
                Docker Built
              </span>
            {/if}

            {#if server.source?.project}
              <a
                href={server.source.project}
                target="_blank"
                rel="noopener noreferrer"
                class="text-xs text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1"
              >
                <ExternalLink size={12} />
                View Source
              </a>
            {/if}
          </div>
        </div>
      </div>

      <button
        onclick={handleClose}
        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors"
      >
        <X size={24} />
      </button>
    </div>

    <!-- Step Indicator -->
    <div class="flex items-center justify-center gap-2 py-4 border-b border-gray-200 dark:border-gray-700">
      {#each [1, 2, 3] as step}
        <div class={`flex items-center gap-2 ${currentStep >= step ? 'text-blue-600 dark:text-blue-400' : 'text-gray-400'}`}>
          <div class={`w-8 h-8 rounded-full flex items-center justify-center ${currentStep >= step ? 'bg-blue-600 text-white' : 'bg-gray-200 dark:bg-gray-700'}`}>
            {step}
          </div>
          <span class="text-sm font-medium">
            {step === 1 ? 'Info' : step === 2 ? 'Action' : 'Configure'}
          </span>
        </div>
        {#if step < 3}
          <div class="w-12 h-0.5 bg-gray-200 dark:border-gray-700" />
        {/if}
      {/each}
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6">
      {#if currentStep === 1}
        <!-- Step 1: Server Info -->
        <div class="space-y-6">
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">About This Server</h3>
            <p class="text-gray-600 dark:text-gray-400">
              {server.about?.description || 'No description available'}
            </p>
          </div>

          {#if server.meta?.tags && server.meta.tags.length > 0}
            <div>
              <h4 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Tags</h4>
              <div class="flex flex-wrap gap-2">
                {#each server.meta.tags as tag}
                  <span class="px-3 py-1 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 text-sm rounded-full">
                    {tag}
                  </span>
                {/each}
              </div>
            </div>
          {/if}

          {#if server.type === 'server' && server.image}
            <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
              <h4 class="text-sm font-medium text-blue-900 dark:text-blue-300 mb-2">Container Information</h4>
              <dl class="space-y-1 text-sm">
                <div class="flex justify-between">
                  <dt class="text-blue-700 dark:text-blue-400">Image:</dt>
                  <dd class="text-blue-900 dark:text-blue-200 font-mono">{server.image}</dd>
                </div>
                {#if server.image.startsWith('mcp/')}
                  <div class="flex justify-between">
                    <dt class="text-blue-700 dark:text-blue-400">Security:</dt>
                    <dd class="text-green-600 dark:text-green-400">Signed, SBOM, Provenance</dd>
                  </div>
                {/if}
              </dl>
            </div>
          {/if}
        </div>

      {:else if currentStep === 2}
        <!-- Step 2: Choose Action -->
        <div class="space-y-6">
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">What would you like to do?</h3>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <!-- Add to TurboMCP -->
              <button
                onclick={() => (actionChoice = 'turbomcp')}
                class={`p-6 border-2 rounded-lg hover:bg-opacity-80 transition-all text-left group ${
                  actionChoice === 'turbomcp'
                    ? 'border-blue-600 bg-blue-50 dark:bg-blue-900/20'
                    : 'border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 hover:border-blue-400'
                }`}
              >
                <div class="flex items-start gap-3 mb-3">
                  <div class={`w-12 h-12 rounded-lg flex items-center justify-center flex-shrink-0 ${
                    actionChoice === 'turbomcp' ? 'bg-blue-600' : 'bg-blue-500'
                  }`}>
                    <Package class="text-white" size={24} />
                  </div>
                  <div>
                    <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">Add to TurboMCP Studio</h4>
                    <p class="text-sm text-gray-600 dark:text-gray-400">
                      Add this server to your local server manager
                    </p>
                  </div>
                </div>
                {#if actionChoice === 'turbomcp'}
                  <div class="text-sm text-blue-600 dark:text-blue-400">✓ Selected</div>
                {/if}
              </button>

              <!-- Export to External Clients -->
              <button
                onclick={() => (actionChoice = 'export')}
                class={`p-6 border-2 rounded-lg hover:bg-opacity-80 transition-all text-left group ${
                  actionChoice === 'export'
                    ? 'border-blue-600 bg-blue-50 dark:bg-blue-900/20'
                    : 'border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 hover:border-blue-400'
                }`}
              >
                <div class="flex items-start gap-3 mb-3">
                  <div class={`w-12 h-12 rounded-lg flex items-center justify-center flex-shrink-0 ${
                    actionChoice === 'export' ? 'bg-blue-600' : 'bg-gray-500'
                  }`}>
                    <Download class="text-white" size={24} />
                  </div>
                  <div>
                    <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">Export to External Clients</h4>
                    <p class="text-sm text-gray-600 dark:text-gray-400">
                      Generate configs for Claude Desktop, LM Studio, Cursor, etc.
                    </p>
                  </div>
                </div>
                {#if actionChoice === 'export'}
                  <div class="text-sm text-blue-600 dark:text-blue-400">✓ Selected</div>
                {/if}
              </button>
            </div>
          </div>
        </div>

      {:else if currentStep === 3}
        <!-- Step 3: Configure + Execute -->
        <div class="space-y-6">
          <!-- Configuration Section (always shown) -->
          {#if secrets.length > 0 || (parameters.properties && Object.keys(parameters.properties).length > 0)}
            <div class="border-b border-gray-200 dark:border-gray-700 pb-6">
              <!-- Secrets -->
              {#if secrets.length > 0}
                <div class="mb-6">
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">Secrets & API Keys</h3>
                  <div class="space-y-4">
                    {#each secrets as secret}
                      <div class="bg-yellow-50 dark:bg-yellow-900/10 border border-yellow-200 dark:border-yellow-800 rounded-lg p-4">
                        <label for={secret.name} class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                          {secret.env}
                          {#if secret.required}<span class="text-red-500">*</span>{/if}
                        </label>
                        <input
                          type="password"
                          id={secret.name}
                          value={secretValues[secret.name] || ''}
                          oninput={(e) => handleSecretChange(secret.name, e.currentTarget.value)}
                          placeholder="Enter {secret.env}"
                          class={`w-full px-3 py-2 border rounded-lg ${
                            validationErrors[secret.name] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'
                          }`}
                        />
                        {#if validationErrors[secret.name]}
                          <div class="text-sm text-red-600 mt-1">{validationErrors[secret.name]}</div>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              <!-- Parameters -->
              {#if parameters.properties && Object.keys(parameters.properties).length > 0}
                <div>
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">Configuration Parameters</h3>
                  <SchemaFormGenerator
                    schema={parameters}
                    serverName={server.name}
                    bind:values={parameterValues}
                    bind:errors={validationErrors}
                    onValueChange={handleParameterChange}
                  />
                </div>
              {/if}
            </div>
          {/if}

          <!-- Execute Section (based on action choice) -->
          {#if actionChoice === 'turbomcp'}
            <!-- TurboMCP: Add to Server Manager -->
            <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-6 text-center">
              <h4 class="text-lg font-semibold text-blue-900 dark:text-blue-300 mb-2">Ready to Add to Server Manager</h4>
              <p class="text-sm text-blue-700 dark:text-blue-400 mb-4">
                Click the button below to add <strong>{server.name}</strong> to your local server list
              </p>
              <Button variant="primary" size="lg" onclick={addToServerManager}>
                Add to Server Manager
              </Button>
            </div>

          {:else if actionChoice === 'export'}
            <!-- Export: Select Clients -->
            <div>
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">Select Target Clients</h3>
                <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
                  Choose one or more clients to generate configurations for
                </p>

                <div class="grid grid-cols-2 md:grid-cols-3 gap-3 mb-6">
                  {#each externalClients as client}
                    <button
                      onclick={() => toggleClient(client.id)}
                      class={`p-4 border rounded-lg text-left transition-all ${
                        selectedClients.has(client.id)
                          ? 'border-blue-600 bg-blue-50 dark:bg-blue-900/20 dark:border-blue-500'
                          : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
                      }`}
                    >
                      <div class="font-medium text-gray-900 dark:text-white">{client.name}</div>
                      <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">{client.description}</div>
                    </button>
                  {/each}
                </div>

                {#if selectedClients.size > 0}
                  <div class="space-y-4">
                    {#each Array.from(selectedClients) as clientId}
                      {@const configData = generatedConfigs.get(clientId)}
                      {#if configData}
                        <div class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                          <div class="flex items-center justify-between mb-3">
                            <h4 class="font-semibold text-gray-900 dark:text-white">{getClientName(clientId)}</h4>
                            <div class="flex gap-2">
                              <Button variant="secondary" size="sm" leftIcon={Copy} onclick={() => copyToClipboard(clientId)}>
                                Copy
                              </Button>
                              <Button variant="secondary" size="sm" leftIcon={Download} onclick={() => exportToFile(clientId)}>
                                Export
                              </Button>
                            </div>
                          </div>

                          {#if configData.notes.length > 0}
                            <div class="mb-3 text-sm text-gray-600 dark:text-gray-400">
                              {#each configData.notes as note}
                                <div>• {note}</div>
                              {/each}
                            </div>
                          {/if}

                          <details>
                            <summary class="text-sm text-gray-600 dark:text-gray-400 cursor-pointer hover:text-gray-900 dark:hover:text-white">
                              Show configuration
                            </summary>
                            <pre class="mt-2 bg-gray-900 text-gray-100 p-3 rounded-lg overflow-x-auto text-xs font-mono max-h-64">{configData.json}</pre>
                          </details>
                        </div>
                      {/if}
                    {/each}
                  </div>
                {/if}
              </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="border-t border-gray-200 dark:border-gray-700 p-6 flex items-center justify-between">
      <div>
        {#if currentStep > 1}
          <Button variant="secondary" onclick={() => { currentStep = currentStep - 1; }}>
            Back
          </Button>
        {/if}
      </div>

      <div class="flex gap-3">
        <Button variant="secondary" onclick={onClose}>
          {currentStep === 3 ? 'Close' : 'Cancel'}
        </Button>
        {#if currentStep < 3}
          <Button
            variant="primary"
            onclick={() => {
              if (currentStep === 2) {
                proceedToStep3();
              } else {
                currentStep = currentStep + 1;
              }
            }}
          >
            Next
          </Button>
        {/if}
      </div>
    </div>
  </div>
</div>
{/if}
