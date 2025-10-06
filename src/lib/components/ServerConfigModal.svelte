<script lang="ts">
  import { serverStore, type ServerInfo, type TransportConfig } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import {
    X,
    Save,
    Trash2,
    AlertCircle,
    Database,
    Globe,
    Wifi,
    Network,
    HardDrive,
    Activity,
    CheckCircle,
    Clock,
    Edit,
    Plus,
    Minus
  } from 'lucide-svelte';

  let showModal = $state(true);
  let selectedServerId: string | undefined = $state(undefined);
  let servers: ServerInfo[] = $state([]);
  let editMode = $state(false);
  let saving = $state(false);

  // Edit form state
  let editForm = $state({
    name: '',
    description: '',
    transport: {} as TransportConfig,
    environment_variables: {} as Record<string, string>
  });

  // Subscribe to store changes
  $effect(() => {
    const unsubscribe = serverStore.subscribe((state: any) => {
      servers = state.servers;
      selectedServerId = state.selectedServerId;
    });
    return unsubscribe;
  });

  const selectedServer = $derived(
    servers.find(s => s.id === selectedServerId)
  );

  // Initialize edit form when server changes
  $effect(() => {
    if (selectedServer && !editMode) {
      editForm.name = selectedServer.config.name;
      editForm.description = selectedServer.config.description || '';
      editForm.transport = JSON.parse(JSON.stringify(selectedServer.config.transport_config));
      editForm.environment_variables = JSON.parse(JSON.stringify(selectedServer.config.environment_variables || {}));
    }
  });

  function closeModal() {
    editMode = false;
    uiStore.closeModal('serverConfig');
    showModal = false;
  }

  function enterEditMode() {
    if (selectedServer) {
      editForm.name = selectedServer.config.name;
      editForm.description = selectedServer.config.description || '';
      editForm.transport = JSON.parse(JSON.stringify(selectedServer.config.transport_config));
      editForm.environment_variables = JSON.parse(JSON.stringify(selectedServer.config.environment_variables || {}));
      editMode = true;
    }
  }

  function cancelEdit() {
    editMode = false;
  }

  async function saveChanges() {
    if (!selectedServer) return;

    try {
      saving = true;

      await serverStore.updateServerConfig(
        selectedServer.id,
        editForm.name,
        editForm.description || undefined,
        editForm.transport,
        editForm.environment_variables
      );

      editMode = false;
      uiStore.showSuccess('Server configuration updated successfully');
    } catch (error) {
      uiStore.showError(`Failed to update server: ${error}`);
    } finally {
      saving = false;
    }
  }

  function addEnvironmentVariable() {
    editForm.environment_variables = { ...editForm.environment_variables, '': '' };
  }

  function removeEnvironmentVariable(key: string) {
    const newVars = { ...editForm.environment_variables };
    delete newVars[key];
    editForm.environment_variables = newVars;
  }

  function updateEnvironmentVariableKey(oldKey: string, newKey: string) {
    if (oldKey === newKey) return;

    const newVars = { ...editForm.environment_variables };
    const value = newVars[oldKey];
    delete newVars[oldKey];
    newVars[newKey] = value;
    editForm.environment_variables = newVars;
  }

  function updateEnvironmentVariableValue(key: string, value: string) {
    editForm.environment_variables = { ...editForm.environment_variables, [key]: value };
  }

  let showDeleteConfirm = $state(false);

  function confirmDeleteServer() {
    showDeleteConfirm = true;
  }

  async function deleteServer() {
    if (!selectedServer) return;

    try {
      // First disconnect if connected
      if (selectedServer.status === 'connected') {
        await serverStore.disconnectServer(selectedServer.id);
      }

      // Delete the server configuration from backend and store
      await serverStore.deleteServerConfig(selectedServer.id);

      uiStore.showSuccess(`Server "${selectedServer.config.name}" deleted`);
      showDeleteConfirm = false;
      closeModal();
    } catch (error) {
      uiStore.showError(`Failed to delete server: ${error}`);
      showDeleteConfirm = false;
    }
  }

  function cancelDelete() {
    showDeleteConfirm = false;
  }

  function getTransportIcon(type: string) {
    switch (type) {
      case 'stdio': return Database;
      case 'http': return Globe;
      case 'websocket': return Wifi;
      case 'tcp': return Network;
      case 'unix': return HardDrive;
      default: return Activity;
    }
  }

  function getStatusColor(status: string) {
    switch (status?.toLowerCase()) {
      case 'connected': return 'text-green-600 bg-green-100';
      case 'connecting': return 'text-yellow-600 bg-yellow-100';
      case 'error': return 'text-red-600 bg-red-100';
      default: return 'text-gray-600 bg-gray-100';
    }
  }

  function formatDateTime(dateString: string) {
    return new Date(dateString).toLocaleString();
  }

  function formatBytes(bytes: number) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }
</script>

{#if showModal && selectedServer}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
    <div class="bg-white rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between p-6 border-b border-gray-200">
        <div class="flex items-center">
          {#if selectedServer.config.transport_config}
            {@const IconComponent = getTransportIcon(selectedServer.config.transport_config.type)}
            <div class="p-2 bg-gray-100 rounded-lg mr-4">
              <IconComponent size={20} class="text-gray-600" />
            </div>
          {/if}
          <div>
            <h2 class="text-xl font-semibold text-gray-900">
              {editMode ? 'Edit Server Configuration' : selectedServer.config.name}
            </h2>
            <div class="flex items-center mt-1">
              <span class="text-sm text-gray-600 mr-3">
                {selectedServer.config.transport_config?.type?.toUpperCase() || 'UNKNOWN'} Transport
              </span>
              <div class="flex items-center px-2 py-1 rounded-full text-xs font-medium {getStatusColor(selectedServer.status)}">
                {#if selectedServer.status === 'connected'}
                  <CheckCircle size={12} class="mr-1" />
                {:else if selectedServer.status === 'connecting'}
                  <Clock size={12} class="mr-1" />
                {:else if selectedServer.status === 'error'}
                  <AlertCircle size={12} class="mr-1" />
                {:else}
                  <Activity size={12} class="mr-1" />
                {/if}
                {selectedServer.status}
              </div>
            </div>
          </div>
        </div>
        <div class="flex items-center space-x-2">
          {#if !editMode}
            <button
              onclick={enterEditMode}
              class="flex items-center px-3 py-2 text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
            >
              <Edit size={16} class="mr-2" />
              Edit
            </button>
          {/if}
          <button onclick={closeModal} class="text-gray-400 hover:text-gray-600">
            <X size={24} />
          </button>
        </div>
      </div>

      <!-- Content -->
      <div class="p-6 overflow-y-auto max-h-[60vh]">
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- Configuration Details -->
          <div class="space-y-6">
            <div>
              <h3 class="text-lg font-semibold text-gray-900 mb-4">Configuration</h3>

              <div class="space-y-4">
                <div>
                  <label class="text-sm font-medium text-gray-700">Server Name</label>
                  {#if editMode}
                    <input
                      type="text"
                      bind:value={editForm.name}
                      class="mt-1 w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                      required
                    />
                  {:else}
                    <p class="text-sm text-gray-900 mt-1">{selectedServer.config.name}</p>
                  {/if}
                </div>

                <div>
                  <label class="text-sm font-medium text-gray-700">Description</label>
                  {#if editMode}
                    <textarea
                      bind:value={editForm.description}
                      class="mt-1 w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                      rows="2"
                      placeholder="Optional description"
                    ></textarea>
                  {:else if selectedServer.config.description}
                    <p class="text-sm text-gray-900 mt-1">{selectedServer.config.description}</p>
                  {:else}
                    <p class="text-sm text-gray-500 mt-1 italic">No description</p>
                  {/if}
                </div>

                <div>
                  <label class="text-sm font-medium text-gray-700">Transport Type</label>
                  {#if editMode}
                    <p class="text-sm text-gray-500 mt-1 italic">
                      Transport type cannot be changed after creation
                    </p>
                  {/if}
                  <p class="text-sm text-gray-900 mt-1 capitalize">
                    {selectedServer.config.transport_config?.type || 'Unknown'}
                  </p>
                </div>

                <!-- Transport-specific details -->
                {#if selectedServer.config.transport_config?.type === 'stdio'}
                  <div>
                    <label class="text-sm font-medium text-gray-700">Command</label>
                    <p class="text-sm text-gray-900 mt-1 font-mono bg-gray-50 p-2 rounded">
                      {selectedServer.config.transport_config.command}
                    </p>
                  </div>

                  {#if selectedServer.config.transport_config.args && selectedServer.config.transport_config.args.length > 0}
                    <div>
                      <label class="text-sm font-medium text-gray-700">Arguments</label>
                      <p class="text-sm text-gray-900 mt-1 font-mono bg-gray-50 p-2 rounded">
                        {selectedServer.config.transport_config.args.join(' ')}
                      </p>
                    </div>
                  {/if}

                  {#if selectedServer.config.transport_config.working_directory}
                    <div>
                      <label class="text-sm font-medium text-gray-700">Working Directory</label>
                      <p class="text-sm text-gray-900 mt-1 font-mono bg-gray-50 p-2 rounded">
                        {selectedServer.config.transport_config.working_directory}
                      </p>
                    </div>
                  {/if}

                {:else if selectedServer.config.transport_config?.type === 'http' || selectedServer.config.transport_config?.type === 'websocket'}
                  <div>
                    <label class="text-sm font-medium text-gray-700">URL</label>
                    <p class="text-sm text-gray-900 mt-1 font-mono bg-gray-50 p-2 rounded">
                      {selectedServer.config.transport_config.url}
                    </p>
                  </div>

                  {#if selectedServer.config.transport_config.headers && Object.keys(selectedServer.config.transport_config.headers).length > 0}
                    <div>
                      <label class="text-sm font-medium text-gray-700">Headers</label>
                      <div class="mt-1 space-y-1">
                        {#each Object.entries(selectedServer.config.transport_config.headers) as [key, value]}
                          <div class="text-sm bg-gray-50 p-2 rounded font-mono">
                            <span class="text-gray-600">{key}:</span> {value}
                          </div>
                        {/each}
                      </div>
                    </div>
                  {/if}

                {:else if selectedServer.config.transport_config?.type === 'tcp'}
                  <div class="grid grid-cols-2 gap-4">
                    <div>
                      <label class="text-sm font-medium text-gray-700">Host</label>
                      <p class="text-sm text-gray-900 mt-1 font-mono bg-gray-50 p-2 rounded">
                        {selectedServer.config.transport_config.host}
                      </p>
                    </div>
                    <div>
                      <label class="text-sm font-medium text-gray-700">Port</label>
                      <p class="text-sm text-gray-900 mt-1 font-mono bg-gray-50 p-2 rounded">
                        {selectedServer.config.transport_config.port}
                      </p>
                    </div>
                  </div>

                {:else if selectedServer.config.transport_config?.type === 'unix'}
                  <div>
                    <label class="text-sm font-medium text-gray-700">Socket Path</label>
                    <p class="text-sm text-gray-900 mt-1 font-mono bg-gray-50 p-2 rounded">
                      {selectedServer.config.transport_config.path}
                    </p>
                  </div>
                {/if}

                <!-- Environment Variables -->
                <div>
                  <div class="flex items-center justify-between">
                    <label class="text-sm font-medium text-gray-700">Environment Variables</label>
                    {#if editMode}
                      <button
                        onclick={addEnvironmentVariable}
                        class="flex items-center text-xs text-blue-600 hover:text-blue-800"
                      >
                        <Plus size={14} class="mr-1" />
                        Add Variable
                      </button>
                    {/if}
                  </div>

                  {#if editMode}
                    <div class="mt-2 space-y-2">
                      {#each Object.entries(editForm.environment_variables) as [key, value], index}
                        <div class="flex items-center space-x-2">
                          <input
                            type="text"
                            value={key}
                            onchange={(e) => updateEnvironmentVariableKey(key, (e.target as HTMLInputElement).value)}
                            placeholder="Variable name"
                            class="flex-1 px-2 py-1 text-xs border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
                          />
                          <span class="text-gray-400">=</span>
                          <input
                            type="text"
                            value={value}
                            onchange={(e) => updateEnvironmentVariableValue(key, (e.target as HTMLInputElement).value)}
                            placeholder="Value"
                            class="flex-1 px-2 py-1 text-xs border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
                          />
                          <button
                            onclick={() => removeEnvironmentVariable(key)}
                            class="text-red-500 hover:text-red-700"
                          >
                            <Minus size={14} />
                          </button>
                        </div>
                      {/each}
                      {#if Object.keys(editForm.environment_variables).length === 0}
                        <p class="text-xs text-gray-500 italic">No environment variables set</p>
                      {/if}
                    </div>
                  {:else if selectedServer.config.environment_variables && Object.keys(selectedServer.config.environment_variables).length > 0}
                    <div class="mt-1 space-y-1">
                      {#each Object.entries(selectedServer.config.environment_variables) as [key, value]}
                        <div class="text-sm bg-gray-50 p-2 rounded font-mono">
                          <span class="text-gray-600">{key}=</span>{value}
                        </div>
                      {/each}
                    </div>
                  {:else}
                    <p class="text-sm text-gray-500 mt-1 italic">No environment variables set</p>
                  {/if}
                </div>

                <!-- Timestamps -->
                <div class="grid grid-cols-2 gap-4">
                  <div>
                    <label class="text-sm font-medium text-gray-700">Created</label>
                    <p class="text-sm text-gray-900 mt-1">
                      {formatDateTime(selectedServer.config.created_at)}
                    </p>
                  </div>
                  <div>
                    <label class="text-sm font-medium text-gray-700">Last Updated</label>
                    <p class="text-sm text-gray-900 mt-1">
                      {formatDateTime(selectedServer.config.updated_at)}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Runtime Information -->
          <div class="space-y-6">
            <!-- Server Capabilities -->
            {#if selectedServer.capabilities}
              <div>
                <h3 class="text-lg font-semibold text-gray-900 mb-4">Capabilities</h3>
                <div class="flex flex-wrap gap-2">
                  {#if selectedServer.capabilities.tools}
                    <span class="px-3 py-1 bg-blue-100 text-blue-700 text-sm rounded-full">Tools</span>
                  {/if}
                  {#if selectedServer.capabilities.resources}
                    <span class="px-3 py-1 bg-green-100 text-green-700 text-sm rounded-full">Resources</span>
                  {/if}
                  {#if selectedServer.capabilities.prompts}
                    <span class="px-3 py-1 bg-purple-100 text-purple-700 text-sm rounded-full">Prompts</span>
                  {/if}
                  {#if selectedServer.capabilities.sampling}
                    <span class="px-3 py-1 bg-orange-100 text-orange-700 text-sm rounded-full">Sampling</span>
                  {/if}
                  {#if selectedServer.capabilities.elicitation}
                    <span class="px-3 py-1 bg-pink-100 text-pink-700 text-sm rounded-full">Elicitation</span>
                  {/if}
                </div>
              </div>
            {/if}

            <!-- Connection Metrics -->
            {#if selectedServer.metrics && selectedServer.status === 'connected'}
              <div>
                <h3 class="text-lg font-semibold text-gray-900 mb-4">Connection Metrics</h3>
                <div class="grid grid-cols-2 gap-4">
                  <div class="bg-gray-50 rounded-lg p-3">
                    <p class="text-xs text-gray-600">Requests Sent</p>
                    <p class="text-lg font-semibold text-gray-900">{selectedServer.metrics.requests_sent}</p>
                  </div>
                  <div class="bg-gray-50 rounded-lg p-3">
                    <p class="text-xs text-gray-600">Responses Received</p>
                    <p class="text-lg font-semibold text-gray-900">{selectedServer.metrics.responses_received}</p>
                  </div>
                  <div class="bg-gray-50 rounded-lg p-3">
                    <p class="text-xs text-gray-600">Data Sent</p>
                    <p class="text-lg font-semibold text-gray-900">{formatBytes(selectedServer.metrics.bytes_sent)}</p>
                  </div>
                  <div class="bg-gray-50 rounded-lg p-3">
                    <p class="text-xs text-gray-600">Data Received</p>
                    <p class="text-lg font-semibold text-gray-900">{formatBytes(selectedServer.metrics.bytes_received)}</p>
                  </div>
                  <div class="bg-gray-50 rounded-lg p-3">
                    <p class="text-xs text-gray-600">Avg Response Time</p>
                    <p class="text-lg font-semibold text-gray-900">{Math.round(selectedServer.metrics.avg_response_time_ms)}ms</p>
                  </div>
                  <div class="bg-gray-50 rounded-lg p-3">
                    <p class="text-xs text-gray-600">Error Count</p>
                    <p class="text-lg font-semibold text-gray-900">{selectedServer.metrics.error_count}</p>
                  </div>
                </div>
              </div>
            {/if}

            <!-- Process Information -->
            {#if selectedServer.process_info && selectedServer.status === 'connected'}
              <div>
                <h3 class="text-lg font-semibold text-gray-900 mb-4">Process Information</h3>
                <div class="space-y-3">
                  <div class="flex justify-between">
                    <span class="text-sm text-gray-600">Process ID</span>
                    <span class="text-sm font-medium text-gray-900">{selectedServer.process_info.pid}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-sm text-gray-600">CPU Usage</span>
                    <span class="text-sm font-medium text-gray-900">{selectedServer.process_info.cpu_usage.toFixed(1)}%</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-sm text-gray-600">Memory Usage</span>
                    <span class="text-sm font-medium text-gray-900">{formatBytes(selectedServer.process_info.memory_usage)}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-sm text-gray-600">Started At</span>
                    <span class="text-sm font-medium text-gray-900">{formatDateTime(selectedServer.process_info.started_at)}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-sm text-gray-600">Status</span>
                    <span class="text-sm font-medium text-gray-900 capitalize">{selectedServer.process_info.status}</span>
                  </div>
                </div>
              </div>
            {/if}

            <!-- Error Information -->
            {#if selectedServer.status === 'error' && selectedServer.metrics.last_error}
              <div>
                <h3 class="text-lg font-semibold text-gray-900 mb-4">Error Details</h3>
                <div class="bg-red-50 border border-red-200 rounded-lg p-4">
                  <div class="flex items-start">
                    <AlertCircle size={16} class="text-red-500 mr-2 mt-0.5 flex-shrink-0" />
                    <div>
                      <p class="text-sm font-medium text-red-800">Last Error</p>
                      <p class="text-sm text-red-700 mt-1">{selectedServer.metrics.last_error}</p>
                    </div>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between p-6 border-t border-gray-200 bg-gray-50">
        <div>
          {#if !editMode}
            <button
              onclick={confirmDeleteServer}
              class="flex items-center px-4 py-2 text-red-600 hover:bg-red-50 rounded-lg transition-colors"
            >
              <Trash2 size={16} class="mr-2" />
              Delete Server
            </button>
          {/if}
        </div>

        <div class="flex space-x-3">
          {#if editMode}
            <button
              onclick={cancelEdit}
              class="btn-secondary"
              disabled={saving}
            >
              Cancel
            </button>
            <button
              onclick={saveChanges}
              class="btn-primary"
              disabled={saving || !editForm.name.trim()}
            >
              {#if saving}
                <div class="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full mr-2"></div>
                Saving...
              {:else}
                <Save size={16} class="mr-2" />
                Save Changes
              {/if}
            </button>
          {:else}
            <button onclick={closeModal} class="btn-secondary">
              Close
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Delete Confirmation Modal -->
{#if showDeleteConfirm}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white rounded-lg p-6 max-w-md w-full mx-4">
      <div class="flex items-center mb-4">
        <div class="flex-shrink-0 w-12 h-12 bg-red-100 rounded-full flex items-center justify-center mr-4">
          <Trash2 size={24} class="text-red-600" />
        </div>
        <div>
          <h3 class="text-lg font-semibold text-gray-900">Delete Server</h3>
          <p class="text-sm text-gray-600">This action cannot be undone</p>
        </div>
      </div>

      <p class="text-gray-700 mb-6">
        Are you sure you want to delete "<strong>{selectedServer?.config.name}</strong>"?
        All configurations and connection history will be permanently removed.
      </p>

      <div class="flex justify-end space-x-3">
        <button
          onclick={cancelDelete}
          class="px-4 py-2 text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-lg transition-colors"
        >
          Cancel
        </button>
        <button
          onclick={deleteServer}
          class="px-4 py-2 bg-red-600 text-white hover:bg-red-700 rounded-lg transition-colors"
        >
          Delete Server
        </button>
      </div>
    </div>
  </div>
{/if}