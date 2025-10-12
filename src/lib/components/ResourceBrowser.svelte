<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import { createCapabilityStore } from '$lib/utils/serverStore';
  import { createLogger } from '$lib/utils/logger';

  // Initialize scoped logger
  const logger = createLogger('ResourceBrowser');

  import EmptyCapabilityState from '$lib/components/ui/EmptyCapabilityState.svelte';
  import JsonViewer from '$lib/components/ui/JsonViewer.svelte';


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

  // Type for capability store data
  interface CapabilityStoreData {
    servers: ServerInfo[];
    totalServers: number;
    connectedServers: number;
    capableServers: number;
    hasCapableServers: boolean;
    selectedServerId: string | undefined;
  }

  // Optimized reactive store for servers with resources capability
  const resourceServersStore = createCapabilityStore('resources');
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

  // ✅ FIXED: Use $derived instead of manual subscription to prevent infinite loops
  const serverData = $derived($resourceServersStore);
  const serverState = $derived($serverStore);
  const globalSelectedId = $derived(serverState.selectedServerId);

  // ✅ FIXED: Derive server list reactively
  const serverList = $derived(
    serverData?.servers instanceof Map
      ? Array.from(serverData.servers.values())
      : (Array.isArray(serverData?.servers) ? serverData.servers : [])
  );

  // ✅ FIXED: Single effect for server selection logic with proper dependency tracking
  $effect(() => {
    // Only run logic if we have valid server data
    if (!serverData || !serverList) return;

    // Sync with global selection if it's a valid resources server
    if (globalSelectedId && serverList.some(s => s.id === globalSelectedId)) {
      if (selectedServerId !== globalSelectedId) {
        selectedServerId = globalSelectedId;
        loadResources(); // Safe: next effect run will early-return due to equality check above
      }
      return; // Exit early if synced with global
    }

    // Auto-manage selection: pick first server if none selected or current is invalid
    const currentServerValid = selectedServerId && serverList.find(s => s.id === selectedServerId);

    if (!currentServerValid && serverList.length > 0) {
      selectedServerId = serverList[0].id;
      if (selectedServerId) {
        loadResources();
      }
    }
  });

  const filteredResources = $derived.by(() => {
    let filtered = resources;

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(resource =>
        resource.uri?.toLowerCase().includes(query) ||
        resource.name?.toLowerCase().includes(query) ||
        resource.description?.toLowerCase().includes(query)
      );
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
    }

    return filtered;
  });

  async function loadResources() {
    if (!selectedServerId) return;

    loading = true;
    try {
      // Check if server supports resources capability
      const serverInfo = await invoke<ServerInfo>('get_server_info', { serverId: selectedServerId });

      if (!serverInfo.capabilities?.resources) {
        resources = [];
        uiStore.showInfo('This server does not support resources operations. Try the Tools tab instead.');
        return;
      }

      const resourceList = await serverStore.listResources(selectedServerId);

      resources = resourceList.map((resource: any) => {
        return {
          uri: resource.uri,
          name: resource.name || extractNameFromUri(resource.uri),
          description: resource.description,
          mimeType: resource.mimeType || resource.mime_type || 'application/octet-stream',
          annotations: resource.annotations || {}
        };
      });

      uiStore.showSuccess(`Loaded ${resources.length} resources`);
    } catch (error) {
      logger.error('Failed to load resources:', error);
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

      // Generate intelligent parameter suggestions based on parameter name and context
      const parameterSuggestions = generateParameterSuggestions(parameterName, resource);
      const knownValues: { [key: string]: string[] } = {
        [parameterName]: parameterSuggestions
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
    const serverInfo = serverData?.servers?.find(s => s.id === selectedServerId);

    contentLoading = true;
    resourceContent = '';
    isHistoricalResult = false;

    try {
      // Substitute parameters in URI
      let actualUri = selectedResource.uri.replace(`{${parameterName}}`, resourceParameters[parameterName]);

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
      logger.error('Failed to load resource content:', error);
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
    const serverInfo = serverData?.servers?.find(s => s.id === selectedServerId);

    contentLoading = true;
    resourceContent = '';
    isHistoricalResult = false;

    try {
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
      logger.error('Failed to load resource content:', error);
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
    selectedServerId = serverId;
    // Update global store so other components stay in sync
    serverStore.selectServer(serverId);
    loadResources();
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

  function tryParseJson(str: string): { isJson: boolean; data: any } {
    try {
      const parsed = JSON.parse(str);
      return { isJson: true, data: parsed };
    } catch {
      return { isJson: false, data: str };
    }
  }

  function formatResourceSize(resource: any) {
    // Try to get size from resource metadata or make an intelligent estimate
    if (resource.size !== undefined && resource.size !== null) {
      return formatBytes(resource.size);
    }

    // For different resource types, provide intelligent defaults
    const uri = resource.uri || '';

    if (uri.startsWith('file://')) {
      return 'File'; // File resources will be read to determine size
    } else if (uri.startsWith('http://') || uri.startsWith('https://')) {
      return 'Remote'; // Remote resources size unknown until fetched
    } else if (uri.includes('database') || uri.includes('sql')) {
      return 'Query result'; // Database resources are dynamic
    } else if (uri.includes('template') || resource.mimeType?.includes('template')) {
      return 'Template'; // Template resources are dynamic
    }

    return 'Dynamic'; // Default for other dynamic resources
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function generateParameterSuggestions(paramName: string, resource: any): string[] {
    const uri = resource.uri || '';
    const description = (resource.description || '').toLowerCase();
    const name = (resource.name || '').toLowerCase();

    // Generate suggestions based on parameter name patterns
    const suggestions: string[] = [];

    switch (paramName.toLowerCase()) {
      case 'name':
        suggestions.push('readme', 'guide', 'api', 'config', 'documentation', 'example', 'tutorial');
        break;
      case 'template':
      case 'type':
        suggestions.push('code_review', 'documentation', 'bug_report', 'feature_request', 'meeting_notes');
        break;
      case 'id':
      case 'identifier':
        suggestions.push('user_001', 'item_123', 'project_abc', 'session_xyz');
        break;
      case 'path':
      case 'file':
      case 'filename':
        suggestions.push('/README.md', '/src/main.js', '/docs/api.md', '/config/settings.json');
        break;
      case 'query':
      case 'search':
        suggestions.push('SELECT * FROM users', 'search term', 'filter criteria');
        break;
      case 'format':
        suggestions.push('json', 'xml', 'csv', 'yaml', 'markdown');
        break;
      case 'language':
      case 'lang':
        suggestions.push('javascript', 'python', 'rust', 'typescript', 'go');
        break;
      case 'version':
        suggestions.push('1.0.0', 'latest', 'beta', 'dev');
        break;
      default:
        // Generate contextual suggestions based on URI and description
        if (uri.includes('file') || description.includes('file')) {
          suggestions.push('config.json', 'data.csv', 'readme.md', 'example.txt');
        } else if (uri.includes('database') || uri.includes('sql') || description.includes('database')) {
          suggestions.push('users', 'orders', 'products', 'sessions');
        } else if (uri.includes('api') || description.includes('api')) {
          suggestions.push('v1', 'v2', 'latest', 'beta');
        } else {
          // Generic suggestions
          suggestions.push('example', 'test', 'sample', 'default');
        }
        break;
    }

    return suggestions;
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
      {#if (serverData?.servers || []).length > 1}
        <div class="mb-3">
          <label class="block text-xs font-medium text-gray-700 mb-1" for="resource-server-select">Server</label>
          <select
            bind:value={selectedServerId}
            onchange={(e) => selectServer(e.currentTarget.value)}
            class="form-select text-sm"
            id="resource-server-select"
          >
            {#each serverData?.servers || [] as server}
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
          class="form-input has-icon-left text-sm"
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
                {@const contentParsed = tryParseJson(resourceContent)}
                <div class="h-full">
                  <div class="flex items-center justify-between mb-3">
                    <div class="flex items-center space-x-2">
                      <h4 class="text-sm font-medium text-gray-900 dark:text-gray-100">Content</h4>
                      {#if isHistoricalResult}
                        <span class="inline-flex items-center px-2 py-1 rounded text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                          <History size={12} class="mr-1" />
                          Historical Result
                        </span>
                      {/if}
                      {#if contentParsed.isJson}
                        <span class="inline-flex items-center px-2 py-1 rounded text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">
                          JSON
                        </span>
                      {/if}
                    </div>
                    {#if !contentParsed.isJson}
                      <button
                        onclick={() => copyToClipboard(resourceContent)}
                        class="btn-secondary text-sm"
                      >
                        <Copy size={14} class="mr-1" />
                        Copy
                      </button>
                    {/if}
                  </div>

                  <div class="bg-gray-50 dark:bg-gray-800 rounded-lg p-4 h-full overflow-auto">
                    {#if contentParsed.isJson}
                      <JsonViewer
                        data={contentParsed.data}
                        expanded={true}
                        showCopy={true}
                        showSearch={true}
                        title="Resource Content"
                      />
                    {:else}
                      <pre class="whitespace-pre-wrap font-mono text-sm text-gray-900 dark:text-gray-100">{resourceContent}</pre>
                    {/if}
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
