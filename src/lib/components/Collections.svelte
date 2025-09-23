<script lang="ts">
  import { onMount } from 'svelte';
  import { serverStore, type ServerInfo, type ToolExecution } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import {
    FolderPlus,
    Folder,
    File,
    Play,
    Pause,
    Square,
    RefreshCw,
    AlertCircle,
    CheckCircle,
    Clock,
    Copy,
    Download,
    Upload,
    Settings,
    Edit,
    Trash2,
    Plus,
    Search,
    Filter,
    Star,
    BookOpen,
    Zap,
    Database,
    FileText,
    Users,
    Timer,
    BarChart3,
    Eye,
    Save,
    Share
  } from 'lucide-svelte';

  interface CollectionItem {
    id: string;
    type: 'workflow' | 'tool-sequence' | 'test-scenario' | 'prompt-collection' | 'resource-set';
    name: string;
    description: string;
    tags: string[];
    createdAt: string;
    updatedAt: string;
    starred: boolean;
    shared: boolean;
    author: string;
    // Type-specific data
    data: any;
  }

  interface Workflow {
    steps: Array<{
      id: string;
      type: 'tool-call' | 'prompt' | 'resource-read' | 'delay' | 'condition';
      name: string;
      serverId?: string;
      config: any;
      dependsOn?: string[];
    }>;
    variables: Record<string, any>;
  }

  interface TestScenario {
    description: string;
    setup: Array<{ action: string; config: any }>;
    tests: Array<{
      name: string;
      type: 'assertion' | 'performance' | 'regression';
      config: any;
      expected: any;
    }>;
    teardown: Array<{ action: string; config: any }>;
  }

  let servers: ServerInfo[] = $state([]);
  let selectedServerId: string | undefined = $state(undefined);
  let collections: CollectionItem[] = $state([]);
  let selectedCollection: CollectionItem | null = $state(null);
  let loading = $state(false);
  let searchQuery = $state('');
  let selectedType = $state('all');
  let showStarredOnly = $state(false);
  let isCreating = $state(false);
  let executingWorkflow = $state(false);

  // New collection form
  let newCollection = $state({
    type: 'workflow' as CollectionItem['type'],
    name: '',
    description: '',
    tags: '',
    data: {}
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

  const filteredCollections = $derived(() => {
    let filtered = collections;

    // Filter by type
    if (selectedType !== 'all') {
      filtered = filtered.filter(item => item.type === selectedType);
    }

    // Filter by starred
    if (showStarredOnly) {
      filtered = filtered.filter(item => item.starred);
    }

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(item =>
        item.name.toLowerCase().includes(query) ||
        item.description.toLowerCase().includes(query) ||
        item.tags.some(tag => tag.toLowerCase().includes(query))
      );
    }

    return filtered.sort((a, b) => {
      // Starred items first
      if (a.starred !== b.starred) {
        return b.starred ? 1 : -1;
      }
      // Then by updated date
      return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime();
    });
  });

  function selectServer(serverId: string) {
    serverStore.selectServer(serverId);
  }

  function createSampleCollections() {
    const samples: CollectionItem[] = [
      {
        id: crypto.randomUUID(),
        type: 'workflow',
        name: 'Code Review Workflow',
        description: 'Automated code review process with multiple validation steps',
        tags: ['development', 'quality', 'automation'],
        createdAt: new Date(Date.now() - 86400000).toISOString(),
        updatedAt: new Date(Date.now() - 3600000).toISOString(),
        starred: true,
        shared: false,
        author: 'You',
        data: {
          steps: [
            {
              id: 'lint-check',
              type: 'tool-call',
              name: 'Run Linter',
              serverId: selectedServerId,
              config: { tool: 'lint_code', parameters: { language: '{{language}}', strict: true } }
            },
            {
              id: 'security-scan',
              type: 'tool-call',
              name: 'Security Scan',
              serverId: selectedServerId,
              config: { tool: 'security_scan', parameters: { code: '{{code}}' } },
              dependsOn: ['lint-check']
            },
            {
              id: 'generate-review',
              type: 'prompt',
              name: 'Generate Review',
              config: { prompt: 'code_review', arguments: { code: '{{code}}', issues: '{{security-scan.result}}' } },
              dependsOn: ['security-scan']
            }
          ],
          variables: { language: 'javascript', code: '', strict: true }
        }
      },
      {
        id: crypto.randomUUID(),
        type: 'test-scenario',
        name: 'API Load Test',
        description: 'Performance testing for API endpoints under various load conditions',
        tags: ['performance', 'testing', 'api'],
        createdAt: new Date(Date.now() - 172800000).toISOString(),
        updatedAt: new Date(Date.now() - 7200000).toISOString(),
        starred: false,
        shared: true,
        author: 'Team',
        data: {
          description: 'Load test critical API endpoints',
          setup: [
            { action: 'connect-server', config: { serverId: selectedServerId } },
            { action: 'prepare-data', config: { dataset: 'test-users' } }
          ],
          tests: [
            {
              name: 'Response Time Under Load',
              type: 'performance',
              config: { endpoint: '/api/users', concurrent: 100, duration: 60 },
              expected: { avgResponseTime: '<500ms', errorRate: '<1%' }
            },
            {
              name: 'Memory Usage',
              type: 'assertion',
              config: { metric: 'memory_usage' },
              expected: { max: '512MB' }
            }
          ],
          teardown: [
            { action: 'cleanup-data', config: { dataset: 'test-users' } }
          ]
        }
      },
      {
        id: crypto.randomUUID(),
        type: 'tool-sequence',
        name: 'Documentation Pipeline',
        description: 'Generate comprehensive documentation from code and API specs',
        tags: ['documentation', 'automation', 'pipeline'],
        createdAt: new Date(Date.now() - 259200000).toISOString(),
        updatedAt: new Date(Date.now() - 1800000).toISOString(),
        starred: true,
        shared: false,
        author: 'You',
        data: {
          sequence: [
            { tool: 'extract_api_schema', parameters: { source: 'openapi.json' } },
            { tool: 'generate_examples', parameters: { schema: '{{step1.result}}' } },
            { tool: 'create_documentation', parameters: { schema: '{{step1.result}}', examples: '{{step2.result}}' } }
          ]
        }
      },
      {
        id: crypto.randomUUID(),
        type: 'prompt-collection',
        name: 'Development Prompts',
        description: 'Collection of useful prompts for software development tasks',
        tags: ['prompts', 'development', 'templates'],
        createdAt: new Date(Date.now() - 345600000).toISOString(),
        updatedAt: new Date(Date.now() - 900000).toISOString(),
        starred: false,
        shared: true,
        author: 'Community',
        data: {
          prompts: [
            { name: 'code_review', description: 'Review code for best practices' },
            { name: 'bug_analysis', description: 'Analyze and suggest fixes for bugs' },
            { name: 'optimization', description: 'Suggest performance optimizations' }
          ]
        }
      }
    ];

    collections = [...samples, ...collections];
    uiStore.showSuccess('Sample collections created');
  }

  async function executeWorkflow(workflow: Workflow) {
    if (!selectedServerId) {
      uiStore.showError('No server selected');
      return;
    }

    executingWorkflow = true;
    try {
      uiStore.showInfo('Starting workflow execution...');

      // Simulate workflow execution
      for (const step of workflow.steps) {
        await new Promise(resolve => setTimeout(resolve, 1000 + Math.random() * 2000));

        switch (step.type) {
          case 'tool-call':
            // Simulate tool call
            if (step.config.tool) {
              uiStore.showInfo(`Executing: ${step.name} (${step.config.tool})`);
            }
            break;
          case 'prompt':
            uiStore.showInfo(`Processing: ${step.name}`);
            break;
          case 'delay':
            uiStore.showInfo(`Waiting: ${step.name}`);
            break;
        }
      }

      uiStore.showSuccess('Workflow completed successfully');
    } catch (error) {
      uiStore.showError(`Workflow failed: ${error}`);
    } finally {
      executingWorkflow = false;
    }
  }

  async function runTestScenario(scenario: TestScenario) {
    uiStore.showInfo('Running test scenario...');

    try {
      // Setup
      for (const setup of scenario.setup) {
        await new Promise(resolve => setTimeout(resolve, 500));
        uiStore.showInfo(`Setup: ${setup.action}`);
      }

      // Run tests
      for (const test of scenario.tests) {
        await new Promise(resolve => setTimeout(resolve, 1000));
        const passed = Math.random() > 0.2; // 80% pass rate
        if (passed) {
          uiStore.showSuccess(`✓ ${test.name}`);
        } else {
          uiStore.showError(`✗ ${test.name}`);
        }
      }

      // Teardown
      for (const teardown of scenario.teardown) {
        await new Promise(resolve => setTimeout(resolve, 300));
        uiStore.showInfo(`Teardown: ${teardown.action}`);
      }

      uiStore.showSuccess('Test scenario completed');
    } catch (error) {
      uiStore.showError(`Test scenario failed: ${error}`);
    }
  }

  function toggleStar(collection: CollectionItem) {
    collection.starred = !collection.starred;
    collection.updatedAt = new Date().toISOString();
    collections = [...collections];
  }

  function deleteCollection(collection: CollectionItem) {
    if (confirm(`Are you sure you want to delete "${collection.name}"?`)) {
      collections = collections.filter(c => c.id !== collection.id);
      if (selectedCollection?.id === collection.id) {
        selectedCollection = null;
      }
      uiStore.showSuccess('Collection deleted');
    }
  }

  function duplicateCollection(collection: CollectionItem) {
    const duplicate: CollectionItem = {
      ...collection,
      id: crypto.randomUUID(),
      name: `${collection.name} (Copy)`,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      starred: false,
      shared: false
    };

    collections = [duplicate, ...collections];
    selectedCollection = duplicate;
    uiStore.showSuccess('Collection duplicated');
  }

  function startCreating(type: CollectionItem['type']) {
    newCollection = {
      type,
      name: '',
      description: '',
      tags: '',
      data: type === 'workflow' ? { steps: [], variables: {} } :
            type === 'test-scenario' ? { description: '', setup: [], tests: [], teardown: [] } :
            type === 'tool-sequence' ? { sequence: [] } :
            type === 'prompt-collection' ? { prompts: [] } :
            type === 'resource-set' ? { resources: [] } : {}
    };
    isCreating = true;
  }

  function saveCollection() {
    if (!newCollection.name.trim()) {
      uiStore.showError('Collection name is required');
      return;
    }

    const collection: CollectionItem = {
      id: crypto.randomUUID(),
      type: newCollection.type,
      name: newCollection.name.trim(),
      description: newCollection.description.trim(),
      tags: newCollection.tags.split(',').map(t => t.trim()).filter(t => t),
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      starred: false,
      shared: false,
      author: 'You',
      data: newCollection.data
    };

    collections = [collection, ...collections];
    selectedCollection = collection;
    isCreating = false;

    uiStore.showSuccess(`${collection.type.replace('-', ' ')} created successfully`);
  }

  function getTypeIcon(type: string) {
    switch (type) {
      case 'workflow': return Zap;
      case 'tool-sequence': return Settings;
      case 'test-scenario': return BarChart3;
      case 'prompt-collection': return FileText;
      case 'resource-set': return Database;
      default: return File;
    }
  }

  function getTypeColor(type: string) {
    switch (type) {
      case 'workflow': return 'text-blue-600 bg-blue-100';
      case 'tool-sequence': return 'text-green-600 bg-green-100';
      case 'test-scenario': return 'text-purple-600 bg-purple-100';
      case 'prompt-collection': return 'text-orange-600 bg-orange-100';
      case 'resource-set': return 'text-indigo-600 bg-indigo-100';
      default: return 'text-gray-600 bg-gray-100';
    }
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    uiStore.showSuccess('Copied to clipboard');
  }

  onMount(() => {
    // Create sample collections after a short delay
    setTimeout(() => createSampleCollections(), 500);
  });
</script>

<div class="h-full flex bg-gray-50">
  <!-- Left Panel: Collections List -->
  <div class="w-1/3 bg-white border-r border-gray-200 flex flex-col">
    <!-- Header -->
    <div class="p-4 border-b border-gray-200">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-lg font-semibold text-gray-900">Collections</h2>
        <div class="flex items-center space-x-2">
          <button
            onclick={() => startCreating('workflow')}
            class="btn-secondary text-sm"
            title="Create new collection"
          >
            <Plus size={14} />
          </button>
        </div>
      </div>

      <!-- Filters -->
      <div class="space-y-3">
        <!-- Search -->
        <div class="relative">
          <Search size={16} class="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400" />
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search collections..."
            class="form-input pl-10 text-sm"
          />
        </div>

        <!-- Type Filter -->
        <div class="flex items-center space-x-2">
          <Filter size={14} class="text-gray-400" />
          <select bind:value={selectedType} class="form-select text-xs">
            <option value="all">All Types</option>
            <option value="workflow">Workflows</option>
            <option value="tool-sequence">Tool Sequences</option>
            <option value="test-scenario">Test Scenarios</option>
            <option value="prompt-collection">Prompt Collections</option>
            <option value="resource-set">Resource Sets</option>
          </select>
        </div>

        <!-- Options -->
        <label class="flex items-center text-sm">
          <input type="checkbox" bind:checked={showStarredOnly} class="form-checkbox mr-2" />
          Show starred only
        </label>
      </div>
    </div>

    <!-- Collections List -->
    <div class="flex-1 overflow-y-auto">
      {#if loading}
        <div class="flex items-center justify-center p-8">
          <RefreshCw size={24} class="animate-spin text-gray-400 mr-3" />
          <span class="text-gray-600">Loading collections...</span>
        </div>
      {:else if filteredCollections.length === 0}
        <div class="text-center p-8">
          <Folder size={48} class="mx-auto text-gray-400 mb-4" />
          <h3 class="text-lg font-medium text-gray-900 mb-2">No Collections Found</h3>
          <p class="text-gray-600 mb-4">
            {searchQuery || selectedType !== 'all' || showStarredOnly
              ? 'No collections match your filters'
              : 'Create your first collection to get started'}
          </p>
          {#if !searchQuery && selectedType === 'all' && !showStarredOnly}
            <div class="flex flex-col space-y-2">
              <button onclick={() => startCreating('workflow')} class="btn-primary text-sm">
                <Plus size={14} class="mr-1" />
                Create Workflow
              </button>
              <button onclick={createSampleCollections} class="btn-secondary text-sm">
                <Download size={14} class="mr-1" />
                Load Samples
              </button>
            </div>
          {/if}
        </div>
      {:else}
        <div class="p-4 space-y-3">
          {#each filteredCollections as collection}
            <button
              onclick={() => selectedCollection = collection}
              class="w-full p-3 text-left bg-gray-50 hover:bg-gray-100 rounded-lg border border-gray-200 transition-colors
                     {selectedCollection?.id === collection.id ? 'ring-2 ring-mcp-primary-500 bg-mcp-primary-50' : ''}"
            >
              <div class="flex items-start justify-between mb-2">
                <div class="flex items-center">
                  {#if collection}
                    {@const IconComponent = getTypeIcon(collection.type)}
                    <IconComponent size={16} class="mr-2 text-gray-500" />
                  {/if}
                  <span class="text-sm font-medium text-gray-900 truncate">{collection.name}</span>
                </div>
                <div class="flex items-center space-x-1">
                  {#if collection.starred}
                    <Star size={12} class="text-yellow-500 fill-current" />
                  {/if}
                  {#if collection.shared}
                    <Users size={12} class="text-blue-500" />
                  {/if}
                </div>
              </div>

              <p class="text-xs text-gray-600 mb-2 line-clamp-2">{collection.description}</p>

              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-2">
                  <span class="text-xs px-2 py-1 rounded {getTypeColor(collection.type)}">
                    {collection.type.replace('-', ' ')}
                  </span>
                  {#if collection.tags.length > 0}
                    <span class="text-xs text-gray-500">
                      {collection.tags.slice(0, 2).join(', ')}{collection.tags.length > 2 ? '...' : ''}
                    </span>
                  {/if}
                </div>
                <span class="text-xs text-gray-500">
                  {new Date(collection.updatedAt).toLocaleDateString()}
                </span>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <!-- Right Panel: Collection Details -->
  <div class="flex-1 flex flex-col bg-white">
    {#if isCreating}
      <!-- Create Collection Form -->
      <div class="p-4 border-b border-gray-200">
        <h3 class="text-lg font-semibold text-gray-900">Create New Collection</h3>
      </div>

      <div class="flex-1 overflow-y-auto p-4">
        <form
          onsubmit={(e) => {
            e.preventDefault();
            saveCollection();
          }}
          class="space-y-4 max-w-2xl"
        >
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Type</label>
            <select bind:value={newCollection.type} class="form-select">
              <option value="workflow">Workflow</option>
              <option value="tool-sequence">Tool Sequence</option>
              <option value="test-scenario">Test Scenario</option>
              <option value="prompt-collection">Prompt Collection</option>
              <option value="resource-set">Resource Set</option>
            </select>
          </div>

          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Name *</label>
            <input
              type="text"
              bind:value={newCollection.name}
              placeholder="Enter collection name..."
              class="form-input"
              required
            />
          </div>

          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
            <textarea
              bind:value={newCollection.description}
              placeholder="Describe what this collection does..."
              class="form-input h-20 resize-none"
            ></textarea>
          </div>

          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Tags</label>
            <input
              type="text"
              bind:value={newCollection.tags}
              placeholder="development, automation, testing (comma-separated)"
              class="form-input"
            />
          </div>

          <div class="flex items-center space-x-3 pt-4">
            <button type="submit" class="btn-primary">
              <Save size={14} class="mr-1" />
              Create Collection
            </button>
            <button
              type="button"
              onclick={() => isCreating = false}
              class="btn-secondary"
            >
              Cancel
            </button>
          </div>
        </form>
      </div>

    {:else if selectedCollection}
      <!-- Collection Details -->
      <div class="p-4 border-b border-gray-200">
        <div class="flex items-center justify-between">
          <div class="flex items-center">
            {#if selectedCollection}
              {@const IconComponent = getTypeIcon(selectedCollection.type)}
              <IconComponent size={20} class="mr-3 text-gray-500" />
            {/if}
            <div>
              <h3 class="text-lg font-semibold text-gray-900">{selectedCollection.name}</h3>
              <div class="flex items-center space-x-4 mt-1 text-sm text-gray-600">
                <span class="px-2 py-1 rounded text-xs {getTypeColor(selectedCollection.type)}">
                  {selectedCollection.type.replace('-', ' ')}
                </span>
                <span>By {selectedCollection.author}</span>
                <span>{new Date(selectedCollection.updatedAt).toLocaleDateString()}</span>
              </div>
            </div>
          </div>

          <div class="flex items-center space-x-2">
            <button
              onclick={() => toggleStar(selectedCollection)}
              class="p-2 text-gray-400 hover:text-yellow-500 rounded"
            >
              <Star size={16} class="{selectedCollection.starred ? 'text-yellow-500 fill-current' : ''}" />
            </button>

            {#if selectedCollection.type === 'workflow'}
              <button
                onclick={() => executeWorkflow(selectedCollection.data)}
                disabled={executingWorkflow}
                class="btn-primary text-sm"
              >
                <Play size={14} class="{executingWorkflow ? 'animate-pulse' : ''} mr-1" />
                {executingWorkflow ? 'Running...' : 'Execute'}
              </button>
            {:else if selectedCollection.type === 'test-scenario'}
              <button
                onclick={() => runTestScenario(selectedCollection.data)}
                class="btn-primary text-sm"
              >
                <Play size={14} class="mr-1" />
                Run Tests
              </button>
            {/if}

            <button
              onclick={() => duplicateCollection(selectedCollection)}
              class="btn-secondary text-sm"
            >
              <Copy size={14} />
            </button>

            <button
              onclick={() => deleteCollection(selectedCollection)}
              class="btn-secondary text-sm text-red-600 hover:bg-red-50"
            >
              <Trash2 size={14} />
            </button>
          </div>
        </div>

        {#if selectedCollection.description}
          <p class="text-sm text-gray-700 mt-3">{selectedCollection.description}</p>
        {/if}

        {#if selectedCollection.tags.length > 0}
          <div class="flex flex-wrap gap-1 mt-3">
            {#each selectedCollection.tags as tag}
              <span class="text-xs bg-gray-100 text-gray-700 px-2 py-1 rounded">{tag}</span>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Collection Content -->
      <div class="flex-1 overflow-y-auto p-4">
        {#if selectedCollection.type === 'workflow'}
          <div class="space-y-4">
            <div class="flex items-center justify-between">
              <h4 class="text-sm font-medium text-gray-900">Workflow Steps</h4>
              <span class="text-xs text-gray-500">
                {selectedCollection.data.steps?.length || 0} step(s)
              </span>
            </div>

            {#if selectedCollection.data.steps?.length > 0}
              <div class="space-y-3">
                {#each selectedCollection.data.steps as step, index}
                  <div class="border border-gray-200 rounded-lg p-3">
                    <div class="flex items-center justify-between mb-2">
                      <div class="flex items-center">
                        <span class="w-6 h-6 bg-blue-100 text-blue-600 rounded-full text-xs flex items-center justify-center mr-3">
                          {index + 1}
                        </span>
                        <span class="text-sm font-medium text-gray-900">{step.name}</span>
                      </div>
                      <span class="text-xs text-gray-500 bg-gray-100 px-2 py-1 rounded">
                        {step.type}
                      </span>
                    </div>
                    {#if step.dependsOn?.length}
                      <p class="text-xs text-gray-600 mb-2">
                        Depends on: {step.dependsOn.join(', ')}
                      </p>
                    {/if}
                    <div class="bg-gray-50 rounded p-2 font-mono text-xs">
                      {JSON.stringify(step.config, null, 2)}
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="text-sm text-gray-500">No steps defined</p>
            {/if}
          </div>

        {:else if selectedCollection.type === 'test-scenario'}
          <div class="space-y-6">
            {#if selectedCollection.data.setup?.length}
              <div>
                <h4 class="text-sm font-medium text-gray-900 mb-3">Setup</h4>
                <div class="space-y-2">
                  {#each selectedCollection.data.setup as setup}
                    <div class="flex items-center justify-between p-2 bg-blue-50 rounded">
                      <span class="text-sm text-blue-900">{setup.action}</span>
                      <span class="text-xs text-blue-600">{JSON.stringify(setup.config)}</span>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}

            {#if selectedCollection.data.tests?.length}
              <div>
                <h4 class="text-sm font-medium text-gray-900 mb-3">Tests</h4>
                <div class="space-y-3">
                  {#each selectedCollection.data.tests as test}
                    <div class="border border-gray-200 rounded-lg p-3">
                      <div class="flex items-center justify-between mb-2">
                        <span class="text-sm font-medium text-gray-900">{test.name}</span>
                        <span class="text-xs text-purple-600 bg-purple-100 px-2 py-1 rounded">
                          {test.type}
                        </span>
                      </div>
                      <div class="grid grid-cols-2 gap-3 text-xs">
                        <div>
                          <p class="text-gray-600 mb-1">Config:</p>
                          <div class="bg-gray-50 rounded p-2 font-mono">
                            {JSON.stringify(test.config, null, 2)}
                          </div>
                        </div>
                        <div>
                          <p class="text-gray-600 mb-1">Expected:</p>
                          <div class="bg-green-50 rounded p-2 font-mono">
                            {JSON.stringify(test.expected, null, 2)}
                          </div>
                        </div>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>

        {:else if selectedCollection.type === 'tool-sequence'}
          <div>
            <h4 class="text-sm font-medium text-gray-900 mb-3">Tool Sequence</h4>
            {#if selectedCollection.data.sequence?.length}
              <div class="space-y-3">
                {#each selectedCollection.data.sequence as step, index}
                  <div class="flex items-center p-3 border border-gray-200 rounded-lg">
                    <span class="w-6 h-6 bg-green-100 text-green-600 rounded-full text-xs flex items-center justify-center mr-3">
                      {index + 1}
                    </span>
                    <div class="flex-1">
                      <p class="text-sm font-medium text-gray-900">{step.tool}</p>
                      <p class="text-xs text-gray-600">{JSON.stringify(step.parameters)}</p>
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="text-sm text-gray-500">No tools in sequence</p>
            {/if}
          </div>

        {:else if selectedCollection.type === 'prompt-collection'}
          <div>
            <h4 class="text-sm font-medium text-gray-900 mb-3">Prompts</h4>
            {#if selectedCollection.data.prompts?.length}
              <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                {#each selectedCollection.data.prompts as prompt}
                  <div class="border border-gray-200 rounded-lg p-3">
                    <h5 class="text-sm font-medium text-gray-900 mb-1">{prompt.name}</h5>
                    <p class="text-xs text-gray-600">{prompt.description}</p>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="text-sm text-gray-500">No prompts in collection</p>
            {/if}
          </div>

        {:else}
          <div class="text-center py-12">
            <Eye size={48} class="mx-auto text-gray-400 mb-4" />
            <h3 class="text-lg font-medium text-gray-900 mb-2">Collection Details</h3>
            <p class="text-gray-600">Detailed view for {selectedCollection.type} coming soon</p>
          </div>
        {/if}

        <!-- Raw Data -->
        <div class="mt-8 pt-6 border-t border-gray-200">
          <div class="flex items-center justify-between mb-3">
            <h4 class="text-sm font-medium text-gray-900">Raw Data</h4>
            <button
              onclick={() => copyToClipboard(JSON.stringify(selectedCollection.data, null, 2))}
              class="btn-secondary text-sm"
            >
              <Copy size={14} class="mr-1" />
              Copy
            </button>
          </div>
          <div class="bg-gray-50 rounded-lg p-4 font-mono text-xs overflow-auto max-h-64">
            <pre>{JSON.stringify(selectedCollection.data, null, 2)}</pre>
          </div>
        </div>
      </div>

    {:else}
      <!-- No Collection Selected -->
      <div class="flex items-center justify-center h-full">
        <div class="text-center">
          <Folder size={48} class="mx-auto text-gray-400 mb-4" />
          <h3 class="text-lg font-semibold text-gray-900 mb-2">Select a Collection</h3>
          <p class="text-gray-600 mb-6">Choose a collection from the list to view its details</p>

          <div class="flex flex-col space-y-2">
            <button onclick={() => startCreating('workflow')} class="btn-primary">
              <Plus size={16} class="mr-2" />
              Create New Collection
            </button>
            {#if collections.length === 0}
              <button onclick={createSampleCollections} class="btn-secondary">
                <Download size={16} class="mr-2" />
                Load Sample Collections
              </button>
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>