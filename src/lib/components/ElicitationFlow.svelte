<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import { filterServersByCapability } from '$lib/utils/serverCapabilities';
  import {
    MessageCircle,
    ArrowRight,
    ArrowLeft,
    Clock,
    CheckCircle,
    AlertCircle,
    XCircle,
    User,
    Server,
    Play,
    Pause,
    RotateCcw,
    Send,
    Eye,
    EyeOff,
    Settings,
    MessageSquare,
    HelpCircle,
    Check,
    X,
    Plus,
    Edit,
    Copy,
    Download,
    RefreshCw
  } from 'lucide-svelte';

  interface ElicitationRequest {
    id: string;
    serverId: string;
    serverName: string;
    type: 'confirmation' | 'input' | 'choice' | 'form';
    title: string;
    message: string;
    timestamp: string;
    status: 'pending' | 'responded' | 'timeout' | 'cancelled';
    priority: 'low' | 'normal' | 'high' | 'urgent';
    timeout?: number; // seconds
    context?: string;
    // Type-specific fields
    confirmationOptions?: { confirm: string; cancel: string };
    inputPrompt?: { placeholder?: string; multiline?: boolean; validation?: string };
    choices?: Array<{ id: string; label: string; description?: string }>;
    formFields?: Array<{
      name: string;
      label: string;
      type: 'text' | 'email' | 'number' | 'select' | 'textarea' | 'checkbox';
      required?: boolean;
      options?: string[];
      placeholder?: string;
    }>;
    // Response
    response?: any;
    respondedAt?: string;
  }

  interface ElicitationFlow {
    id: string;
    serverId: string;
    serverName: string;
    title: string;
    description: string;
    requests: ElicitationRequest[];
    currentRequestIndex: number;
    status: 'active' | 'completed' | 'failed' | 'cancelled';
    startedAt: string;
    completedAt?: string;
  }

  let servers: ServerInfo[] = $state([]);
  let selectedServerId: string | undefined = $state(undefined);
  let elicitationFlows: ElicitationFlow[] = $state([]);
  let pendingRequests: ElicitationRequest[] = $state([]);
  let selectedFlow: ElicitationFlow | null = $state(null);
  let selectedRequest: ElicitationRequest | null = $state(null);
  let loading = $state(false);
  let autoRespond = $state(false);
  let showCompleted = $state(false);

  // Form state for responses
  let responseData = $state<Record<string, any>>({});

  // Subscribe to stores
  $effect(() => {
    const unsubscribeServers = serverStore.subscribe((state: any) => {
      // Filter to only show connected servers that support elicitation
      const connectedServers = filterServersByCapability(state.servers, 'elicitation');
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

  const activeFlows = $derived.by(() =>
    elicitationFlows.filter(flow => flow.status === 'active')
  );

  const completedFlows = $derived.by(() =>
    elicitationFlows.filter(flow => flow.status === 'completed' || flow.status === 'failed' || flow.status === 'cancelled')
  );

  function selectServer(serverId: string) {
    serverStore.selectServer(serverId);
  }

  async function loadElicitationRequests() {
    if (!selectedServerId) return;

    loading = true;
    try {
      // Get elicitation requests from the MCP server
      const requests = await invoke<any[]>('get_elicitation_requests', {
        serverId: selectedServerId
      });

      // Convert the raw requests to our UI format
      const convertedRequests: ElicitationRequest[] = requests.map((req: any) => ({
        id: req.id || crypto.randomUUID(),
        serverId: selectedServerId!,  // Safe due to guard above
        serverName: servers.find(s => s.id === selectedServerId)?.config.name || 'Unknown Server',
        type: req.request_type || 'confirmation',
        title: req.title || 'Server Request',
        message: req.message || req.description || 'Server is requesting user input',
        timestamp: req.timestamp || new Date().toISOString(),
        status: 'pending' as const,
        priority: req.priority || 'normal',
        timeout: req.timeout,
        context: req.context,
        // Handle different request types
        confirmationOptions: req.confirmation_options,
        inputPrompt: req.input_prompt,
        choices: req.choices,
        formFields: req.form_fields,
      }));

      // Update our state with real requests
      pendingRequests = [...convertedRequests, ...pendingRequests];

      if (convertedRequests.length > 0) {
        uiStore.showInfo(`Loaded ${convertedRequests.length} real elicitation requests from server`);

        // Auto-select the first request
        selectedRequest = convertedRequests[0];
      } else {
        uiStore.showInfo('No pending elicitation requests from server');
      }
    } catch (error) {
      console.error('Failed to load elicitation requests:', error);
      uiStore.showError(`Failed to load elicitation requests: ${error}`);
    } finally {
      loading = false;
    }
  }

  async function respondToRequest(request: ElicitationRequest, response: any) {
    const originalRequest = findRequestById(request.id);
    if (!originalRequest) return;

    try {
      // Send response to server via backend
      await invoke('send_elicitation_response', {
        serverId: request.serverId,
        requestId: request.id,
        response: response
      });

      originalRequest.response = response;
      originalRequest.status = 'responded';
      originalRequest.respondedAt = new Date().toISOString();

      // Clear response data
      responseData = {};

      uiStore.showSuccess('Response sent to server');

      // Update flows and requests
      elicitationFlows = [...elicitationFlows];
      pendingRequests = [...pendingRequests];

      // Advance flow if this was part of a flow
      if (selectedFlow) {
        const requestIndex = selectedFlow.requests.findIndex(r => r.id === request.id);
        if (requestIndex !== -1) {
          selectedFlow.currentRequestIndex = Math.max(selectedFlow.currentRequestIndex, requestIndex + 1);

          // Check if flow is complete
          const allResponded = selectedFlow.requests.every(r => r.status === 'responded');
          if (allResponded) {
            selectedFlow.status = 'completed';
            selectedFlow.completedAt = new Date().toISOString();
            uiStore.showSuccess(`Flow "${selectedFlow.title}" completed`);
          } else {
            // Select next pending request in flow
            const nextRequest = selectedFlow.requests.find(r => r.status === 'pending');
            if (nextRequest) {
              selectedRequest = nextRequest;
            }
          }
        }
      }
    } catch (error) {
      console.error('Failed to send elicitation response:', error);
      uiStore.showError(`Failed to send response: ${error}`);
    }
  }

  function findRequestById(id: string): ElicitationRequest | null {
    // Check flows
    for (const flow of elicitationFlows) {
      const request = flow.requests.find(r => r.id === id);
      if (request) return request;
    }
    // Check standalone requests
    return pendingRequests.find(r => r.id === id) || null;
  }

  function cancelRequest(request: ElicitationRequest) {
    const originalRequest = findRequestById(request.id);
    if (!originalRequest) return;

    originalRequest.status = 'cancelled';
    elicitationFlows = [...elicitationFlows];
    pendingRequests = [...pendingRequests];

    uiStore.showInfo('Request cancelled');
  }

  function getPriorityColor(priority: string) {
    switch (priority) {
      case 'urgent': return 'text-red-600 dark:text-red-400 bg-red-100 dark:bg-red-900/30';
      case 'high': return 'text-orange-600 dark:text-orange-400 bg-orange-100 dark:bg-orange-900/30';
      case 'normal': return 'text-blue-600 dark:text-blue-400 bg-blue-100 dark:bg-blue-900/30';
      case 'low': return 'text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-800';
      default: return 'text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-800';
    }
  }

  function getStatusColor(status: string) {
    switch (status) {
      case 'pending': return 'text-yellow-600 dark:text-yellow-400 bg-yellow-100 dark:bg-yellow-900/30';
      case 'responded': return 'text-green-600 dark:text-green-400 bg-green-100 dark:bg-green-900/30';
      case 'timeout': return 'text-red-600 dark:text-red-400 bg-red-100 dark:bg-red-900/30';
      case 'cancelled': return 'text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-800';
      default: return 'text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-800';
    }
  }

  function formatResponse(response: any): string {
    if (typeof response === 'string') return response;
    if (typeof response === 'boolean') return response ? 'Yes' : 'No';
    return JSON.stringify(response, null, 2);
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    uiStore.showSuccess('Copied to clipboard');
  }

  onMount(() => {
    // Load elicitation requests on component mount
    if (selectedServerId) {
      setTimeout(() => loadElicitationRequests(), 500);
    }
  });
</script>

<div class="h-full flex bg-gray-50 dark:bg-gray-900">
  <!-- Left Panel: Request Queue -->
  <div class="w-1/3 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 flex flex-col">
    <!-- Header -->
    <div class="p-4 border-b border-gray-200 dark:border-gray-700">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">Elicitation</h2>
        <button
          onclick={loadElicitationRequests}
          class="btn-primary text-sm"
          title="Load real elicitation requests"
          disabled={loading}
        >
          {#if loading}
            <RefreshCw size={14} class="animate-spin" />
          {:else}
            <RefreshCw size={14} />
          {/if}
        </button>
      </div>

      <!-- Server Selection -->
      {#if servers.length > 1}
        <div class="mb-3">
          <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">Server</label>
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
        <label class="flex items-center text-sm text-gray-700 dark:text-gray-300">
          <input type="checkbox" bind:checked={autoRespond} class="form-checkbox mr-2" />
          Auto-respond to simple requests
        </label>
        <label class="flex items-center text-sm text-gray-700 dark:text-gray-300">
          <input type="checkbox" bind:checked={showCompleted} class="form-checkbox mr-2" />
          Show completed flows
        </label>
      </div>
    </div>

    <!-- Active Flows -->
    <div class="p-4 border-b border-gray-200 dark:border-gray-700">
      <h3 class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3 flex items-center">
        <MessageCircle size={14} class="mr-2" />
        Active Flows ({activeFlows.length})
      </h3>

      {#if activeFlows.length === 0}
        <p class="text-xs text-gray-500 dark:text-gray-400">No active flows</p>
      {:else}
        <div class="space-y-2">
          {#each activeFlows as flow}
            <button
              onclick={() => { selectedFlow = flow; selectedRequest = flow.requests[flow.currentRequestIndex] || flow.requests[0]; }}
              class="w-full p-3 text-left bg-blue-50 dark:bg-blue-900/20 hover:bg-blue-100 dark:hover:bg-blue-900/30 rounded-lg border border-blue-200 dark:border-blue-800 transition-colors
                     {selectedFlow?.id === flow.id ? 'ring-2 ring-blue-500' : ''}"
            >
              <div class="flex items-center justify-between mb-1">
                <span class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">{flow.title}</span>
                <span class="text-xs text-blue-600 dark:text-blue-400">
                  {flow.currentRequestIndex + 1}/{flow.requests.length}
                </span>
              </div>
              <p class="text-xs text-gray-600 dark:text-gray-400 truncate">{flow.description}</p>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                {flow.serverName} • {new Date(flow.startedAt).toLocaleTimeString()}
              </p>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Standalone Pending Requests -->
    <div class="p-4 border-b border-gray-200 dark:border-gray-700">
      <h3 class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3 flex items-center">
        <Clock size={14} class="mr-2" />
        Pending Requests ({pendingRequests.filter(r => r.status === 'pending').length})
      </h3>

      <div class="space-y-2">
        {#each pendingRequests.filter(r => r.status === 'pending') as request}
          <button
            onclick={() => { selectedRequest = request; selectedFlow = null; }}
            class="w-full p-3 text-left bg-yellow-50 dark:bg-yellow-900/20 hover:bg-yellow-100 dark:hover:bg-yellow-900/30 rounded-lg border border-yellow-200 dark:border-yellow-800 transition-colors
                   {selectedRequest?.id === request.id && !selectedFlow ? 'ring-2 ring-yellow-500' : ''}"
          >
            <div class="flex items-center justify-between mb-1">
              <span class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">{request.title}</span>
              <span class="text-xs px-2 py-1 rounded {getPriorityColor(request.priority)}">
                {request.priority}
              </span>
            </div>
            <p class="text-xs text-gray-600 dark:text-gray-400 truncate">{request.message}</p>
            <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
              {request.serverName} • {new Date(request.timestamp).toLocaleTimeString()}
            </p>
          </button>
        {/each}
      </div>
    </div>

    <!-- Completed Flows -->
    {#if showCompleted}
      <div class="flex-1 overflow-y-auto p-4">
        <h3 class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3 flex items-center">
          <CheckCircle size={14} class="mr-2" />
          Completed ({completedFlows.length})
        </h3>

        <div class="space-y-2">
          {#each completedFlows.slice(0, 5) as flow}
            <button
              onclick={() => { selectedFlow = flow; selectedRequest = null; }}
              class="w-full p-3 text-left bg-gray-50 dark:bg-gray-700 hover:bg-gray-100 dark:hover:bg-gray-600 rounded-lg border border-gray-200 dark:border-gray-600 transition-colors
                     {selectedFlow?.id === flow.id ? 'ring-2 ring-gray-500 dark:ring-gray-400' : ''}"
            >
              <div class="flex items-center justify-between mb-1">
                <span class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">{flow.title}</span>
                <span class="text-xs px-2 py-1 rounded {getStatusColor(flow.status)}">
                  {flow.status}
                </span>
              </div>
              <p class="text-xs text-gray-500 dark:text-gray-400">
                {flow.serverName} • {flow.completedAt ? new Date(flow.completedAt).toLocaleTimeString() : ''}
              </p>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <!-- Right Panel: Request Details -->
  <div class="flex-1 flex flex-col bg-white dark:bg-gray-800">
    {#if selectedRequest}
      <!-- Header -->
      <div class="p-4 border-b border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between">
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{selectedRequest.title}</h3>
            <div class="flex items-center space-x-4 mt-1 text-sm text-gray-600 dark:text-gray-400">
              <span class="px-2 py-1 rounded text-xs {getStatusColor(selectedRequest.status)}">
                {selectedRequest.status.toUpperCase()}
              </span>
              <span class="px-2 py-1 rounded text-xs {getPriorityColor(selectedRequest.priority)}">
                {selectedRequest.priority.toUpperCase()}
              </span>
              <span>{new Date(selectedRequest.timestamp).toLocaleString()}</span>
              {#if selectedRequest.timeout}
                <span>Timeout: {selectedRequest.timeout}s</span>
              {/if}
            </div>
          </div>

          {#if selectedRequest.status === 'pending'}
            <div class="flex items-center space-x-2">
              <button
                onclick={() => cancelRequest(selectedRequest!)}
                class="btn-secondary text-sm text-red-600 hover:bg-red-50"
              >
                <X size={14} class="mr-1" />
                Cancel
              </button>
            </div>
          {/if}
        </div>
      </div>

      <div class="flex-1 overflow-y-auto p-4">
        <div class="space-y-6">
          <!-- Request Message -->
          <div>
            <h4 class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">Request</h4>
            <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
              <p class="text-sm text-gray-900 dark:text-gray-100">{selectedRequest.message}</p>
              {#if selectedRequest.context}
                <div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-600">
                  <span class="text-xs text-gray-600 dark:text-gray-400">Context: </span>
                  <span class="text-xs font-mono bg-gray-200 dark:bg-gray-600 text-gray-900 dark:text-gray-100 px-2 py-1 rounded">{selectedRequest.context}</span>
                </div>
              {/if}
            </div>
          </div>

          <!-- Response Interface -->
          {#if selectedRequest.status === 'pending'}
            <div>
              <h4 class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">Response</h4>

              {#if selectedRequest.type === 'confirmation'}
                <div class="space-y-3">
                  <button
                    onclick={() => respondToRequest(selectedRequest!, true)}
                    class="w-full p-3 bg-green-50 hover:bg-green-100 border border-green-200 rounded-lg text-left transition-colors"
                  >
                    <div class="flex items-center">
                      <Check size={16} class="text-green-600 mr-3" />
                      <span class="text-sm font-medium text-green-900">
                        {selectedRequest.confirmationOptions?.confirm || 'Confirm'}
                      </span>
                    </div>
                  </button>
                  <button
                    onclick={() => respondToRequest(selectedRequest!, false)}
                    class="w-full p-3 bg-red-50 hover:bg-red-100 border border-red-200 rounded-lg text-left transition-colors"
                  >
                    <div class="flex items-center">
                      <X size={16} class="text-red-600 mr-3" />
                      <span class="text-sm font-medium text-red-900">
                        {selectedRequest.confirmationOptions?.cancel || 'Cancel'}
                      </span>
                    </div>
                  </button>
                </div>

              {:else if selectedRequest.type === 'input'}
                <div class="space-y-3">
                  {#if selectedRequest.inputPrompt?.multiline}
                    <textarea
                      bind:value={responseData.input}
                      placeholder={selectedRequest.inputPrompt?.placeholder || 'Enter your response...'}
                      class="form-input h-24 resize-none"
                    ></textarea>
                  {:else}
                    <input
                      type="text"
                      bind:value={responseData.input}
                      placeholder={selectedRequest.inputPrompt?.placeholder || 'Enter your response...'}
                      class="form-input"
                    />
                  {/if}
                  <button
                    onclick={() => respondToRequest(selectedRequest!, responseData.input)}
                    disabled={!responseData.input?.trim()}
                    class="btn-primary"
                  >
                    <Send size={14} class="mr-1" />
                    Send Response
                  </button>
                </div>

              {:else if selectedRequest.type === 'choice'}
                <div class="space-y-2">
                  {#each selectedRequest.choices || [] as choice}
                    <button
                      onclick={() => respondToRequest(selectedRequest!, choice.id)}
                      class="w-full p-3 bg-gray-50 dark:bg-gray-700 hover:bg-gray-100 dark:hover:bg-gray-600 border border-gray-200 dark:border-gray-600 rounded-lg text-left transition-colors"
                    >
                      <div class="flex items-start">
                        <div class="flex-1">
                          <p class="text-sm font-medium text-gray-900 dark:text-gray-100">{choice.label}</p>
                          {#if choice.description}
                            <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">{choice.description}</p>
                          {/if}
                        </div>
                        <ArrowRight size={14} class="text-gray-400 dark:text-gray-500 mt-0.5" />
                      </div>
                    </button>
                  {/each}
                </div>

              {:else if selectedRequest.type === 'form'}
                <form
                  onsubmit={(e) => {
                    e.preventDefault();
                    respondToRequest(selectedRequest!, responseData);
                  }}
                  class="space-y-4"
                >
                  {#each selectedRequest.formFields || [] as field}
                    <div>
                      <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        {field.label}
                        {#if field.required}
                          <span class="text-red-500">*</span>
                        {/if}
                      </label>

                      {#if field.type === 'textarea'}
                        <textarea
                          bind:value={responseData[field.name]}
                          placeholder={field.placeholder}
                          required={field.required}
                          class="form-input h-20 resize-none"
                        ></textarea>
                      {:else if field.type === 'select'}
                        <select
                          bind:value={responseData[field.name]}
                          required={field.required}
                          class="form-select"
                        >
                          <option value="">Select...</option>
                          {#each field.options || [] as option}
                            <option value={option}>{option}</option>
                          {/each}
                        </select>
                      {:else if field.type === 'checkbox'}
                        <label class="flex items-center">
                          <input
                            type="checkbox"
                            bind:checked={responseData[field.name]}
                            class="form-checkbox"
                          />
                          <span class="ml-2 text-sm text-gray-700 dark:text-gray-300">Enable</span>
                        </label>
                      {:else}
                        <input
                          type={field.type}
                          bind:value={responseData[field.name]}
                          placeholder={field.placeholder}
                          required={field.required}
                          class="form-input"
                        />
                      {/if}
                    </div>
                  {/each}

                  <button type="submit" class="btn-primary">
                    <Send size={14} class="mr-1" />
                    Submit Response
                  </button>
                </form>
              {/if}
            </div>

          {:else if selectedRequest.status === 'responded'}
            <!-- Show Response -->
            <div>
              <h4 class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">Your Response</h4>
              <div class="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-4">
                <div class="flex items-center justify-between mb-2">
                  <span class="text-xs text-green-700 dark:text-green-400 font-medium">
                    Responded at {selectedRequest!.respondedAt ? new Date(selectedRequest!.respondedAt).toLocaleTimeString() : ''}
                  </span>
                  <button
                    onclick={() => copyToClipboard(formatResponse(selectedRequest!.response))}
                    class="text-green-600 dark:text-green-400 hover:text-green-800 dark:hover:text-green-300"
                  >
                    <Copy size={12} />
                  </button>
                </div>
                <pre class="text-sm text-green-800 dark:text-green-200 whitespace-pre-wrap font-sans">
                  {formatResponse(selectedRequest!.response)}
                </pre>
              </div>
            </div>
          {/if}

          <!-- Flow Progress -->
          {#if selectedFlow}
            <div>
              <h4 class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">Flow Progress</h4>
              <div class="space-y-2">
                {#each selectedFlow.requests as request, index}
                  <div class="flex items-center">
                    <div class="flex items-center justify-center w-6 h-6 rounded-full border-2
                                {request.status === 'responded' ? 'bg-green-100 dark:bg-green-900/30 border-green-500 dark:border-green-400' :
                                 request.status === 'pending' ? 'bg-yellow-100 dark:bg-yellow-900/30 border-yellow-500 dark:border-yellow-400' :
                                 'bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600'}">
                      {#if request.status === 'responded'}
                        <Check size={12} class="text-green-600 dark:text-green-400" />
                      {:else if request.status === 'pending'}
                        <Clock size={12} class="text-yellow-600 dark:text-yellow-400" />
                      {:else}
                        <span class="text-xs text-gray-500 dark:text-gray-400">{index + 1}</span>
                      {/if}
                    </div>
                    <div class="ml-3 flex-1">
                      <p class="text-sm text-gray-900 dark:text-gray-100">{request.title}</p>
                      <p class="text-xs text-gray-500 dark:text-gray-400">{request.type}</p>
                    </div>
                    {#if index < selectedFlow.requests.length - 1}
                      <ArrowRight size={14} class="text-gray-300 dark:text-gray-600" />
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>

    {:else if selectedFlow}
      <!-- Flow Overview -->
      <div class="p-4 border-b border-gray-200 dark:border-gray-700">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{selectedFlow.title}</h3>
        <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">{selectedFlow.description}</p>
        <div class="flex items-center space-x-4 mt-2 text-sm text-gray-500 dark:text-gray-400">
          <span>Server: {selectedFlow.serverName}</span>
          <span>Started: {new Date(selectedFlow.startedAt).toLocaleString()}</span>
          <span class="px-2 py-1 rounded text-xs {getStatusColor(selectedFlow.status)}">
            {selectedFlow.status.toUpperCase()}
          </span>
        </div>
      </div>

      <div class="flex-1 p-4">
        <h4 class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-4">Requests in Flow</h4>
        <div class="space-y-3">
          {#each selectedFlow.requests as request, index}
            <button
              onclick={() => selectedRequest = request}
              class="w-full p-4 text-left border border-gray-200 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
            >
              <div class="flex items-center justify-between">
                <div class="flex items-center">
                  <div class="flex items-center justify-center w-8 h-8 rounded-full border-2 mr-3
                              {request.status === 'responded' ? 'bg-green-100 dark:bg-green-900/30 border-green-500 dark:border-green-400' :
                               request.status === 'pending' ? 'bg-yellow-100 dark:bg-yellow-900/30 border-yellow-500 dark:border-yellow-400' :
                               'bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600'}">
                    {#if request.status === 'responded'}
                      <Check size={14} class="text-green-600 dark:text-green-400" />
                    {:else if request.status === 'pending'}
                      <Clock size={14} class="text-yellow-600 dark:text-yellow-400" />
                    {:else}
                      <span class="text-sm text-gray-500 dark:text-gray-400">{index + 1}</span>
                    {/if}
                  </div>
                  <div>
                    <h5 class="text-sm font-medium text-gray-900 dark:text-gray-100">{request.title}</h5>
                    <p class="text-xs text-gray-600 dark:text-gray-400">{request.type} • {request.priority} priority</p>
                  </div>
                </div>
                <ArrowRight size={16} class="text-gray-300 dark:text-gray-600" />
              </div>
            </button>
          {/each}
        </div>
      </div>

    {:else}
      <!-- No Selection -->
      <div class="flex items-center justify-center h-full">
        <div class="text-center">
          <MessageCircle size={48} class="mx-auto text-gray-400 dark:text-gray-500 mb-4" />
          <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">Select an Elicitation Request</h3>
          <p class="text-gray-600 dark:text-gray-400">Choose a request or flow from the queue to respond</p>

          {#if elicitationFlows.length === 0 && pendingRequests.length === 0}
            <button
              onclick={loadElicitationRequests}
              class="btn-primary mt-4"
              disabled={loading}
            >
              {#if loading}
                <RefreshCw size={16} class="mr-2 animate-spin" />
                Loading...
              {:else}
                <RefreshCw size={16} class="mr-2" />
                Refresh Requests
              {/if}
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>