<script lang="ts">
  import Button from './ui/Button.svelte';
  import { serverStore } from '$lib/stores/serverStore';
  import { invoke } from '@tauri-apps/api/core';

  // Component props using Svelte 5 runes mode
  const { serverId = '' } = $props();

  // Get current selected server from store if serverId not provided
  const servers = $derived($serverStore.servers);
  const selectedServer = $derived($serverStore.selectedServer);
  const currentServerId = $derived(serverId || selectedServer?.id || '');
  const currentServer = $derived(servers.find(s => s.id === currentServerId));

  // Testing state
  let loading = $state(false);
  let error = $state(null);
  let lastRequest = $state(null);
  let lastResponse = $state(null);

  // TurboMCP Production Sampling Configuration
  let llmProvider = $state('openai'); // 'openai' | 'anthropic'
  let apiKey = $state('');
  let baseUrl = $state('');
  let organization = $state('');
  let model = $state('gpt-3.5-turbo');
  let systemPrompt = $state('You are a helpful AI assistant integrated with MCP servers.');
  let temperature = $state(0.7);
  let maxTokens = $state(150);
  let stopSequences = $state('');

  // Model Preferences (TurboMCP feature)
  let costPriority = $state(0.7);
  let speedPriority = $state(0.6);
  let intelligencePriority = $state(0.8);

  // Context and conversation management
  let includeContext = $state('ThisServer'); // 'ThisServer' | 'AllServers' | 'None'
  let conversationHistory = $state([]);
  let showAdvanced = $state(false);
  let userMessage = $state('');

  // Server data derived from store
  const serverData = $derived($serverStore.servers.find(s => s.id === serverId));


  async function sendTestRequest() {
    if (!currentServerId) {
      error = 'No server selected';
      return;
    }

    if (!userMessage.trim()) {
      error = 'Please enter a message to send';
      return;
    }

    if (!apiKey.trim()) {
      error = `Please enter your ${llmProvider === 'openai' ? 'OpenAI' : 'Anthropic'} API key`;
      return;
    }

    loading = true;
    error = null;

    // Build conversation messages including history
    const messages = [...conversationHistory];

    // Add current user message
    messages.push({
      role: 'user',
      content: {
        type: 'text',
        text: userMessage,
        annotations: null,
        meta: null
      }
    });

    // Build TurboMCP CreateMessageRequest
    const createMessageRequest = {
      messages,
      model_preferences: {
        cost_priority: costPriority,
        speed_priority: speedPriority,
        intelligence_priority: intelligencePriority,
        hints: [
          {
            name: model
          }
        ]
      },
      system_prompt: systemPrompt.trim() || null,
      include_context: includeContext,
      temperature: temperature,
      max_tokens: maxTokens,
      stop_sequences: stopSequences.trim() ? stopSequences.split(',').map(s => s.trim()).filter(Boolean) : null,
      metadata: {
        user_id: 'mcp-studio-user',
        session_id: crypto.randomUUID(),
        timestamp: new Date().toISOString()
      },
      _meta: null
    };

    // LLM Backend Configuration for TurboMCP
    const llmBackendConfig = {
      provider: llmProvider === 'openai'
        ? {
            OpenAI: {
              api_key: apiKey,
              base_url: baseUrl.trim() || null,
              organization: organization.trim() || null
            }
          }
        : {
            Anthropic: {
              api_key: apiKey,
              base_url: baseUrl.trim() || null
            }
          },
      default_model: model,
      timeout_seconds: 30,
      max_retries: 3
    };

    lastRequest = { createMessageRequest, llmBackendConfig };
    lastResponse = null;

    try {
      // Use TurboMCP production sampling through Tauri backend
      const response = await invoke('create_sampling_request', {
        server_id: currentServerId,
        create_message_request: createMessageRequest,
        llm_backend_config: llmBackendConfig
      });

      // Add user message to conversation history
      conversationHistory.push({
        role: 'user',
        content: {
          type: 'text',
          text: userMessage
        }
      });

      // Add assistant response to conversation history
      if (response.content) {
        conversationHistory.push({
          role: 'assistant',
          content: response.content
        });
      }

      lastResponse = response;

      // Clear the user message for next input
      userMessage = '';

    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to send sampling request';
      console.error('Sampling request failed:', err);
    } finally {
      loading = false;
    }
  }

  function clearHistory() {
    lastRequest = null;
    lastResponse = null;
    error = null;
    conversationHistory = [];
    userMessage = '';
  }

  function exportConfiguration() {
    const config = {
      llmProvider,
      model,
      systemPrompt,
      temperature,
      maxTokens,
      stopSequences,
      costPriority,
      speedPriority,
      intelligencePriority,
      includeContext,
      baseUrl,
      organization
    };

    navigator.clipboard.writeText(JSON.stringify(config, null, 2));
    // Could show a toast notification here
  }

</script>

<div class="h-full flex flex-col bg-white dark:bg-gray-900">

    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded bg-green-100 dark:bg-green-900 flex items-center justify-center">
          <svg class="w-4 h-4 text-green-600 dark:text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
          </svg>
        </div>
        <div>
          <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Sampling Tester</h2>
          <p class="text-sm text-gray-600 dark:text-gray-400">
            Test sampling workflows with {serverData?.name || 'server'}
          </p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="ghost" onclick={exportConfiguration} title="Export configuration">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/>
          </svg>
        </Button>
        {#if currentServer}
          <span class="text-sm text-gray-500 dark:text-gray-400">
            Connected to: {currentServer.config.name || currentServer.id}
          </span>
        {/if}
      </div>
    </div>

    <div class="flex-1 overflow-auto">
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 p-6">

        <!-- Left Panel: TurboMCP Sampling Configuration -->
        <div class="space-y-6">

          <!-- LLM Provider Configuration -->
          <section>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">LLM Provider</h3>
            <div class="space-y-4">
              <div class="grid grid-cols-2 gap-2">
                <button
                  onclick={() => llmProvider = 'openai'}
                  class="px-4 py-2 text-sm rounded-md border transition-colors {llmProvider === 'openai' ? 'bg-green-50 border-green-200 text-green-700 dark:bg-green-900/20 dark:border-green-800 dark:text-green-300' : 'border-gray-300 text-gray-700 hover:bg-gray-50 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700'}"
                >
                  OpenAI
                </button>
                <button
                  onclick={() => llmProvider = 'anthropic'}
                  class="px-4 py-2 text-sm rounded-md border transition-colors {llmProvider === 'anthropic' ? 'bg-blue-50 border-blue-200 text-blue-700 dark:bg-blue-900/20 dark:border-blue-800 dark:text-blue-300' : 'border-gray-300 text-gray-700 hover:bg-gray-50 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700'}"
                >
                  Anthropic
                </button>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  API Key {llmProvider === 'openai' ? '(OpenAI)' : '(Anthropic)'}
                </label>
                <input
                  type="password"
                  bind:value={apiKey}
                  placeholder={llmProvider === 'openai' ? 'sk-...' : 'sk-ant-...'}
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                />
              </div>

              {#if llmProvider === 'openai'}
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Model</label>
                  <select bind:value={model} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-white">
                    <option value="gpt-3.5-turbo">GPT-3.5 Turbo</option>
                    <option value="gpt-4">GPT-4</option>
                    <option value="gpt-4-turbo">GPT-4 Turbo</option>
                    <option value="gpt-4o">GPT-4o</option>
                  </select>
                </div>
              {:else}
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Model</label>
                  <select bind:value={model} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-white">
                    <option value="claude-3-haiku-20240307">Claude 3 Haiku</option>
                    <option value="claude-3-sonnet-20240229">Claude 3 Sonnet</option>
                    <option value="claude-3-opus-20240229">Claude 3 Opus</option>
                  </select>
                </div>
              {/if}
            </div>
          </section>

          <!-- Model Preferences (TurboMCP Feature) -->
          <section>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Model Preferences</h3>
            <div class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Cost Priority: {costPriority}
                </label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.1"
                  bind:value={costPriority}
                  class="w-full"
                />
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Speed Priority: {speedPriority}
                </label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.1"
                  bind:value={speedPriority}
                  class="w-full"
                />
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Intelligence Priority: {intelligencePriority}
                </label>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.1"
                  bind:value={intelligencePriority}
                  class="w-full"
                />
              </div>
            </div>
          </section>

          <!-- Context Configuration -->
          <section>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Context Configuration</h3>
            <div class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Include Context</label>
                <select bind:value={includeContext} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-white">
                  <option value="ThisServer">This Server Only</option>
                  <option value="AllServers">All Connected Servers</option>
                  <option value="None">No Server Context</option>
                </select>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">System Prompt</label>
                <textarea
                  bind:value={systemPrompt}
                  placeholder="You are a helpful AI assistant..."
                  class="w-full h-24 p-3 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                ></textarea>
              </div>
            </div>
          </section>

          <!-- Sampling Parameters -->
          <section>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Sampling Parameters</h3>
            <div class="space-y-4">
              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    Temperature: {temperature}
                  </label>
                  <input
                    type="range"
                    min="0"
                    max="2"
                    step="0.1"
                    bind:value={temperature}
                    class="w-full"
                  />
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Max Tokens</label>
                  <input
                    type="number"
                    bind:value={maxTokens}
                    min="1"
                    max="4096"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                  />
                </div>
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Stop Sequences (comma-separated)</label>
                <input
                  type="text"
                  bind:value={stopSequences}
                  placeholder="\\n\\n, END, ..."
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                />
              </div>
            </div>
          </section>

          <!-- Message Composer -->
          <section>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Message Composer</h3>
            <div class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">User Message</label>
                <textarea
                  bind:value={userMessage}
                  placeholder="Enter your message to test sampling..."
                  class="w-full h-24 p-3 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                ></textarea>
              </div>

              {#if conversationHistory.length > 0}
                <div>
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    Conversation History ({conversationHistory.length} messages)
                  </label>
                  <div class="max-h-32 overflow-y-auto border border-gray-300 dark:border-gray-600 rounded-md p-2 bg-gray-50 dark:bg-gray-800">
                    {#each conversationHistory as message, i}
                      <div class="text-xs mb-1">
                        <span class="font-medium {message.role === 'user' ? 'text-blue-600 dark:text-blue-400' : 'text-green-600 dark:text-green-400'}">
                          {message.role}:
                        </span>
                        <span class="text-gray-700 dark:text-gray-300">
                          {typeof message.content === 'string' ? message.content.substring(0, 60) : message.content.text?.substring(0, 60)}...
                        </span>
                      </div>
                    {/each}
                  </div>
                  <button
                    onclick={() => conversationHistory = []}
                    class="mt-2 text-xs text-red-600 dark:text-red-400 hover:underline"
                  >
                    Clear History
                  </button>
                </div>
              {/if}
            </div>
          </section>

          <!-- Actions -->
          <section>
            <div class="flex gap-2">
              <Button
                variant="primary"
                onclick={sendTestRequest}
                disabled={loading}
                class="flex-1"
              >
                {#if loading}
                  <svg class="animate-spin -ml-1 mr-3 h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                  Testing...
                {:else}
                  Send Test Request
                {/if}
              </Button>
              <Button variant="outline" onclick={clearHistory}>Clear</Button>
            </div>
          </section>
        </div>

        <!-- Right Panel: Results -->
        <div class="space-y-6">

          <!-- Last Request -->
          {#if lastRequest}
          <section>
            <h3 class="text-md font-medium text-gray-900 dark:text-white mb-3">Last Request</h3>
            <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
              <pre class="text-sm text-gray-800 dark:text-gray-200 whitespace-pre-wrap font-mono">{JSON.stringify(lastRequest, null, 2)}</pre>
            </div>
          </section>
          {/if}

          <!-- Response -->
          {#if lastResponse}
          <section>
            <h3 class="text-md font-medium text-gray-900 dark:text-white mb-3">Response</h3>
            <div class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
              <div class="flex items-center justify-between mb-3">
                <div class="flex items-center gap-4 text-sm text-gray-600 dark:text-gray-400">
                  <span>Model: {lastResponse.model}</span>
                  <span>Stop: {lastResponse.stopReason}</span>
                  {#if lastResponse.usage}
                    <span>Tokens: {lastResponse.usage.inputTokens + lastResponse.usage.outputTokens}</span>
                  {/if}
                </div>
                <Button variant="ghost" onclick={() => navigator.clipboard.writeText(lastResponse.content.text)}>
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/>
                  </svg>
                </Button>
              </div>


              <div class="text-gray-900 dark:text-white">
                <pre class="whitespace-pre-wrap font-sans">{lastResponse.content.text || lastResponse.content}</pre>
              </div>
            </div>
          </section>
          {/if}

          <!-- Error Display -->
          {#if error}
          <section>
            <div class="border border-red-200 dark:border-red-800 rounded-lg p-4 bg-red-50 dark:bg-red-900/20">
              <div class="flex items-center gap-2 mb-2">
                <svg class="w-5 h-5 text-red-600 dark:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                <span class="font-medium text-red-800 dark:text-red-200">Error</span>
              </div>
              <p class="text-red-700 dark:text-red-300 text-sm">{error}</p>
            </div>
          </section>
          {/if}


          <!-- Usage Tips -->
          {#if !lastRequest && !lastResponse}
          <section>
            <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
              <h3 class="text-md font-medium text-blue-800 dark:text-blue-200 mb-2">💡 Getting Started</h3>
              <ul class="text-sm text-blue-700 dark:text-blue-300 space-y-1">
                <li>• Configure your LLM provider (OpenAI or Anthropic) with API key</li>
                <li>• Adjust model preferences for cost, speed, and intelligence priorities</li>
                <li>• Include server context to utilize MCP resources and tools</li>
                <li>• Use conversation history to maintain context across requests</li>
                <li>• Export configurations for team sharing and reuse</li>
              </ul>
            </div>
          </section>
          {/if}
        </div>
      </div>
    </div>
</div>