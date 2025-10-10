<!--
  MCP Studio Settings
  Simple, provider-centric configuration with 2025 local LLM support
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { fetch as tauriFetch } from '@tauri-apps/plugin-http';
  import Button from './ui/Button.svelte';

  interface LLMModelsResponse {
    data: Array<{
      id?: string;
      name?: string;
    }>;
  }
  import {
    Settings as SettingsIcon,
    Zap,
    Globe,
    Monitor,
    CheckCircle,
    AlertCircle,
    Clock,
    Plus,
    Edit3,
    Eye,
    EyeOff,
    Trash2,
    RefreshCw,
    ExternalLink,
    Cpu,
    Cloud,
    DollarSign
  } from 'lucide-svelte';

  // ===============================================
  // TYPES & INTERFACES
  // ===============================================

  interface LLMProvider {
    id: string;
    name: string;
    type: 'cloud' | 'local';
    icon: string;
    default_base_url: string;
    requires_api_key: boolean;
    description: string;
    status: 'unconfigured' | 'configured' | 'active' | 'error';
    api_key?: string;
    base_url?: string;
    models?: string[];
    last_error?: string;
    usage_stats?: {
      total_requests: number;
      successful_requests: number;
      total_cost: number;
    };
  }

  interface QuickSetupModal {
    show: boolean;
    provider: LLMProvider | null;
    api_key: string;
    base_url: string;
    testing: boolean;
    test_result: 'none' | 'success' | 'error';
    test_message: string;
    available_models: string[];
    selected_model: string;
  }

  // ===============================================
  // STATE MANAGEMENT
  // ===============================================

  // Note: 'providers' and 'usage' tabs commented out - vestigial code for future LLM integration
  let activeTab: 'providers' | 'global' | 'usage' = $state('global');

  // Provider management
  let providers: LLMProvider[] = $state([]);
  let loading = $state(false);
  let refreshing = $state(false);

  // Quick setup modal
  let quickSetup: QuickSetupModal = $state({
    show: false,
    provider: null,
    api_key: '',
    base_url: '',
    testing: false,
    test_result: 'none',
    test_message: '',
    available_models: [],
    selected_model: ''
  });

  // Show API keys toggle
  let showApiKeys = $state(false);

  // ===============================================
  // PROVIDER DEFINITIONS (2025 Local + Cloud)
  // ===============================================

  const AVAILABLE_PROVIDERS: Omit<LLMProvider, 'status' | 'api_key' | 'base_url' | 'models'>[] = [
    // Cloud Providers
    {
      id: 'openai',
      name: 'OpenAI',
      type: 'cloud',
      icon: '🤖',
      default_base_url: 'https://api.openai.com/v1',
      requires_api_key: true,
      description: 'GPT-4o, GPT-4o-mini, GPT-3.5-turbo, DALL-E'
    },
    {
      id: 'anthropic',
      name: 'Anthropic',
      type: 'cloud',
      icon: '🧠',
      default_base_url: 'https://api.anthropic.com/v1',
      requires_api_key: true,
      description: 'Claude 3.5 Sonnet, Claude 3.5 Haiku, Claude 3 Opus'
    },

    // Local Providers (2025)
    {
      id: 'ollama',
      name: 'Ollama',
      type: 'local',
      icon: '🦙',
      default_base_url: 'http://localhost:11434/v1',
      requires_api_key: false,
      description: 'Local models: Llama, Mistral, CodeLlama, DeepSeek-R1'
    },
    {
      id: 'lmstudio',
      name: 'LM Studio',
      type: 'local',
      icon: '🎬',
      default_base_url: 'http://localhost:1234/v1',
      requires_api_key: false,
      description: 'Local GUI for GGUF models with OpenAI-compatible API'
    },
    {
      id: 'gpt4all',
      name: 'GPT4All',
      type: 'local',
      icon: '🌐',
      default_base_url: 'http://localhost:4891/v1',
      requires_api_key: false,
      description: 'Free local models with privacy focus'
    },
    {
      id: 'jan',
      name: 'Jan',
      type: 'local',
      icon: '📱',
      default_base_url: 'http://localhost:1337/v1',
      requires_api_key: false,
      description: 'Open-source ChatGPT alternative running locally'
    },
    {
      id: 'custom',
      name: 'Custom',
      type: 'local',
      icon: '⚙️',
      default_base_url: 'http://localhost:8000/v1',
      requires_api_key: false,
      description: 'Any OpenAI-compatible API endpoint'
    }
  ];

  // ===============================================
  // COMPUTED STATE
  // ===============================================

  const activeProvider = $derived(providers.find(p => p.status === 'active'));
  const configuredProviders = $derived(providers.filter(p => p.status !== 'unconfigured'));
  const localProviders = $derived(providers.filter(p => p.type === 'local'));
  const cloudProviders = $derived(providers.filter(p => p.type === 'cloud'));

  // ===============================================
  // PROVIDER MANAGEMENT
  // ===============================================

  async function loadProviders() {
    loading = true;
    try {
      // Use the same backend call as SamplingWorkbench for consistency
      const [providerStatusData, llmConfigData] = await Promise.all([
        invoke('get_llm_provider_statuses'),
        invoke('get_llm_config')
      ]);


      const backendConfig = llmConfigData as any;
      const activeProviderId = backendConfig?.active_provider;

      // Merge with available provider definitions
      providers = AVAILABLE_PROVIDERS.map(available => {
        const existing = (providerStatusData as any[]).find(p => p.provider_id === available.id);
        const isActiveProvider = activeProviderId === available.id;
        const providerConfig = backendConfig?.providers?.[available.id];


        let status = 'unconfigured';
        if (providerConfig?.enabled) {
          status = isActiveProvider ? 'active' : 'configured';
        }


        return {
          ...available,
          status,
          api_key: existing?.api_key,
          base_url: providerConfig?.base_url || existing?.base_url || available.default_base_url,
          models: providerConfig?.available_models || existing?.available_models || [],
          usage_stats: existing?.usage_stats
        } as LLMProvider;
      });

    } catch (error) {
      console.error('Failed to load providers:', error);
    } finally {
      loading = false;
    }
  }

  async function refreshProviders() {
    refreshing = true;
    await loadProviders();
    refreshing = false;
  }

  // ===============================================
  // QUICK SETUP MODAL
  // ===============================================

  function openQuickSetup(provider: LLMProvider) {
    quickSetup = {
      show: true,
      provider,
      api_key: provider.api_key || '',
      base_url: provider.base_url || provider.default_base_url,
      testing: false,
      test_result: 'none',
      test_message: '',
      available_models: [],
      selected_model: ''
    };
  }

  function closeQuickSetup() {
    quickSetup.show = false;
    quickSetup.provider = null;
    quickSetup.api_key = '';
    quickSetup.base_url = '';
    quickSetup.test_result = 'none';
    quickSetup.test_message = '';
  }

  async function testConnection() {
    if (!quickSetup.provider) return;

    quickSetup.testing = true;
    quickSetup.test_result = 'none';

    try {
      // Use Tauri backend command to bypass CORS restrictions
      const data = await invoke<LLMModelsResponse>('fetch_llm_models', {
        baseUrl: quickSetup.base_url
      });

      if (data && data.data) {
        const models = data.data.map((model: any) => model.id || model.name || model);
        quickSetup.available_models = models;
        quickSetup.test_result = 'success';
        quickSetup.test_message = `Connection successful! Found ${models.length} models available.`;

        // Auto-select first model if none selected
        if (models.length > 0 && !quickSetup.selected_model) {
          quickSetup.selected_model = models[0];
        }
      } else {
        quickSetup.test_result = 'error';
        quickSetup.test_message = 'Connection successful but no models found.';
      }
    } catch (error) {
      quickSetup.test_result = 'error';
      quickSetup.test_message = `Connection failed: ${error}`;
      quickSetup.available_models = [];
    } finally {
      quickSetup.testing = false;
    }
  }

  async function saveProvider() {
    if (!quickSetup.provider) return;

    try {
      // Only set API key for providers that actually require them
      // Local providers (like LM Studio, Ollama) should not trigger keyring access
      if (quickSetup.provider.requires_api_key && quickSetup.api_key) {
        await invoke('set_llm_api_key', {
          providerId: quickSetup.provider.id,
          apiKey: quickSetup.api_key
        });
      }

      // Fetch available models for local providers
      let availableModels;
      let defaultModel;

      if (quickSetup.provider.type === 'local') {
        // Use models from test connection if available, otherwise fetch them
        if (quickSetup.available_models.length > 0) {
          availableModels = quickSetup.available_models;
          defaultModel = quickSetup.selected_model || availableModels[0];
        } else {
          try {
            // Fetch models from local LM Studio API using Tauri backend (avoids CORS)
            const data = await invoke<LLMModelsResponse>('fetch_llm_models', {
              baseUrl: quickSetup.base_url
            });

            if (data && data.data) {
              availableModels = data.data.map((model: any) => model.id || model.name || model) || ['local-model'];
              defaultModel = availableModels[0] || 'local-model';
            } else {
              // Fallback if models fetch fails
              availableModels = ['local-model'];
              defaultModel = 'local-model';
            }
          } catch (error) {
            console.warn('Failed to fetch models from local provider:', error);
            availableModels = ['local-model'];
            defaultModel = 'local-model';
          }
        }
      } else {
        // Cloud providers use predefined models
        availableModels = quickSetup.provider.id === 'anthropic' ? ['claude-3-5-sonnet-20241022'] :
                         ['gpt-4o', 'gpt-4o-mini'];
        defaultModel = quickSetup.provider.id === 'anthropic' ? 'claude-3-5-sonnet-20241022' : 'gpt-4o';
      }

      // Create complete LLMProviderConfig object based on provider type
      const config = {
        provider_type: quickSetup.provider.type === 'local' ? 'local' :
                      quickSetup.provider.id === 'anthropic' ? 'anthropic' : 'openai',
        display_name: quickSetup.provider.name,
        enabled: true,
        default_model: defaultModel,
        available_models: availableModels,
        base_url: quickSetup.base_url,
        organization: null,
        max_retries: 3,
        timeout_seconds: quickSetup.provider.type === 'local' ? 30 : 60,
        rate_limit: {
          requests_per_minute: quickSetup.provider.type === 'local' ? 1000 : 100,
          tokens_per_minute: quickSetup.provider.type === 'local' ? null : 1000000,
          exponential_backoff: true,
          initial_backoff_ms: 1000,
          max_backoff_ms: quickSetup.provider.type === 'local' ? 30000 : 60000
        },
        cost_config: {
          input_cost_per_1k: quickSetup.provider.type === 'local' ? 0.0 :
                             quickSetup.provider.id === 'anthropic' ? 3.0 : 2.5,
          output_cost_per_1k: quickSetup.provider.type === 'local' ? 0.0 :
                              quickSetup.provider.id === 'anthropic' ? 15.0 : 10.0,
          thinking_cost_per_1k: null,
          currency: 'USD'
        },
        capabilities: {
          supports_structured_outputs: quickSetup.provider.id === 'openai',
          structured_output_models: quickSetup.provider.id === 'openai' ? ['gpt-4o'] : [],
          max_structured_output_tokens: quickSetup.provider.id === 'openai' ? 200000 : null,
          supports_batch_processing: quickSetup.provider.type === 'cloud',
          batch_discount_percentage: quickSetup.provider.type === 'cloud' ? 50.0 : null,
          supports_parallel_function_calling: quickSetup.provider.id === 'openai',
          supports_strict_function_calling: quickSetup.provider.type === 'cloud',
          supports_vision: quickSetup.provider.type === 'cloud',
          supported_image_formats: quickSetup.provider.type === 'cloud' ? ['jpeg', 'png', 'gif', 'webp'] : [],
          supports_audio: quickSetup.provider.id === 'openai',
          supported_audio_formats: quickSetup.provider.id === 'openai' ? ['wav', 'mp3'] : [],
          supports_streaming: true,
          supports_function_calling: quickSetup.provider.type === 'cloud',
          supports_computer_use: quickSetup.provider.id === 'anthropic',
          supports_thinking_tokens: false,
          max_context_tokens: quickSetup.provider.type === 'local' ? 32768 : 200000
        }
      };

      // Update provider configuration with complete config
      // The backend expects UpdateLLMConfigRequest structure passed as the 'config' parameter
      const updateRequest = {
        provider_id: quickSetup.provider.id,
        config: config  // The full LLMProviderConfig object
      };

      await invoke('update_llm_provider_config', { config: updateRequest });

      // Set as active provider
      await invoke('set_active_llm_provider', {
        providerId: quickSetup.provider.id
      });

      await loadProviders();

      // Preload/warm up the selected model for better UX
      if (quickSetup.provider.type === 'local' && defaultModel && quickSetup.base_url) {
        try {
          // Send a small completion request to warm up the model
          await invoke('llm_completion_request', {
            baseUrl: quickSetup.base_url,
            apiKey: '',
            model: defaultModel,
            messages: [{ role: 'user', content: 'Hello' }],
            maxTokens: 1,
            temperature: 0.1
          });
        } catch (error) {
          // Warmup failure is not critical - the model will load on first real request
        }
      }

      closeQuickSetup();
    } catch (error) {
      quickSetup.test_result = 'error';
      quickSetup.test_message = `Failed to save: ${error}`;
    }
  }

  async function removeProvider(providerId: string) {
    try {
      // Remove API key and deactivate provider
      await invoke('remove_llm_api_key', { providerId: providerId });
      await loadProviders();
    } catch (error) {
      console.error('Failed to remove provider:', error);
    }
  }

  async function setActiveProvider(providerId: string) {
    try {
      await invoke('set_active_llm_provider', { providerId: providerId });
      await loadProviders();
    } catch (error) {
      console.error('Failed to set active provider:', error);
    }
  }

  // ===============================================
  // HELPER FUNCTIONS
  // ===============================================

  function getProviderStatusIcon(provider: LLMProvider) {
    switch (provider.status) {
      case 'active': return CheckCircle;
      case 'configured': return Clock;
      case 'error': return AlertCircle;
      default: return Plus;
    }
  }

  function getProviderStatusColor(provider: LLMProvider) {
    switch (provider.status) {
      case 'active': return 'text-green-600';
      case 'configured': return 'text-blue-600';
      case 'error': return 'text-red-600';
      default: return 'text-gray-400';
    }
  }

  function getProviderStatusText(provider: LLMProvider) {
    switch (provider.status) {
      case 'active': return 'Active';
      case 'configured': return 'Ready';
      case 'error': return 'Error';
      default: return 'Setup';
    }
  }

  function maskApiKey(key: string): string {
    if (!key || key.length < 8) return key;
    return key.substring(0, 4) + '••••••••' + key.substring(key.length - 4);
  }

  // ===============================================
  // LIFECYCLE
  // ===============================================

  onMount(() => {
    loadProviders();
  });
</script>

<!-- =============================================== -->
<!-- MAIN SETTINGS UI -->
<!-- =============================================== -->

<div class="settings-container">
  <!-- Header -->
  <div class="settings-header">
    <div class="flex items-center gap-3">
      <div class="w-10 h-10 rounded-lg bg-gradient-to-br from-mcp-primary-500 to-mcp-primary-600 flex items-center justify-center">
        <SettingsIcon size={20} class="text-white" />
      </div>
      <div>
        <h1 class="text-2xl font-semibold text-primary">Settings</h1>
        <p class="text-sm text-secondary">Configure LLM providers and preferences</p>
      </div>
    </div>
    <Button
      variant="secondary"
      size="sm"
      onclick={refreshProviders}
      disabled={refreshing}
      loading={refreshing}
    >
      <RefreshCw size={16} />
      Refresh
    </Button>
  </div>

  <!-- Tab Navigation -->
  <div class="tab-navigation">
    <!-- LLM Providers tab - vestigial, commented out for future use -->
    <!-- <button
      onclick={() => activeTab = 'providers'}
      class="tab-button {activeTab === 'providers' ? 'active' : ''}"
    >
      <Zap size={16} />
      LLM Providers
    </button> -->
    <button
      onclick={() => activeTab = 'global'}
      class="tab-button {activeTab === 'global' ? 'active' : ''}"
    >
      <SettingsIcon size={16} />
      Global Settings
    </button>
    <!-- Usage & Costs tab - vestigial, commented out for future use -->
    <!-- <button
      onclick={() => activeTab = 'usage'}
      class="tab-button {activeTab === 'usage' ? 'active' : ''}"
    >
      <DollarSign size={16} />
      Usage & Costs
    </button> -->
  </div>

  <!-- Tab Content -->
  <div class="tab-content">
    {#if activeTab === 'providers'}

      <!-- Active Provider Summary -->
      {#if activeProvider}
        <div class="active-provider-card">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <span class="text-2xl">{activeProvider.icon}</span>
              <div>
                <h3 class="font-semibold text-primary">
                  {activeProvider.name}
                  <span class="text-green-600 text-sm ml-2">Active</span>
                </h3>
                <p class="text-sm text-secondary">
                  {activeProvider.models?.length || 0} models available
                </p>
              </div>
            </div>
            <CheckCircle size={20} class="text-green-600" />
          </div>
        </div>
      {:else}
        <div class="no-active-provider">
          <AlertCircle size={24} class="text-orange-500 mx-auto mb-2" />
          <p class="text-center text-secondary">
            No active LLM provider. Choose one below to get started.
          </p>
        </div>
      {/if}

      <!-- Provider Categories -->

      <!-- Cloud Providers -->
      <div class="provider-section">
        <div class="section-header">
          <div class="flex items-center gap-2">
            <Cloud size={18} class="text-blue-600" />
            <h2 class="text-lg font-semibold text-primary">Cloud Providers</h2>
          </div>
          <p class="text-sm text-secondary">Requires API key</p>
        </div>

        <div class="provider-grid">
          {#each cloudProviders as provider}
            <div class="provider-card {provider.status}">
              <div class="provider-header">
                <div class="flex items-center gap-3">
                  <span class="text-xl">{provider.icon}</span>
                  <div class="flex-1">
                    <h3 class="provider-name">{provider.name}</h3>
                    <p class="provider-description">{provider.description}</p>
                  </div>
                  {#if provider}
                    {@const StatusIcon = getProviderStatusIcon(provider)}
                    <StatusIcon size={16} class={getProviderStatusColor(provider)} />
                  {/if}
                </div>
              </div>

              <div class="provider-actions">
                {#if provider.status === 'unconfigured'}
                  <Button
                    variant="primary"
                    size="sm"
                    onclick={() => openQuickSetup(provider)}
                    class="w-full"
                  >
                    <Plus size={14} />
                    Setup
                  </Button>
                {:else}
                  <div class="flex gap-2">
                    {#if provider.status === 'configured'}
                      <Button
                        variant="primary"
                        size="sm"
                        onclick={() => setActiveProvider(provider.id)}
                        class="flex-1"
                      >
                        Activate
                      </Button>
                    {/if}
                    <Button
                      variant="ghost"
                      size="sm"
                      onclick={() => openQuickSetup(provider)}
                    >
                      <Edit3 size={14} />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onclick={() => removeProvider(provider.id)}
                    >
                      <Trash2 size={14} />
                    </Button>
                  </div>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </div>

      <!-- Local Providers -->
      <div class="provider-section">
        <div class="section-header">
          <div class="flex items-center gap-2">
            <Monitor size={18} class="text-green-600" />
            <h2 class="text-lg font-semibold text-primary">Local Providers</h2>
          </div>
          <p class="text-sm text-secondary">Run on your machine</p>
        </div>

        <div class="provider-grid">
          {#each localProviders as provider}
            <div class="provider-card {provider.status}">
              <div class="provider-header">
                <div class="flex items-center gap-3">
                  <span class="text-xl">{provider.icon}</span>
                  <div class="flex-1">
                    <h3 class="provider-name">{provider.name}</h3>
                    <p class="provider-description">{provider.description}</p>
                  </div>
                  {#if provider}
                    {@const StatusIcon = getProviderStatusIcon(provider)}
                    <StatusIcon size={16} class={getProviderStatusColor(provider)} />
                  {/if}
                </div>
              </div>

              <div class="provider-actions">
                {#if provider.status === 'unconfigured'}
                  <Button
                    variant="secondary"
                    size="sm"
                    onclick={() => openQuickSetup(provider)}
                    class="w-full"
                  >
                    <Plus size={14} />
                    Connect
                  </Button>
                {:else}
                  <div class="flex gap-2">
                    {#if provider.status === 'configured'}
                      <Button
                        variant="primary"
                        size="sm"
                        onclick={() => setActiveProvider(provider.id)}
                        class="flex-1"
                      >
                        Activate
                      </Button>
                    {/if}
                    <Button
                      variant="ghost"
                      size="sm"
                      onclick={() => openQuickSetup(provider)}
                    >
                      <Edit3 size={14} />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onclick={() => removeProvider(provider.id)}
                    >
                      <Trash2 size={14} />
                    </Button>
                  </div>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </div>

    {:else if activeTab === 'global'}
      <div class="settings-section">
        <h2 class="section-title">Global Settings</h2>
        <p class="mb-6 text-secondary">Application-wide preferences</p>

        <div class="setting-group">
          <label class="setting-label">
            <input type="checkbox" class="setting-checkbox" />
            Auto-start sampling on server connection
          </label>
          <p class="setting-description">
            Automatically begin sampling when a new MCP server connects
          </p>
        </div>

        <div class="setting-group">
          <label for="settings-default-temperature" class="setting-label">Default Temperature</label>
          <input id="settings-default-temperature" type="range" min="0" max="2" step="0.1" value="0.7" class="setting-slider" />
          <p class="setting-description">
            Default temperature for sampling requests (0.7 recommended)
          </p>
        </div>
      </div>

    {:else if activeTab === 'usage'}
      <div class="settings-section">
        <h2 class="section-title">Usage & Costs</h2>
        <p class="mb-6 text-secondary">Track your LLM usage and spending</p>

        {#if configuredProviders.length > 0}
          <div class="usage-grid">
            {#each configuredProviders as provider}
              {#if provider.usage_stats}
                <div class="usage-card">
                  <div class="flex items-center gap-3 mb-3">
                    <span class="text-xl">{provider.icon}</span>
                    <h3 class="font-semibold">{provider.name}</h3>
                  </div>
                  <div class="usage-stats">
                    <div class="usage-stat">
                      <span class="usage-label">Requests</span>
                      <span class="usage-value">{provider.usage_stats.total_requests}</span>
                    </div>
                    <div class="usage-stat">
                      <span class="usage-label">Success Rate</span>
                      <span class="usage-value">
                        {Math.round((provider.usage_stats.successful_requests / Math.max(provider.usage_stats.total_requests, 1)) * 100)}%
                      </span>
                    </div>
                    <div class="usage-stat">
                      <span class="usage-label">Total Cost</span>
                      <span class="usage-value">${provider.usage_stats.total_cost.toFixed(3)}</span>
                    </div>
                  </div>
                </div>
              {/if}
            {/each}
          </div>
        {:else}
          <div class="empty-state">
            <Cpu size={24} class="text-gray-400 mx-auto mb-2" />
            <p class="text-center text-secondary">
              No usage data yet. Configure a provider to start tracking.
            </p>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<!-- =============================================== -->
<!-- QUICK SETUP MODAL -->
<!-- =============================================== -->

{#if quickSetup.show && quickSetup.provider}
  <div class="modal-overlay" onclick={closeQuickSetup} onkeydown={(e) => e.key === 'Escape' && closeQuickSetup()} role="button" tabindex="0">
    <div class="modal-content" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <div class="flex items-center gap-3">
          <span class="text-2xl">{quickSetup.provider.icon}</span>
          <div>
            <h2 class="text-xl font-semibold text-primary">
              Setup {quickSetup.provider.name}
            </h2>
            <p class="text-sm text-secondary">
              {quickSetup.provider.description}
            </p>
          </div>
        </div>
        <button onclick={closeQuickSetup} class="hover:opacity-75 text-tertiary">
          ×
        </button>
      </div>

      <div class="modal-body">
        <!-- API Key (if required) -->
        {#if quickSetup.provider.requires_api_key}
          <div class="form-group">
            <label for="settings-api-key" class="form-label">API Key</label>
            <div class="relative">
              <input
                id="settings-api-key"
                type={showApiKeys ? "text" : "password"}
                bind:value={quickSetup.api_key}
                placeholder="sk-..."
                class="form-input pr-10"
              />
              <button
                onclick={() => showApiKeys = !showApiKeys}
                class="absolute right-3 top-1/2 -translate-y-1/2 hover:opacity-75 text-tertiary"
                aria-label={showApiKeys ? "Hide API key" : "Show API key"}
              >
                {#if showApiKeys}
                  <EyeOff size={16} />
                {:else}
                  <Eye size={16} />
                {/if}
              </button>
            </div>
          </div>
        {/if}

        <!-- Base URL -->
        <div class="form-group">
          <label for="settings-base-url" class="form-label">
            Base URL
            {#if quickSetup.provider.type === 'local'}
              <span class="text-xs text-gray-500">(Make sure your local server is running)</span>
            {/if}
          </label>
          <input
            id="settings-base-url"
            type="text"
            bind:value={quickSetup.base_url}
            class="form-input"
            placeholder={quickSetup.provider.default_base_url}
          />
        </div>

        <!-- Test Result -->
        {#if quickSetup.test_result !== 'none'}
          <div class="test-result {quickSetup.test_result}">
            {#if quickSetup.test_result === 'success'}
              <CheckCircle size={16} class="text-green-600" />
            {:else}
              <AlertCircle size={16} class="text-red-600" />
            {/if}
            <span>{quickSetup.test_message}</span>
          </div>
        {/if}

        <!-- Model Selection (shown after successful test) -->
        {#if quickSetup.test_result === 'success' && quickSetup.available_models.length > 0}
          <div class="form-group">
            <label for="settings-default-model" class="form-label">Default Model</label>
            <select
              id="settings-default-model"
              bind:value={quickSetup.selected_model}
              class="form-input"
            >
              {#each quickSetup.available_models as model}
                <option value={model}>{model}</option>
              {/each}
            </select>
            <p class="text-xs text-gray-500 mt-1">
              This will be the default model for sampling requests
            </p>
          </div>
        {/if}
      </div>

      <div class="modal-actions">
        <Button
          variant="secondary"
          onclick={testConnection}
          disabled={quickSetup.testing}
          loading={quickSetup.testing}
        >
          <RefreshCw size={14} />
          Test Connection
        </Button>
        <Button
          variant="ghost"
          onclick={closeQuickSetup}
        >
          Cancel
        </Button>
        <Button
          variant="primary"
          onclick={saveProvider}
          disabled={quickSetup.testing || (quickSetup.provider.requires_api_key && !quickSetup.api_key.trim())}
        >
          Save & Activate
        </Button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Container */
  .settings-container {
    @apply h-full flex flex-col;
    background: var(--mcp-surface-primary);
  }

  /* Header */
  .settings-header {
    @apply flex items-center justify-between p-6 border-b;
    border-color: var(--mcp-border-primary);
  }

  /* Tab Navigation */
  .tab-navigation {
    @apply flex border-b;
    border-color: var(--mcp-border-primary);
    background: var(--mcp-surface-secondary);
  }

  .tab-button {
    @apply flex items-center gap-2 px-6 py-3 text-sm font-medium border-b-2 border-transparent transition-colors;
    color: var(--mcp-text-secondary);
  }

  .tab-button:hover {
    color: var(--mcp-text-primary);
    border-color: var(--mcp-border-secondary);
  }

  .tab-button.active {
    @apply text-mcp-primary-600 border-mcp-primary-600;
    background: var(--mcp-surface-primary);
  }

  /* Tab Content */
  .tab-content {
    @apply flex-1 overflow-y-auto p-6 space-y-6;
  }

  /* Active Provider Card */
  .active-provider-card {
    @apply p-4 border rounded-lg;
    background: var(--mcp-success-50);
    border-color: var(--mcp-success-200);
  }

  .no-active-provider {
    @apply p-4 border rounded-lg;
    background: var(--mcp-warning-50);
    border-color: var(--mcp-warning-200);
  }

  /* Provider Sections */
  .provider-section {
    @apply space-y-4;
  }

  .section-header {
    @apply flex items-center justify-between;
  }

  .provider-grid {
    @apply grid grid-cols-1 md:grid-cols-2 gap-4;
  }

  /* Provider Cards */
  .provider-card {
    @apply p-4 border rounded-lg transition-all hover:shadow-md;
    background: var(--mcp-card-background);
    border-color: var(--mcp-border-primary);
  }

  .provider-card.active {
    background: var(--mcp-success-50);
    border-color: var(--mcp-success-200);
  }

  .provider-card.configured {
    background: var(--mcp-primary-50);
    border-color: var(--mcp-primary-200);
  }

  .provider-card.error {
    background: var(--mcp-error-50);
    border-color: var(--mcp-error-200);
  }

  .provider-header {
    @apply mb-3;
  }

  .provider-name {
    @apply font-semibold;
    color: var(--mcp-text-primary);
  }

  .provider-description {
    @apply text-xs mt-1;
    color: var(--mcp-text-secondary);
  }

  .provider-actions {
    @apply mt-3;
  }

  /* Settings Section */
  .settings-section {
    @apply space-y-6;
  }

  .section-title {
    @apply text-lg font-semibold;
    color: var(--mcp-text-primary);
  }

  .setting-group {
    @apply p-4 border rounded-lg;
    border-color: var(--mcp-border-primary);
  }

  .setting-label {
    @apply flex items-center gap-2 text-sm font-medium;
    color: var(--mcp-text-primary);
  }

  .setting-checkbox {
    @apply w-4 h-4 text-mcp-primary-600 rounded;
  }

  .setting-slider {
    @apply w-full mt-2;
  }

  .setting-description {
    @apply text-xs mt-1;
    color: var(--mcp-text-secondary);
  }

  /* Usage Cards */
  .usage-grid {
    @apply grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4;
  }

  .usage-card {
    @apply p-4 border rounded-lg;
    border-color: var(--mcp-border-primary);
  }

  .usage-stats {
    @apply space-y-2;
  }

  .usage-stat {
    @apply flex justify-between;
  }

  .usage-label {
    @apply text-xs;
    color: var(--mcp-text-secondary);
  }

  .usage-value {
    @apply text-sm font-medium;
    color: var(--mcp-text-primary);
  }

  /* Empty State */
  .empty-state {
    @apply py-12 text-center;
  }

  /* Modal */
  .modal-overlay {
    @apply fixed inset-0 bg-black/50 flex items-center justify-center z-50;
  }

  .modal-content {
    @apply rounded-lg shadow-xl max-w-md w-full mx-4;
    background: var(--mcp-surface-elevated);
  }

  .modal-header {
    @apply flex items-center justify-between p-6 border-b;
    border-color: var(--mcp-border-primary);
  }

  .modal-body {
    @apply p-6 space-y-4;
  }

  .modal-actions {
    @apply flex items-center justify-end gap-3 p-6 border-t;
    border-color: var(--mcp-border-primary);
  }

  /* Form Elements */
  .form-group {
    @apply space-y-2;
  }

  .form-label {
    @apply block text-sm font-medium;
    color: var(--mcp-text-primary);
  }

  .form-input {
    @apply w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-mcp-primary-500 focus:border-mcp-primary-500;
    background: var(--mcp-input-background);
    border-color: var(--mcp-border-primary);
    color: var(--mcp-text-primary);
  }

  /* Test Result */
  .test-result {
    @apply flex items-center gap-2 p-3 rounded-lg text-sm;
  }

  .test-result.success {
    background: var(--mcp-success-50);
    color: var(--mcp-success-700);
  }

  .test-result.error {
    background: var(--mcp-error-50);
    color: var(--mcp-error-700);
  }
</style>