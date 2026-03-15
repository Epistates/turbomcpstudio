<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { X, Save, Info } from 'lucide-svelte';
  import { uiStore } from '$lib/stores/uiStore';
  import { serverStore } from '$lib/stores/serverStore';
  import Button from './ui/Button.svelte';
  import SchemaFormGenerator from './SchemaFormGenerator.svelte';

  // Get selected server from store
  const serverState = $derived($serverStore);
  const server = $derived(serverState.selectedServerId ? serverState.servers.get(serverState.selectedServerId) : undefined);

  // Detect server type
  const isDockerServer = $derived(server?.config?.transport_config?.type === 'stdio' &&
                      server?.config?.transport_config?.command?.includes('docker'));

  // State for editing
  let name = $state('');
  let description = $state('');
  let command = $state('');
  let args = $state<string[]>([]);
  let workingDirectory = $state('');
  let url = $state('');
  let envVars = $state<Record<string, string>>({});

  // Docker config state
  let dockerConfigParams = $state<Record<string, any>>({});
  let dockerConfigErrors = $state<Record<string, string>>({});

  // Initialize form when server changes
  $effect(() => {
    if (server) {
      name = server.config.name;
      description = server.config.description || '';

      const transport = server.config.transport_config;
      if (transport.type === 'stdio') {
        command = transport.command;
        args = transport.args || [];
        workingDirectory = transport.working_directory || '';
      } else if (transport.type === 'http' || transport.type === 'webSocket') {
        url = transport.url;
      }

      envVars = { ...server.config.environment_variables };
    }
  });

  function handleClose() {
    serverStore.selectServer(undefined);
    uiStore.closeModal('serverConfig');
  }

  async function handleSave() {
    if (!server) return;

    try {
      // Build updated config
      const updatedConfig = {
        ...server.config,
        name,
        description: description || undefined,
        environment_variables: envVars,
      };

      // Update transport config based on type
      if (server.config.transport_config.type === 'stdio') {
        updatedConfig.transport_config = {
          type: 'stdio',
          command,
          args,
          working_directory: workingDirectory || undefined,
        };
      } else if (server.config.transport_config.type === 'http') {
        updatedConfig.transport_config = {
          type: 'http',
          url,
          headers: server.config.transport_config.headers || {},
        };
      } else if (server.config.transport_config.type === 'webSocket') {
        updatedConfig.transport_config = {
          type: 'webSocket',
          url,
          headers: server.config.transport_config.headers || {},
        };
      }

      // Save to backend
      await invoke('update_server_config', {
        request: {
          id: server.id,
          name: updatedConfig.name,
          description: updatedConfig.description,
          transport: updatedConfig.transport_config,
          environment_variables: updatedConfig.environment_variables
        }
      });

      // Reload servers
      await serverStore.loadServers();

      uiStore.showSuccess('Server configuration updated successfully');
      handleClose();
    } catch (error) {
      console.error('Failed to update server:', error);
      uiStore.showError(`Failed to update server: ${error}`);
    }
  }

  function handleEnvVarAdd() {
    const key = prompt('Environment variable name:');
    if (!key) return;
    const value = prompt('Environment variable value:');
    if (value === null) return;
    envVars[key] = value;
    envVars = { ...envVars };
  }

  function handleEnvVarRemove(key: string) {
    delete envVars[key];
    envVars = { ...envVars };
  }
</script>

{#if server}
<div
  class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
  onclick={handleClose}
  onkeydown={(e) => e.key === 'Escape' && handleClose()}
  role="presentation"
>
  <div
    class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-3xl w-full max-h-[90vh] flex flex-col"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-labelledby="edit-server-title"
    tabindex="-1"
  >
    <!-- Header -->
    <div class="flex items-start justify-between p-6 border-b border-gray-200 dark:border-gray-700">
      <div>
        <h2 id="edit-server-title" class="text-2xl font-bold text-gray-900 dark:text-white">
          Edit Server Configuration
        </h2>
        <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
          {isDockerServer ? 'Docker MCP Server' : 'Native Transport Server'}
        </p>
      </div>
      <button
        onclick={handleClose}
        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
        aria-label="Close"
      >
        <X size={24} />
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6 space-y-6">
      <!-- Basic Info -->
      <div class="space-y-4">
        <div>
          <label for="edit-server-name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Server Name <span class="text-red-500">*</span>
          </label>
          <input
            id="edit-server-name"
            type="text"
            bind:value={name}
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            placeholder="My MCP Server"
          />
        </div>

        <div>
          <label for="edit-server-description" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Description
          </label>
          <textarea
            id="edit-server-description"
            bind:value={description}
            rows="2"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            placeholder="Optional description"
          ></textarea>
        </div>
      </div>

      <!-- Transport Configuration -->
      {#if server.config.transport_config.type === 'stdio'}
        <div class="space-y-4">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Transport Configuration</h3>

          <div>
            <label for="edit-server-stdio-command" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Command <span class="text-red-500">*</span>
            </label>
            <input
              id="edit-server-stdio-command"
              type="text"
              bind:value={command}
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
              placeholder="node"
            />
          </div>

          <div>
            <label for="edit-server-stdio-args" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Arguments (one per line)
            </label>
            <textarea
              id="edit-server-stdio-args"
              value={args.join('\n')}
              oninput={(e) => args = e.currentTarget.value.split('\n').filter(a => a.trim())}
              rows="3"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
              placeholder="server.js"
            ></textarea>
          </div>

          <div>
            <label for="edit-server-stdio-workdir" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Working Directory
            </label>
            <input
              id="edit-server-stdio-workdir"
              type="text"
              bind:value={workingDirectory}
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
              placeholder="/path/to/directory"
            />
          </div>
        </div>
      {:else if server.config.transport_config.type === 'http' || server.config.transport_config.type === 'webSocket'}
        <div class="space-y-4">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Transport Configuration</h3>

          <div>
            <label for="edit-server-url" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              URL <span class="text-red-500">*</span>
            </label>
            <input
              id="edit-server-url"
              type="text"
              bind:value={url}
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
              placeholder="http://localhost:3000"
            />
          </div>
        </div>
      {/if}

      <!-- Environment Variables -->
      <div class="space-y-4">
        <div class="flex items-center justify-between">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Environment Variables</h3>
          <Button variant="secondary" size="sm" onclick={handleEnvVarAdd}>
            Add Variable
          </Button>
        </div>

        {#if Object.keys(envVars).length > 0}
          <div class="space-y-2">
            {#each Object.entries(envVars) as [key, value], i}
              <div class="flex gap-2 items-center">
                <input
                  type="text"
                  value={key}
                  disabled
                  aria-label="Variable name"
                  class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-white font-mono text-sm"
                />
                <label for="edit-server-env-value-{i}" class="sr-only">Value for {key}</label>
                <input
                  id="edit-server-env-value-{i}"
                  type="text"
                  bind:value={envVars[key]}
                  class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
                />
                <button
                  onclick={() => handleEnvVarRemove(key)}
                  class="px-3 py-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded"
                  aria-label="Remove environment variable {key}"
                >
                  <X size={16} />
                </button>
              </div>
            {/each}
          </div>
        {:else}
          <p class="text-sm text-gray-500 dark:text-gray-400">No environment variables configured</p>
        {/if}
      </div>

      <!-- Info Notice -->
      <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
        <div class="flex gap-2">
          <Info class="text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" size={16} />
          <div class="text-sm text-blue-800 dark:text-blue-200">
            <strong>Note:</strong> Changes will take effect after you reconnect to the server. If the server is currently connected, you may need to disconnect and reconnect.
          </div>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="border-t border-gray-200 dark:border-gray-700 p-6">
      <div class="flex justify-end gap-3">
        <Button variant="secondary" onclick={handleClose}>
          Cancel
        </Button>
        <Button variant="primary" leftIcon={Save} onclick={handleSave}>
          Save Changes
        </Button>
      </div>
    </div>
  </div>
</div>
{/if}
