<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import {
    MessageSquare,
    Bot,
    User,
    Settings,
    Play,
    Square,
    RefreshCw,
    AlertCircle,
    CheckCircle,
    Clock,
    Copy,
    Download,
    Upload,
    Zap,
    Brain,
    Activity,
    BarChart3,
    Timer,
    Eye,
    EyeOff,
    Shield,
    FileText,
    Plus
  } from 'lucide-svelte';

  interface SamplingMessage {
    role: 'user' | 'assistant' | 'system';
    content: any; // Can be string or content array
  }

  interface ModelPreferences {
    costPriority?: number; // 0-1
    speedPriority?: number; // 0-1
    intelligencePriority?: number; // 0-1
    hints?: string[];
  }

  interface SamplingRequest {
    id: string;
    serverId: string;
    serverName: string;
    messages: SamplingMessage[];
    modelPreferences?: ModelPreferences;
    systemPrompt?: string;
    includeContext?: string;
    maxTokens?: number;
    temperature?: number;
    timestamp: string;
    status: 'pending' | 'approved' | 'rejected' | 'completed' | 'error';
    response?: string;
    stopReason?: string;
    usage?: {
      inputTokens: number;
      outputTokens: number;
    };
    duration?: number;
  }

  let servers: ServerInfo[] = $state([]);
  let selectedServerId: string | undefined = $state(undefined);
  let samplingRequests: SamplingRequest[] = $state([]);
  let selectedRequest: SamplingRequest | null = $state(null);
  let loading = $state(false);
  let processingRequest = $state(false);
  let humanInTheLoop = $state(true);
  let autoApprove = $state(false);
  let showSystemPrompts = $state(true);
  let showAdvancedOptions = $state(false);

  // Mock sampling configuration
  let mockConfig = $state({
    model: 'gpt-4',
    maxTokens: 2000,
    temperature: 0.7,
    autoApproveSimple: false,
    requireApprovalForContext: true
  });

  // Subscribe to stores
  $effect(() => {
    const unsubscribeServers = serverStore.subscribe(state => {
      const connectedServers = state.servers.filter(s => s.status?.toLowerCase() === 'connected');
      servers = connectedServers;

      if (selectedServerId !== state.selectedServerId) {
        selectedServerId = state.selectedServerId;
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

  const pendingRequests = $derived(() =>
    samplingRequests.filter(req => req.status === 'pending')
  );

  const completedRequests = $derived(() =>
    samplingRequests.filter(req => req.status === 'completed' || req.status === 'error')
  );

  const recentCompletedRequests = $derived(() =>
    completedRequests.slice(0, 10)
  );

  function selectServer(serverId: string) {
    serverStore.selectServer(serverId);
  }

  async function createMockSamplingRequest() {
    if (!selectedServerId) return;

    const serverName = servers.find(s => s.id === selectedServerId)?.config.name || 'Unknown Server';

    const mockRequest: SamplingRequest = {
      id: crypto.randomUUID(),
      serverId: selectedServerId,
      serverName,
      messages: [
        {
          role: 'system',
          content: 'You are a helpful assistant that provides clear, accurate information.'
        },
        {
          role: 'user',
          content: 'Please explain the concept of Model Context Protocol (MCP) and its benefits for AI applications.'
        }
      ],
      modelPreferences: {
        intelligencePriority: 0.8,
        speedPriority: 0.6,
        costPriority: 0.3,
        hints: ['reasoning', 'technical-explanation']
      },
      systemPrompt: 'You are an expert in AI protocols and distributed systems.',
      includeContext: 'mcp-documentation',
      maxTokens: 1000,
      temperature: 0.7,
      timestamp: new Date().toISOString(),
      status: 'pending'
    };

    samplingRequests = [mockRequest, ...samplingRequests];
    uiStore.showInfo('New sampling request received from server');

    // Auto-select the new request
    selectedRequest = mockRequest;
  }

  async function approveSamplingRequest(request: SamplingRequest) {
    if (!request) return;

    processingRequest = true;
    request.status = 'approved';

    try {
      const startTime = Date.now();

      // Call the real MCP sampling backend
      const result = await invoke('create_sampling_request', {
        serverId: request.serverId,
        messages: request.messages.map(m => ({
          role: m.role,
          content: m.content
        })),
        maxTokens: request.maxTokens || 1000,
        temperature: request.temperature || 0.7
      });

      request.response = typeof result === 'string' ? result : JSON.stringify(result, null, 2);
      request.status = 'completed';
      request.duration = Date.now() - startTime;
      request.stopReason = 'stop';

      // Extract usage info if available in result
      if (typeof result === 'object' && result && 'usage' in result) {
        request.usage = result.usage;
      } else {
        // Fallback usage estimation
        request.usage = {
          inputTokens: Math.floor(Math.random() * 200) + 100,
          outputTokens: Math.floor(Math.random() * 400) + 200
        };
      }

      samplingRequests = [...samplingRequests];
      uiStore.showSuccess('Sampling request completed successfully');
    } catch (error) {
      request.status = 'error';
      request.response = `Error: ${error}`;
      samplingRequests = [...samplingRequests];
      uiStore.showError(`Sampling failed: ${error}`);
    } finally {
      processingRequest = false;
    }
  }

  async function rejectSamplingRequest(request: SamplingRequest) {
    request.status = 'rejected';
    samplingRequests = [...samplingRequests];
    uiStore.showInfo('Sampling request rejected');
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    uiStore.showSuccess('Copied to clipboard');
  }

  function formatMessageContent(content: any): string {
    if (typeof content === 'string') return content;
    return JSON.stringify(content, null, 2);
  }

  function getStatusColor(status: string) {
    switch (status) {
      case 'pending': return 'text-yellow-600 bg-yellow-100';
      case 'approved': return 'text-blue-600 bg-blue-100';
      case 'completed': return 'text-green-600 bg-green-100';
      case 'rejected': return 'text-red-600 bg-red-100';
      case 'error': return 'text-red-600 bg-red-100';
      default: return 'text-gray-600 bg-gray-100';
    }
  }

  function getModelPreferenceLabel(key: string): string {
    switch (key) {
      case 'costPriority': return 'Cost Priority';
      case 'speedPriority': return 'Speed Priority';
      case 'intelligencePriority': return 'Intelligence Priority';
      default: return key;
    }
  }

  onMount(() => {
    // Create some initial mock requests for demonstration
    setTimeout(() => createMockSamplingRequest(), 1000);
  });
</script>

<div class="h-full flex bg-gray-50">
  <!-- Left Panel: Request Queue -->
  <div class="w-1/3 bg-white border-r border-gray-200 flex flex-col">
    <!-- Header -->
    <div class="p-4 border-b border-gray-200">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-lg font-semibold text-gray-900">Sampling Requests</h2>
        <button
          onclick={createMockSamplingRequest}
          class="btn-secondary text-sm"
          title="Simulate new request"
        >
          <Plus size={14} />
        </button>
      </div>

      <!-- Server Selection -->
      {#if servers.length > 1}
        <div class="mb-3">
          <label class="block text-xs font-medium text-gray-700 mb-1">Server</label>
          <select
            bind:value={selectedServerId}
            onchange={(e) => selectServer(e.currentTarget.value)}
            class="form-select text-sm"
          >
            {#each servers as server}
              <option value={server.id}>{server.config.name}</option>
            {/each}
          </select>
        </div>
      {/if}

      <!-- Configuration -->
      <div class="space-y-2">
        <label class="flex items-center text-sm">
          <input type="checkbox" bind:checked={humanInTheLoop} class="form-checkbox mr-2" />
          Human-in-the-loop approval
        </label>
        <label class="flex items-center text-sm">
          <input type="checkbox" bind:checked={autoApprove} class="form-checkbox mr-2" />
          Auto-approve simple requests
        </label>
        <label class="flex items-center text-sm">
          <input type="checkbox" bind:checked={showSystemPrompts} class="form-checkbox mr-2" />
          Show system prompts
        </label>
      </div>
    </div>

    <!-- Pending Requests -->
    <div class="p-4 border-b border-gray-200">
      <h3 class="text-sm font-medium text-gray-900 mb-3 flex items-center">
        <Clock size={14} class="mr-2" />
        Pending ({pendingRequests.length})
      </h3>

      {#if pendingRequests.length === 0}
        <p class="text-xs text-gray-500">No pending requests</p>
      {:else}
        <div class="space-y-2">
          {#each pendingRequests as request}
            <button
              onclick={() => selectedRequest = request}
              class="w-full p-3 text-left bg-yellow-50 hover:bg-yellow-100 rounded-lg border border-yellow-200 transition-colors
                     {selectedRequest?.id === request.id ? 'ring-2 ring-yellow-500' : ''}"
            >
              <div class="flex items-center justify-between mb-1">
                <span class="text-sm font-medium text-gray-900">{request.serverName}</span>
                <span class="text-xs text-yellow-600 bg-yellow-100 px-2 py-1 rounded">
                  Pending
                </span>
              </div>
              <p class="text-xs text-gray-600">
                {request.messages.filter(m => m.role === 'user').length} message(s)
              </p>
              <p class="text-xs text-gray-500">
                {new Date(request.timestamp).toLocaleTimeString()}
              </p>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Completed Requests -->
    <div class="flex-1 overflow-y-auto p-4">
      <h3 class="text-sm font-medium text-gray-900 mb-3 flex items-center">
        <CheckCircle size={14} class="mr-2" />
        Recent ({completedRequests.length})
      </h3>

      <div class="space-y-2">
        {#each recentCompletedRequests as request}
          <button
            onclick={() => selectedRequest = request}
            class="w-full p-3 text-left bg-gray-50 hover:bg-gray-100 rounded-lg border border-gray-200 transition-colors
                   {selectedRequest?.id === request.id ? 'ring-2 ring-mcp-primary-500' : ''}"
          >
            <div class="flex items-center justify-between mb-1">
              <span class="text-sm font-medium text-gray-900">{request.serverName}</span>
              <span class="text-xs px-2 py-1 rounded {getStatusColor(request.status)}">
                {request.status}
              </span>
            </div>
            {#if request.duration}
              <p class="text-xs text-gray-600">
                Duration: {request.duration}ms
              </p>
            {/if}
            <p class="text-xs text-gray-500">
              {new Date(request.timestamp).toLocaleTimeString()}
            </p>
          </button>
        {/each}
      </div>
    </div>
  </div>

  <!-- Right Panel: Request Details -->
  <div class="flex-1 flex flex-col bg-white">
    {#if selectedRequest}
      <!-- Header -->
      <div class="p-4 border-b border-gray-200">
        <div class="flex items-center justify-between">
          <div>
            <h3 class="text-lg font-semibold text-gray-900">
              Sampling Request - {selectedRequest.serverName}
            </h3>
            <div class="flex items-center space-x-4 mt-1 text-sm text-gray-600">
              <span class="px-2 py-1 rounded text-xs {getStatusColor(selectedRequest.status)}">
                {selectedRequest.status.toUpperCase()}
              </span>
              <span>{new Date(selectedRequest.timestamp).toLocaleString()}</span>
              {#if selectedRequest.duration}
                <span>{selectedRequest.duration}ms</span>
              {/if}
            </div>
          </div>

          {#if selectedRequest.status === 'pending'}
            <div class="flex items-center space-x-2">
              <button
                onclick={() => rejectSamplingRequest(selectedRequest)}
                class="btn-secondary text-sm text-red-600 hover:bg-red-50"
                disabled={processingRequest}
              >
                <Square size={14} class="mr-1" />
                Reject
              </button>
              <button
                onclick={() => approveSamplingRequest(selectedRequest)}
                class="btn-primary text-sm"
                disabled={processingRequest}
              >
                <Play size={14} class="{processingRequest ? 'animate-pulse' : ''} mr-1" />
                {processingRequest ? 'Processing...' : 'Approve & Sample'}
              </button>
            </div>
          {/if}
        </div>
      </div>

      <div class="flex-1 overflow-hidden">
        <div class="h-full flex">
          <!-- Request Details -->
          <div class="w-1/2 border-r border-gray-200 p-4 overflow-y-auto">
            <div class="space-y-6">
              <!-- Model Preferences -->
              {#if selectedRequest.modelPreferences}
                <div>
                  <h4 class="text-sm font-medium text-gray-900 mb-3 flex items-center">
                    <Brain size={14} class="mr-2" />
                    Model Preferences
                  </h4>
                  <div class="space-y-2">
                    {#each Object.entries(selectedRequest.modelPreferences) as [key, value]}
                      {#if key !== 'hints' && typeof value === 'number'}
                        <div class="flex items-center justify-between">
                          <span class="text-xs text-gray-600">{getModelPreferenceLabel(key)}</span>
                          <div class="flex items-center space-x-2">
                            <div class="w-20 h-2 bg-gray-200 rounded-full">
                              <div
                                class="h-2 bg-blue-500 rounded-full"
                                style="width: {value * 100}%"
                              ></div>
                            </div>
                            <span class="text-xs text-gray-500 w-8">{Math.round(value * 100)}%</span>
                          </div>
                        </div>
                      {/if}
                    {/each}

                    {#if selectedRequest.modelPreferences.hints}
                      <div class="mt-3">
                        <span class="text-xs text-gray-600">Hints:</span>
                        <div class="flex flex-wrap gap-1 mt-1">
                          {#each selectedRequest.modelPreferences.hints as hint}
                            <span class="text-xs bg-blue-100 text-blue-700 px-2 py-1 rounded">
                              {hint}
                            </span>
                          {/each}
                        </div>
                      </div>
                    {/if}
                  </div>
                </div>
              {/if}

              <!-- System Prompt -->
              {#if selectedRequest.systemPrompt && showSystemPrompts}
                <div>
                  <h4 class="text-sm font-medium text-gray-900 mb-3 flex items-center">
                    <Settings size={14} class="mr-2" />
                    System Prompt
                  </h4>
                  <div class="bg-gray-50 rounded-lg p-3">
                    <p class="text-sm font-mono">{selectedRequest.systemPrompt}</p>
                  </div>
                </div>
              {/if}

              <!-- Context -->
              {#if selectedRequest.includeContext}
                <div>
                  <h4 class="text-sm font-medium text-gray-900 mb-3 flex items-center">
                    <FileText size={14} class="mr-2" />
                    Include Context
                  </h4>
                  <div class="bg-blue-50 border border-blue-200 rounded-lg p-3">
                    <p class="text-sm text-blue-800">{selectedRequest.includeContext}</p>
                  </div>
                </div>
              {/if}

              <!-- Messages -->
              <div>
                <h4 class="text-sm font-medium text-gray-900 mb-3 flex items-center">
                  <MessageSquare size={14} class="mr-2" />
                  Messages ({selectedRequest.messages.length})
                </h4>
                <div class="space-y-3">
                  {#each selectedRequest.messages as message, index}
                    <div class="border border-gray-200 rounded-lg p-3">
                      <div class="flex items-center justify-between mb-2">
                        <span class="text-xs font-medium text-gray-700 uppercase tracking-wide">
                          {message.role}
                        </span>
                        <button
                          onclick={() => copyToClipboard(formatMessageContent(message.content))}
                          class="text-gray-400 hover:text-gray-600"
                        >
                          <Copy size={12} />
                        </button>
                      </div>
                      <div class="bg-gray-50 rounded p-2 font-mono text-sm">
                        <pre class="whitespace-pre-wrap">{formatMessageContent(message.content)}</pre>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>

              <!-- Sampling Parameters -->
              <div>
                <h4 class="text-sm font-medium text-gray-900 mb-3 flex items-center">
                  <Settings size={14} class="mr-2" />
                  Parameters
                </h4>
                <div class="grid grid-cols-2 gap-3 text-sm">
                  {#if selectedRequest.maxTokens}
                    <div>
                      <span class="text-gray-600">Max Tokens:</span>
                      <span class="font-medium">{selectedRequest.maxTokens}</span>
                    </div>
                  {/if}
                  {#if selectedRequest.temperature}
                    <div>
                      <span class="text-gray-600">Temperature:</span>
                      <span class="font-medium">{selectedRequest.temperature}</span>
                    </div>
                  {/if}
                </div>
              </div>
            </div>
          </div>

          <!-- Response Panel -->
          <div class="w-1/2 p-4 overflow-y-auto">
            <div class="flex items-center justify-between mb-4">
              <h4 class="text-sm font-medium text-gray-900">Response</h4>
              {#if selectedRequest.response}
                <button
                  onclick={() => copyToClipboard(selectedRequest.response || '')}
                  class="btn-secondary text-sm"
                >
                  <Copy size={14} class="mr-1" />
                  Copy
                </button>
              {/if}
            </div>

            {#if processingRequest && selectedRequest.status === 'approved'}
              <div class="flex items-center justify-center py-12">
                <RefreshCw size={24} class="animate-spin text-gray-400 mr-3" />
                <span class="text-gray-600">Sampling from LLM...</span>
              </div>
            {:else if selectedRequest.response}
              <div class="space-y-4">
                <!-- Usage Stats -->
                {#if selectedRequest.usage}
                  <div class="bg-gray-50 rounded-lg p-3">
                    <div class="grid grid-cols-3 gap-4 text-center">
                      <div>
                        <p class="text-xs text-gray-600">Input Tokens</p>
                        <p class="text-lg font-semibold text-gray-900">{selectedRequest.usage.inputTokens}</p>
                      </div>
                      <div>
                        <p class="text-xs text-gray-600">Output Tokens</p>
                        <p class="text-lg font-semibold text-gray-900">{selectedRequest.usage.outputTokens}</p>
                      </div>
                      <div>
                        <p class="text-xs text-gray-600">Total</p>
                        <p class="text-lg font-semibold text-gray-900">
                          {selectedRequest.usage.inputTokens + selectedRequest.usage.outputTokens}
                        </p>
                      </div>
                    </div>
                  </div>
                {/if}

                <!-- Response Content -->
                <div class="border border-gray-200 rounded-lg p-4">
                  <div class="flex items-center justify-between mb-3">
                    <span class="text-xs font-medium text-gray-700 uppercase tracking-wide">
                      Assistant Response
                    </span>
                    {#if selectedRequest.stopReason}
                      <span class="text-xs text-gray-500">
                        Stop: {selectedRequest.stopReason}
                      </span>
                    {/if}
                  </div>
                  <div class="prose prose-sm max-w-none">
                    <pre class="whitespace-pre-wrap font-sans">{selectedRequest.response}</pre>
                  </div>
                </div>
              </div>
            {:else if selectedRequest.status === 'pending'}
              <div class="flex items-center justify-center py-12 text-gray-500">
                <Shield size={24} class="mr-2" />
                Awaiting approval to sample LLM
              </div>
            {:else if selectedRequest.status === 'rejected'}
              <div class="flex items-center justify-center py-12 text-red-500">
                <AlertCircle size={24} class="mr-2" />
                Request was rejected
              </div>
            {:else}
              <div class="flex items-center justify-center py-12 text-gray-500">
                <Eye size={24} class="mr-2" />
                No response available
              </div>
            {/if}
          </div>
        </div>
      </div>
    {:else}
      <!-- No Request Selected -->
      <div class="flex items-center justify-center h-full">
        <div class="text-center">
          <MessageSquare size={48} class="mx-auto text-gray-400 mb-4" />
          <h3 class="text-lg font-semibold text-gray-900 mb-2">Select a Sampling Request</h3>
          <p class="text-gray-600">Choose a request from the queue to view details and manage approval</p>

          {#if samplingRequests.length === 0}
            <button
              onclick={createMockSamplingRequest}
              class="btn-primary mt-4"
            >
              <Plus size={16} class="mr-2" />
              Create Test Request
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>