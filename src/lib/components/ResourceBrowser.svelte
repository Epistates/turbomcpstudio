<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';

  interface ResourceExecution {
    id: string;
    serverId: string;
    serverName: string;
    resourceUri: string;
    resourceName: string;
    parameters?: Record<string, any>;
    result: string;
    timestamp: string;
    duration?: number;
    status: 'success' | 'error';
    error?: string;
  }

  import {
    Database,
    File,
    Folder,
    Search,
    Download,
    Upload,
    RefreshCw,
    AlertCircle,
    CheckCircle,
    Clock,
    ExternalLink,
    Copy,
    Eye,
    Edit,
    Filter,
    Play,
    History
  } from 'lucide-svelte';

  let servers: ServerInfo[] = $state([]);
  let selectedServerId: string | undefined = $state(undefined);
  let resources: any[] = $state([]);
  let loading = $state(false);
  let searchQuery = $state('');
  let selectedResource: any = $state(null);
  let resourceContent: string = $state('');
  let contentLoading = $state(false);
  let executionHistory: ResourceExecution[] = $state([]);
  let resourceParameters = $state<Record<string, any>>({});
  let showParameterForm = $state(false);
  let parameterName = $state('');
  let parameterOptions: string[] = $state([]);
  let isHistoricalResult = $state(false);
  let filterType = $state('all');

  // Subscribe to stores
  $effect(() => {
    const unsubscribeServers = serverStore.subscribe(state => {
      const connectedServers = state.servers.filter(s => s.status?.toLowerCase() === 'connected');
      servers = connectedServers;

      if (selectedServerId !== state.selectedServerId) {
        selectedServerId = state.selectedServerId;
        if (selectedServerId) {
          loadResources();
        }
      }

      // Auto-select first connected server if none selected
      if (!state.selectedServerId && connectedServers.length > 0 && !selectedServerId) {
        serverStore.selectServer(connectedServers[0].id);
      }
    });

    return () => {
      unsubscribeServers();
    };
  });

  const filteredResources = $derived.by(() => {
    let filtered = resources;
    console.log('🔍 FILTER DEBUG: Starting with resources:', resources.length);

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(resource =>
        resource.uri?.toLowerCase().includes(query) ||
        resource.name?.toLowerCase().includes(query) ||
        resource.description?.toLowerCase().includes(query)
      );
      console.log('🔍 FILTER DEBUG: After search filter:', filtered.length);
    }

    // Filter by type
    if (filterType !== 'all') {
      filtered = filtered.filter(resource => {
        const mimeType = resource.mimeType || '';
        switch (filterType) {
          case 'text':
            return mimeType.startsWith('text/') || mimeType === 'application/json';
          case 'image':
            return mimeType.startsWith('image/');
          case 'document':
            return mimeType.includes('pdf') || mimeType.includes('document') || mimeType.includes('spreadsheet');
          case 'data':
            return mimeType.includes('json') || mimeType.includes('xml') || mimeType.includes('csv');
          default:
            return true;
        }
      });
      console.log('🔍 FILTER DEBUG: After type filter:', filtered.length, 'filterType:', filterType);
    }

    console.log('🔍 FILTER DEBUG: Final filtered resources:', filtered.length);
    return filtered;
  });

  async function loadResources() {
    if (!selectedServerId) return;

    loading = true;
    try {
      // Check if server supports resources capability
      const serverInfo = await invoke('get_server_info', { serverId: selectedServerId });
      console.log('🔍 RESOURCE DEBUG: Server info received:', serverInfo);
      console.log('🔍 RESOURCE DEBUG: Capabilities:', serverInfo.capabilities);
      console.log('🔍 RESOURCE DEBUG: Resources capability:', serverInfo.capabilities?.resources);
      console.log('🔍 RESOURCE DEBUG: typeof capabilities:', typeof serverInfo.capabilities);
      console.log('🔍 RESOURCE DEBUG: typeof resources:', typeof serverInfo.capabilities?.resources);
      console.log('🔍 RESOURCE DEBUG: resources truthy?:', !!serverInfo.capabilities?.resources);
      console.log('🔍 RESOURCE DEBUG: JSON.stringify capabilities:', JSON.stringify(serverInfo.capabilities, null, 2));

      if (!serverInfo.capabilities?.resources) {
        resources = [];
        console.log('❌ RESOURCE DEBUG: No resources capability found, showing info message');
        uiStore.showInfo('This server does not support resources operations. Try the Tools tab instead.');
        return;
      }

      console.log('✅ RESOURCE DEBUG: Capability check passed, calling list_resources...');
      const resourceList = await invoke('list_resources', { serverId: selectedServerId }) as any[];
      console.log('✅ RESOURCE DEBUG: list_resources call succeeded, got:', resourceList);
      console.log('✅ RESOURCE DEBUG: Raw resource list structure:', JSON.stringify(resourceList, null, 2));

      resources = resourceList.map((resource: any) => {
        console.log('✅ RESOURCE DEBUG: Processing resource:', resource);
        return {
          uri: resource.uri,
          name: resource.name || extractNameFromUri(resource.uri),
          description: resource.description,
          mimeType: resource.mimeType || resource.mime_type || 'application/octet-stream',
          annotations: resource.annotations || {}
        };
      });

      console.log('✅ RESOURCE DEBUG: Processed resources array:', resources);
      console.log('✅ RESOURCE DEBUG: Resources length:', resources.length);
      uiStore.showSuccess(`Loaded ${resources.length} resources`);
    } catch (error) {
      console.error('Failed to load resources:', error);
      const errorStr = String(error);

      if (errorStr.includes('Method not found') || errorStr.includes('-32601')) {
        uiStore.showError('This server does not implement resources operations. Check server capabilities or try a different MCP server.');
      } else {
        uiStore.showError(`Failed to load resources: ${error}`);
      }
      resources = [];
    } finally {
      loading = false;
    }
  }

  function selectResource(resource: any) {
    selectedResource = resource;
    resourceContent = '';
    isHistoricalResult = false;

    // Check if resource has parameters
    if (resource.uri.includes('{') && resource.uri.includes('}')) {
      setupParameterForm(resource);
    } else {
      // Don't auto-load, just prepare for manual load
      showParameterForm = false;
      parameterName = '';
      parameterOptions = [];
      resourceParameters = {};
    }
  }

  function setupParameterForm(resource: any) {
    const paramMatch = resource.uri.match(/\{(\w+)\}/);
    if (paramMatch) {
      parameterName = paramMatch[1];

      // Known values for TurboMCP example 04 server
      const knownValues: { [key: string]: string[] } = {
        'name': ['readme', 'guide', 'api'],
        'template': ['code_review', 'documentation']
      };

      parameterOptions = knownValues[parameterName] || [];
      resourceParameters = {};
      showParameterForm = true;
    }
  }

  async function executeResourceLoad() {
    if (!selectedServerId || !selectedResource || !resourceParameters[parameterName]) return;

    const startTime = Date.now();
    const executionId = crypto.randomUUID();
    const serverInfo = servers.find(s => s.id === selectedServerId);

    contentLoading = true;
    resourceContent = '';
    isHistoricalResult = false;

    try {
      // Substitute parameters in URI
      let actualUri = selectedResource.uri.replace(`{${parameterName}}`, resourceParameters[parameterName]);

      console.log('🔍 RESOURCE DEBUG: About to call read_resource with:', {
        serverId: selectedServerId,
        originalUri: selectedResource.uri,
        actualUri: actualUri,
        parameters: resourceParameters
      });

      const content = await invoke('read_resource', {
        serverId: selectedServerId,
        resourceUri: actualUri
      });

      // Format content
      resourceContent = formatContent(content);

      // Track execution history
      const execution: ResourceExecution = {
        id: executionId,
        serverId: selectedServerId,
        serverName: serverInfo?.config.name || 'Unknown Server',
        resourceUri: actualUri,
        resourceName: selectedResource.name || selectedResource.uri,
        parameters: { ...resourceParameters },
        result: resourceContent,
        timestamp: new Date().toISOString(),
        duration: Date.now() - startTime,
        status: 'success'
      };

      executionHistory = [execution, ...executionHistory.slice(0, 49)];
      uiStore.showSuccess(`Loaded content for ${selectedResource.name || actualUri}`);
    } catch (error) {
      console.error('Failed to load resource content:', error);
      const errorMessage = `Error loading content: ${error}`;
      resourceContent = errorMessage;

      // Track failed execution
      const execution: ResourceExecution = {
        id: executionId,
        serverId: selectedServerId,
        serverName: serverInfo?.config.name || 'Unknown Server',
        resourceUri: selectedResource.uri,
        resourceName: selectedResource.name || selectedResource.uri,
        parameters: { ...resourceParameters },
        result: '',
        timestamp: new Date().toISOString(),
        duration: Date.now() - startTime,
        status: 'error',
        error: String(error)
      };

      executionHistory = [execution, ...executionHistory.slice(0, 49)];
      uiStore.showError(`Failed to load content: ${error}`);
    } finally {
      contentLoading = false;
    }
  }

  async function loadResourceContent(resource: any) {
    if (!selectedServerId || !resource) return;

    const startTime = Date.now();
    const executionId = crypto.randomUUID();
    const serverInfo = servers.find(s => s.id === selectedServerId);

    contentLoading = true;
    resourceContent = '';
    isHistoricalResult = false;

    try {
      console.log('🔍 RESOURCE DEBUG: About to call read_resource with:', {
        serverId: selectedServerId,
        resourceUri: resource.uri
      });

      const content = await invoke('read_resource', {
        serverId: selectedServerId,
        resourceUri: resource.uri
      });

      // Format content
      resourceContent = formatContent(content);

      // Track execution history
      const execution: ResourceExecution = {
        id: executionId,
        serverId: selectedServerId,
        serverName: serverInfo?.config.name || 'Unknown Server',
        resourceUri: resource.uri,
        resourceName: resource.name || resource.uri,
        result: resourceContent,
        timestamp: new Date().toISOString(),
        duration: Date.now() - startTime,
        status: 'success'
      };

      executionHistory = [execution, ...executionHistory.slice(0, 49)];
      uiStore.showSuccess(`Loaded content for ${resource.name || resource.uri}`);
    } catch (error) {
      console.error('Failed to load resource content:', error);
      const errorMessage = `Error loading content: ${error}`;
      resourceContent = errorMessage;

      // Track failed execution
      const execution: ResourceExecution = {
        id: executionId,
        serverId: selectedServerId,
        serverName: serverInfo?.config.name || 'Unknown Server',
        resourceUri: resource.uri,
        resourceName: resource.name || resource.uri,
        result: '',
        timestamp: new Date().toISOString(),
        duration: Date.now() - startTime,
        status: 'error',
        error: String(error)
      };

      executionHistory = [execution, ...executionHistory.slice(0, 49)];
      uiStore.showError(`Failed to load content: ${error}`);
    } finally {
      contentLoading = false;
    }
  }

  function formatContent(content: any): string {
    if (Array.isArray(content)) {
      // Multiple content blocks
      return content.map((item: any, index: number) => {
        let text = `--- Content Block ${index + 1} ---\n`;
        if (item.type === 'text') {
          text += item.text || item.content || '';
        } else if (item.type === 'resource') {
          text += `Resource: ${item.resource?.uri || 'Unknown'}\n`;
          text += item.resource?.text || item.resource?.content || '';
        } else {
          text += JSON.stringify(item, null, 2);
        }
        return text;
      }).join('\n\n');
    } else if (typeof content === 'object' && content !== null) {
      // Single content object
      const contentObj = content as any;
      if (contentObj.type === 'text') {
        return contentObj.text || contentObj.content || '';
      } else if (contentObj.type === 'resource') {
        return contentObj.resource?.text || contentObj.resource?.content || '';
      } else {
        return JSON.stringify(content, null, 2);
      }
    } else if (typeof content === 'string') {
      // Handle string formatting
      return content
        .replace(/\\n/g, '\n')  // Convert \n to actual newlines
        .replace(/\\t/g, '\t')  // Convert \t to actual tabs
        .replace(/\\r/g, '\r')  // Convert \r to actual carriage returns
        .replace(/\\\\/g, '\\'); // Convert \\\\ to single backslash
    } else {
      // Plain content
      return String(content);
    }
  }

  function rerunResourceExecution(execution: ResourceExecution) {
    // Simply display the historical result without making a new call
    resourceContent = execution.result;
    isHistoricalResult = true;

    // If there were parameters, restore them for reference
    if (execution.parameters) {
      resourceParameters = { ...execution.parameters };
      showParameterForm = true;
    }
  }

  function selectServer(serverId: string) {
    serverStore.selectServer(serverId);
  }

  function extractNameFromUri(uri: string): string {
    try {
      // Extract filename from various URI schemes
      if (uri.startsWith('file://')) {
        return uri.split('/').pop() || uri;
      } else if (uri.startsWith('http://') || uri.startsWith('https://')) {
        const url = new URL(uri);
        return url.pathname.split('/').pop() || url.hostname;
      } else {
        // For other URIs, try to extract the last part
        return uri.split('/').pop() || uri.split(':').pop() || uri;
      }
    } catch (error) {
      return uri;
    }
  }

  function getResourceIcon(resource: any) {
    const mimeType = resource.mimeType || '';
    if (mimeType.startsWith('text/') || mimeType === 'application/json') {
      return File;
    } else if (resource.uri?.includes('://')) {
      return ExternalLink;
    } else {
      return Database;
    }
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    uiStore.showSuccess('Copied to clipboard');
  }

  function formatResourceSize(resource: any) {
    // Placeholder - in real implementation would show actual size
    return 'Unknown size';
  }

  onMount(() => {
    if (selectedServerId) {
      loadResources();
    }
  });
</script>

<div class="h-full flex" style="background: var(--mcp-surface-secondary)">
  <!-- Left Panel: Resource List -->
  <div class="w-1/3 bg-white border-r border-gray-200 flex flex-col">
    <!-- Header -->
    <div class="p-4 border-b border-gray-200">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-lg font-semibold text-gray-900">Resources</h2>
        <button
          onclick={loadResources}
          disabled={loading || !selectedServerId}
          class="btn-secondary {loading ? 'opacity-50' : ''}"
        >
          <RefreshCw size={16} class={loading ? 'animate-spin' : ''} />
        </button>
      </div>

      <!-- Server Selection -->
      {#if servers.length > 1}
        <div class="mb-3">
          <label class="block text-xs font-medium text-gray-700 mb-1" for="resource-server-select">Server</label>
          <select
            bind:value={selectedServerId}
            onchange={(e) => selectServer(e.currentTarget.value)}
            class="form-select text-sm"
            id="resource-server-select"
          >
            {#each servers as server}
              <option value={server.id}>{server.config.name}</option>
            {/each}
          </select>
        </div>
      {/if}

      <!-- Search -->
      <div class="relative mb-3">
        <Search size={16} class="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400" />
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search resources..."
          class="form-input pl-10 text-sm"
        />
      </div>

      <!-- Filters -->
      <div class="flex items-center space-x-2">
        <Filter size={14} class="text-gray-400" />
        <select bind:value={filterType} class="form-select text-xs">
          <option value="all">All Types</option>
          <option value="text">Text</option>
          <option value="image">Images</option>
          <option value="document">Documents</option>
          <option value="data">Data</option>
        </select>
      </div>
    </div>

    <!-- Resource List -->
    <div class="flex-1 overflow-y-auto">
      {#if loading}
        <div class="flex items-center justify-center p-8">
          <RefreshCw size={24} class="animate-spin text-gray-400 mr-3" />
          <span class="text-gray-600">Loading resources...</span>
        </div>
      {:else if !selectedServerId}
        <div class="text-center p-8">
          <Database size={48} class="mx-auto text-gray-400 mb-4" />
          <h3 class="text-lg font-medium text-gray-900 mb-2">No Server Selected</h3>
          <p class="text-gray-600">Select a connected server to view resources</p>
        </div>
      {:else if filteredResources.length === 0}
        <div class="text-center p-8">
          <File size={48} class="mx-auto text-gray-400 mb-4" />
          <h3 class="text-lg font-medium text-gray-900 mb-2">No Resources Found</h3>
          <p class="text-gray-600">
            {#if console.log('🔍 UI DEBUG: Showing no resources. filteredResources.length:', filteredResources.length, 'resources.length:', resources.length)}<!-- -->{/if}
            {searchQuery ? 'No resources match your search' : 'This server has no available resources'}
          </p>
        </div>
      {:else}
        <div class="p-4 space-y-3">
          {#each filteredResources as resource}
            <button
              onclick={() => selectResource(resource)}
              class="w-full p-3 text-left bg-gray-50 hover:bg-gray-100 rounded-lg border border-gray-200 transition-colors
                     {selectedResource?.uri === resource.uri ? 'ring-2 ring-mcp-primary-500 bg-mcp-primary-50' : ''}"
            >
              <div class="flex items-start">
                {#if resource}
                  {@const IconComponent = getResourceIcon(resource)}
                  <IconComponent size={16} class="text-gray-500 mr-3 mt-0.5" />
                {/if}
                <div class="flex-1 min-w-0">
                  <h4 class="text-sm font-medium text-gray-900 truncate">
                    {resource.name || resource.uri}
                  </h4>
                  {#if resource.description}
                    <p class="text-xs text-gray-600 mt-1 line-clamp-2">
                      {resource.description}
                    </p>
                  {/if}
                  <div class="flex items-center space-x-2 mt-2">
                    <span class="text-xs text-gray-500 bg-gray-200 px-2 py-1 rounded">
                      {resource.mimeType || 'unknown'}
                    </span>
                    {#if resource.annotations}
                      {#each Object.entries(resource.annotations) as [key, value]}
                        <span class="text-xs text-blue-600 bg-blue-100 px-2 py-1 rounded">
                          {key}: {value}
                        </span>
                      {/each}
                    {/if}
                  </div>
                </div>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <!-- Right Panel: Resource Explorer -->
  <div class="flex-1 flex flex-col bg-white">
    {#if selectedResource}
      <!-- Header -->
      <div class="p-4 border-b border-gray-200">
        <div class="flex items-center justify-between">
          <div>
            <h3 class="text-lg font-semibold text-gray-900">
              {selectedResource.name || selectedResource.uri}
            </h3>
            {#if selectedResource.description}
              <p class="text-sm text-gray-600 mt-1">{selectedResource.description}</p>
            {/if}
          </div>

          <div class="flex items-center space-x-2">
            <button
              onclick={() => copyToClipboard(selectedResource.uri)}
              class="btn-secondary text-sm"
              title="Copy URI"
            >
              <Copy size={14} />
            </button>
            {#if !showParameterForm}
              <button
                onclick={() => loadResourceContent(selectedResource)}
                class="btn-primary text-sm"
              >
                <Play size={14} class="{contentLoading ? 'animate-pulse' : ''} mr-1" />
                {contentLoading ? 'Loading...' : 'Load'}
              </button>
            {/if}
          </div>
        </div>
      </div>

      <div class="flex-1 overflow-hidden">
        <div class="h-full flex flex-col">
          <!-- Content Area -->
          <div class="flex-1 flex">
            <!-- Parameters Panel (if needed) -->
            {#if showParameterForm}
              <div class="w-1/2 border-r border-gray-200 p-4 overflow-y-auto">
                <h4 class="text-sm font-medium text-gray-900 mb-4">Parameters</h4>

                <div class="space-y-4">
                  <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">
                      {parameterName}
                      <span class="text-red-500">*</span>
                    </label>

                    {#if parameterOptions.length > 0}
                      <select
                        bind:value={resourceParameters[parameterName]}
                        class="form-select w-full"
                      >
                        <option value="">-- Select {parameterName} --</option>
                        {#each parameterOptions as option}
                          <option value={option}>{option}</option>
                        {/each}
                      </select>
                    {:else}
                      <input
                        type="text"
                        bind:value={resourceParameters[parameterName]}
                        placeholder="Enter {parameterName}..."
                        class="form-input w-full"
                      />
                    {/if}
                  </div>

                  <button
                    onclick={executeResourceLoad}
                    disabled={contentLoading || !resourceParameters[parameterName]}
                    class="btn-primary w-full {contentLoading || !resourceParameters[parameterName] ? 'opacity-50 cursor-not-allowed' : ''}"
                  >
                    <Play size={14} class="{contentLoading ? 'animate-pulse' : ''} mr-1" />
                    {contentLoading ? 'Loading...' : 'Load Resource'}
                  </button>
                </div>
              </div>
            {/if}

            <!-- Results Panel -->
            <div class="{showParameterForm ? 'w-1/2' : 'w-full'} p-4">
              {#if contentLoading}
                <div class="flex items-center justify-center h-full">
                  <RefreshCw size={24} class="animate-spin text-gray-400 mr-3" />
                  <span class="text-gray-600">Loading content...</span>
                </div>
              {:else if resourceContent}
                <div class="h-full">
                  <div class="flex items-center justify-between mb-3">
                    <div class="flex items-center space-x-2">
                      <h4 class="text-sm font-medium text-gray-900">Content</h4>
                      {#if isHistoricalResult}
                        <span class="inline-flex items-center px-2 py-1 rounded text-xs font-medium bg-blue-100 text-blue-800">
                          <History size={12} class="mr-1" />
                          Historical Result
                        </span>
                      {/if}
                    </div>
                    <button
                      onclick={() => copyToClipboard(resourceContent)}
                      class="btn-secondary text-sm"
                    >
                      <Copy size={14} class="mr-1" />
                      Copy
                    </button>
                  </div>

                  <div class="bg-gray-50 rounded-lg p-4 h-full overflow-auto font-mono text-sm">
                    <pre class="whitespace-pre-wrap">{resourceContent}</pre>
                  </div>
                </div>
              {:else}
                <div class="flex items-center justify-center h-full text-gray-500">
                  <Eye size={24} class="mr-2" />
                  {showParameterForm ? 'Configure parameters and load' : 'Click Load to view resource data'}
                </div>
              {/if}
            </div>
          </div>

          <!-- Execution History - spans full width -->
          {#if executionHistory.length > 0}
            {@const resourceExecutions = executionHistory.filter(e => e.resourceUri === selectedResource.uri ||
              (e.parameters && selectedResource.uri.includes('{') &&
               e.resourceUri.replace(/\{[^}]+\}/g, '').includes(selectedResource.uri.replace(/\{[^}]+\}/g, ''))))}

            {#if resourceExecutions.length > 0}
              <div class="border-t border-gray-200 p-4 max-h-64 overflow-y-auto">
                <div class="flex items-center space-x-2 mb-3">
                  <History size={16} class="text-gray-500" />
                  <h4 class="text-sm font-medium text-gray-900">Execution History</h4>
                  <span class="text-xs text-gray-500">({resourceExecutions.length})</span>
                </div>

                <div class="space-y-2">
                  {#each resourceExecutions as execution}
                    <button
                      onclick={() => rerunResourceExecution(execution)}
                      class="w-full p-2 text-left bg-gray-50 hover:bg-gray-100 rounded border border-gray-200 transition-colors"
                    >
                      <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-2">
                          {#if execution.status === 'success'}
                            <CheckCircle size={12} class="text-green-500" />
                          {:else}
                            <AlertCircle size={12} class="text-red-500" />
                          {/if}
                          <span class="text-xs font-medium text-gray-900">
                            {execution.parameters ?
                              Object.entries(execution.parameters).map(([k,v]) => `${k}=${v}`).join(', ') :
                              'Direct load'
                            }
                          </span>
                        </div>
                        <div class="text-xs text-gray-500">
                          {execution.duration ? `${execution.duration}ms` : ''}
                        </div>
                      </div>
                      <div class="text-xs text-gray-600 mt-1">
                        {new Date(execution.timestamp).toLocaleString()}
                      </div>
                      {#if execution.error}
                        <div class="text-xs text-red-600 mt-1 truncate">
                          Error: {execution.error}
                        </div>
                      {/if}
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          {/if}
        </div>
      </div>
    {:else}
      <!-- No Resource Selected -->
      <div class="flex items-center justify-center h-full">
        <div class="text-center">
          <File size={48} class="mx-auto text-gray-400 mb-4" />
          <h3 class="text-lg font-semibold text-gray-900 mb-2">Select a Resource</h3>
          <p class="text-gray-600">Choose a resource from the list to explore</p>
        </div>
      </div>
    {/if}
  </div>
</div>