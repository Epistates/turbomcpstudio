<!--
  World-Class Collections Manager - The "Postman for MCP" Interface

  This component provides the revolutionary Collections UI that leapfrogs MCP Inspector:
  - Cross-server workflows with variable passing
  - Visual workflow designer with drag-and-drop
  - Real-time execution monitoring
  - Advanced testing and validation
  - Collection templates and sharing
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import ExecutionMonitor from './ExecutionMonitor.svelte';

  // NEW: Composed components for operation configuration
  import ToolStepConfig from './collections/ToolStepConfig.svelte';
  import ResourceStepConfig from './collections/ResourceStepConfig.svelte';
  import PromptStepConfig from './collections/PromptStepConfig.svelte';
  import SamplingStepConfig from './collections/SamplingStepConfig.svelte';
  import VariableExtractor from './collections/VariableExtractor.svelte';
  import AssertionEditor from './collections/AssertionEditor.svelte';
  import type {
    Collection,
    WorkflowStep,
    McpOperation,
    WorkflowExecution,
    CollectionTemplate
  } from '$lib/types/collections';
  import {
    Plus,
    Play,
    Pause,
    Square,
    Copy,
    Download,
    Upload,
    Settings,
    Edit,
    Trash2,
    Save,
    FolderOpen,
    FileText,
    GitBranch,
    Clock,
    CheckCircle,
    AlertCircle,
    Zap,
    Database,
    MessageSquare,
    Brain,
    ArrowRight,
    MoreVertical,
    Eye,
    Share2
  } from 'lucide-svelte';

  // State management
  let collections: Collection[] = $state([]);
  let selectedCollection: Collection | null = $state(null);
  let currentExecution: WorkflowExecution | null = $state(null);
  let servers: ServerInfo[] = $state([]);
  let loading = $state(false);
  let saving = $state(false);

  // UI state
  let viewMode: 'list' | 'editor' | 'execution' = $state('list');
  let showTemplates = $state(false);
  let showImportDialog = $state(false);
  let showExportDialog = $state(false);

  // Editor state
  let selectedStep: WorkflowStep | null = $state(null);
  let isEditing = $state(false);

  // Subscribe to stores
  $effect(() => {
    const unsubscribeServers = serverStore.subscribe((state: any) => {
      servers = state.servers.filter((s: any) => s.status?.toLowerCase() === 'connected');
    });

    return () => {
      unsubscribeServers();
    };
  });

  // Load collections on mount
  onMount(async () => {
    await loadCollections();
  });

  async function loadCollections() {
    loading = true;
    try {
      const result = await invoke('list_collections');
      collections = Array.isArray(result) ? result : [];
    } catch (error) {
      console.error('Failed to load collections:', error);
      uiStore.showError(`Failed to load collections: ${error}`);
    } finally {
      loading = false;
    }
  }

  async function createNewCollection() {
    const newCollection: Collection = {
      id: crypto.randomUUID(),
      name: 'New Collection',
      description: 'A new collection for testing MCP workflows',
      tags: [],
      workflow: [],
      variables: {},
      environment: {
        name: 'Default',
        description: 'Default environment',
        servers: {},
        variables: {}
      },
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      created_by: 'user',
      version: '1.0.0',
      last_run: undefined,
      run_count: 0
    };

    collections = [newCollection, ...collections];
    selectedCollection = newCollection;
    viewMode = 'editor';
    isEditing = true;
  }

  async function saveCollection(collection: Collection) {
    saving = true;
    try {
      await invoke('save_collection', { collection });
      collection.updated_at = new Date().toISOString();
      collections = [...collections];
      uiStore.showSuccess('Collection saved successfully');
      isEditing = false;
    } catch (error) {
      console.error('Failed to save collection:', error);
      uiStore.showError(`Failed to save collection: ${error}`);
    } finally {
      saving = false;
    }
  }

  async function deleteCollection(collectionId: string) {
    if (!confirm('Are you sure you want to delete this collection?')) return;

    try {
      await invoke('delete_collection', { id: collectionId });
      collections = collections.filter(c => c.id !== collectionId);
      if (selectedCollection?.id === collectionId) {
        selectedCollection = null;
        viewMode = 'list';
      }
      uiStore.showSuccess('Collection deleted successfully');
    } catch (error) {
      console.error('Failed to delete collection:', error);
      uiStore.showError(`Failed to delete collection: ${error}`);
    }
  }

  async function executeWorkflow(collection: Collection) {
    if (!collection.workflow.length) {
      uiStore.showError('Collection has no workflow steps to execute');
      return;
    }

    try {
      currentExecution = await invoke('execute_workflow', {
        collection: collection,
        userVariables: {}
      });
      viewMode = 'execution';
      uiStore.showSuccess('Workflow execution started');
    } catch (error) {
      console.error('Failed to start workflow execution:', error);
      uiStore.showError(`Failed to start workflow: ${error}`);
    }
  }

  function addWorkflowStep(collection: Collection, stepType: string) {
    const newStep: WorkflowStep = {
      id: crypto.randomUUID(),
      name: `New ${stepType} Step`,
      description: '',
      enabled: true,
      continue_on_error: false,
      timeout_ms: 30000,
      depends_on: [],
      operation: createDefaultOperation(stepType),
      variable_extracts: [],
      assertions: [],
      execution_order: collection.workflow.length
    };

    collection.workflow.push(newStep);
    selectedStep = newStep;
    isEditing = true;
  }

  function createDefaultOperation(stepType: string): McpOperation {
    switch (stepType) {
      case 'tool':
        return {
          type: 'tool',
          server_alias: '',
          tool_name: '',
          parameters: {}
        };
      case 'resource':
        return {
          type: 'resource',
          server_alias: '',
          resource_uri: ''
        };
      case 'prompt':
        return {
          type: 'prompt',
          server_alias: '',
          prompt_name: '',
          parameters: {}
        };
      case 'sampling':
        return {
          type: 'sampling',
          server_alias: '',
          messages: [],
          max_tokens: 1000,
          temperature: 0.7,
          auto_approve: false
        };
      default:
        return {
          type: 'tool',
          server_alias: '',
          tool_name: '',
          parameters: {}
        };
    }
  }

  function getStepIcon(operation: McpOperation) {
    switch (operation.type) {
      case 'tool': return Zap;
      case 'resource': return Database;
      case 'prompt': return MessageSquare;
      case 'sampling': return Brain;
      case 'elicitation': return MessageSquare;
      case 'delay': return Clock;
      case 'conditional': return GitBranch;
      default: return FileText;
    }
  }

  function getStepStatusColor(status: string) {
    switch (status) {
      case 'completed': return 'text-green-600 bg-green-100';
      case 'running': return 'text-blue-600 bg-blue-100';
      case 'failed': return 'text-red-600 bg-red-100';
      case 'pending': return 'text-yellow-600 bg-yellow-100';
      case 'skipped': return 'text-gray-600 bg-gray-100';
      default: return 'text-gray-600 bg-gray-100';
    }
  }

  function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  }

  // Import/Export Functions
  let importData = $state('');
  let templates: any[] = $state([]);
  let selectedTemplate: any = $state(null);
  let templateName = $state('');
  let templateVariables = $state<Record<string, string>>({});

  async function exportCollection(collection: Collection, includeHistory = false) {
    try {
      const exportData = await invoke<string>('export_collection', {
        collectionId: collection.id,
        includeExecutionHistory: includeHistory
      });

      // Create downloadable file
      const blob = new Blob([exportData], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${collection.name.replace(/[^a-zA-Z0-9-_]/g, '_')}.mcpstudio.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      uiStore.showSuccess('Collection exported successfully');
    } catch (error) {
      console.error('Export failed:', error);
      uiStore.showError(`Export failed: ${error}`);
    }
  }

  async function importFromJSON() {
    if (!importData.trim()) {
      uiStore.showError('Please paste collection JSON data');
      return;
    }

    try {
      const collectionId = await invoke('import_collection', {
        jsonData: importData,
        overwriteExisting: false
      });

      await loadCollections();
      showImportDialog = false;
      importData = '';
      uiStore.showSuccess('Collection imported successfully');
    } catch (error) {
      console.error('Import failed:', error);
      uiStore.showError(`Import failed: ${error}`);
    }
  }

  async function loadTemplates() {
    try {
      templates = await invoke('get_collection_templates');
    } catch (error) {
      console.error('Failed to load templates:', error);
      uiStore.showError(`Failed to load templates: ${error}`);
    }
  }

  async function createFromTemplate() {
    if (!selectedTemplate || !templateName.trim()) {
      uiStore.showError('Please select a template and provide a name');
      return;
    }

    try {
      const collectionId = await invoke('create_collection_from_template', {
        templateId: selectedTemplate.template_id,
        collectionName: templateName,
        variableValues: templateVariables
      });

      await loadCollections();
      showTemplates = false;
      selectedTemplate = null;
      templateName = '';
      templateVariables = {};
      uiStore.showSuccess('Collection created from template');
    } catch (error) {
      console.error('Template creation failed:', error);
      uiStore.showError(`Template creation failed: ${error}`);
    }
  }

  // Load templates when templates dialog is opened
  $effect(() => {
    if (showTemplates && templates.length === 0) {
      loadTemplates();
    }
  });
</script>

<!-- Collections Manager Interface -->
<div class="h-full flex flex-col bg-gray-50 dark:bg-gray-900">
  <!-- Header -->
  <div class="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
    <div class="flex items-center space-x-4">
      <h1 class="text-xl font-semibold text-gray-900 dark:text-gray-100">Collections</h1>
      <span class="text-sm text-gray-500 dark:text-gray-400">
        {collections.length} collection{collections.length !== 1 ? 's' : ''}
      </span>
    </div>

    <div class="flex items-center space-x-2">
      <!-- View Mode Buttons -->
      <div class="flex items-center space-x-1 bg-gray-100 dark:bg-gray-700 rounded-lg p-1">
        <button
          onclick={() => viewMode = 'list'}
          class="px-3 py-1 text-sm rounded-md transition-colors
                 {viewMode === 'list' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-gray-100' : 'text-gray-600 dark:text-gray-400'}"
        >
          List
        </button>
        {#if selectedCollection}
          <button
            onclick={() => viewMode = 'editor'}
            class="px-3 py-1 text-sm rounded-md transition-colors
                   {viewMode === 'editor' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-gray-100' : 'text-gray-600 dark:text-gray-400'}"
          >
            Editor
          </button>
        {/if}
        {#if currentExecution}
          <button
            onclick={() => viewMode = 'execution'}
            class="px-3 py-1 text-sm rounded-md transition-colors
                   {viewMode === 'execution' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-gray-100' : 'text-gray-600 dark:text-gray-400'}"
          >
            Execution
          </button>
        {/if}
      </div>

      <!-- Action Buttons -->
      <button
        onclick={() => showImportDialog = true}
        class="btn-secondary text-sm"
        title="Import Collection"
      >
        <Upload size={14} />
      </button>

      <button
        onclick={() => showTemplates = !showTemplates}
        class="btn-secondary text-sm"
        title="Collection Templates"
      >
        <FolderOpen size={14} />
      </button>

      <button
        onclick={createNewCollection}
        class="btn-primary text-sm"
      >
        <Plus size={14} class="mr-1" />
        New Collection
      </button>
    </div>
  </div>

  <!-- Main Content -->
  <div class="flex-1 overflow-hidden">
    {#if viewMode === 'list'}
      <!-- Collections List View -->
      <div class="h-full p-4">
        {#if loading}
          <div class="flex items-center justify-center h-64">
            <div class="text-center">
              <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto mb-3"></div>
              <p class="text-gray-600 dark:text-gray-400">Loading collections...</p>
            </div>
          </div>
        {:else if collections.length === 0}
          <div class="flex items-center justify-center h-64">
            <div class="text-center">
              <FileText size={48} class="mx-auto text-gray-400 dark:text-gray-500 mb-4" />
              <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">No Collections Yet</h3>
              <p class="text-gray-600 dark:text-gray-400 mb-4">
                Create your first collection to start building cross-server workflows
              </p>
              <button onclick={createNewCollection} class="btn-primary">
                <Plus size={16} class="mr-2" />
                Create Collection
              </button>
            </div>
          </div>
        {:else}
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {#each collections as collection}
              <div class="card hover:shadow-lg transition-shadow cursor-pointer"
                   onclick={() => { selectedCollection = collection; viewMode = 'editor'; }}
                   onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (() => { selectedCollection = collection; viewMode = 'editor'; })()}
                   role="button"
                   tabindex="0">
                <!-- Collection Header -->
                <div class="flex items-start justify-between mb-3">
                  <div class="flex-1">
                    <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-1">
                      {collection.name}
                    </h3>
                    {#if collection.description}
                      <p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-2">
                        {collection.description}
                      </p>
                    {/if}
                  </div>
                  <div class="flex items-center space-x-1">
                    <button
                      onclick={(e) => { e.stopPropagation(); executeWorkflow(collection); }}
                      class="p-1 text-gray-400 hover:text-green-600 dark:hover:text-green-400"
                      title="Execute Workflow"
                      disabled={!collection.workflow.length}
                    >
                      <Play size={16} />
                    </button>
                    <button
                      onclick={(e) => { e.stopPropagation(); exportCollection(collection); }}
                      class="p-1 text-gray-400 hover:text-blue-600 dark:hover:text-blue-400"
                      title="Export Collection"
                    >
                      <Download size={16} />
                    </button>
                    <button
                      onclick={(e) => { e.stopPropagation(); deleteCollection(collection.id); }}
                      class="p-1 text-gray-400 hover:text-red-600 dark:hover:text-red-400"
                      title="Delete Collection"
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                </div>

                <!-- Collection Stats -->
                <div class="grid grid-cols-3 gap-3 mb-3">
                  <div class="text-center">
                    <div class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                      {collection.workflow.length}
                    </div>
                    <div class="text-xs text-gray-600 dark:text-gray-400">Steps</div>
                  </div>
                  <div class="text-center">
                    <div class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                      {Object.keys(collection.environment.servers).length}
                    </div>
                    <div class="text-xs text-gray-600 dark:text-gray-400">Servers</div>
                  </div>
                  <div class="text-center">
                    <div class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                      {collection.run_count}
                    </div>
                    <div class="text-xs text-gray-600 dark:text-gray-400">Runs</div>
                  </div>
                </div>

                <!-- Tags -->
                {#if collection.tags.length > 0}
                  <div class="flex flex-wrap gap-1 mb-3">
                    {#each collection.tags.slice(0, 3) as tag}
                      <span class="text-xs bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-200 px-2 py-1 rounded">
                        {tag}
                      </span>
                    {/each}
                    {#if collection.tags.length > 3}
                      <span class="text-xs text-gray-500 dark:text-gray-400">
                        +{collection.tags.length - 3} more
                      </span>
                    {/if}
                  </div>
                {/if}

                <!-- Last Run -->
                <div class="text-xs text-gray-500 dark:text-gray-400 flex items-center justify-between">
                  <span>
                    {collection.last_run
                      ? `Last run: ${new Date(collection.last_run).toLocaleDateString()}`
                      : 'Never run'
                    }
                  </span>
                  <span>v{collection.version}</span>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

    {:else if viewMode === 'editor' && selectedCollection}
      <!-- Collection Editor View -->
      <div class="h-full flex">
        <!-- Workflow Steps Panel -->
        <div class="w-1/3 border-r border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
          <div class="p-4 border-b border-gray-200 dark:border-gray-700">
            <div class="flex items-center justify-between mb-3">
              <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                {selectedCollection.name}
              </h3>
              <button
                onclick={() => saveCollection(selectedCollection!)}
                class="btn-primary text-sm"
                disabled={saving}
              >
                {#if saving}
                  <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-1"></div>
                {:else}
                  <Save size={14} class="mr-1" />
                {/if}
                Save
              </button>
            </div>

            <!-- Add Step Buttons -->
            <div class="grid grid-cols-2 gap-2">
              <button
                onclick={() => addWorkflowStep(selectedCollection!, 'tool')}
                class="btn-secondary text-sm justify-center"
              >
                <Zap size={12} class="mr-1" />
                Tool
              </button>
              <button
                onclick={() => addWorkflowStep(selectedCollection!, 'resource')}
                class="btn-secondary text-sm justify-center"
              >
                <Database size={12} class="mr-1" />
                Resource
              </button>
              <button
                onclick={() => addWorkflowStep(selectedCollection!, 'prompt')}
                class="btn-secondary text-sm justify-center"
              >
                <MessageSquare size={12} class="mr-1" />
                Prompt
              </button>
              <button
                onclick={() => addWorkflowStep(selectedCollection!, 'sampling')}
                class="btn-secondary text-sm justify-center"
              >
                <Brain size={12} class="mr-1" />
                Sampling
              </button>
            </div>
          </div>

          <!-- Workflow Steps List -->
          <div class="flex-1 overflow-y-auto p-4">
            {#if selectedCollection.workflow.length === 0}
              <div class="text-center py-8">
                <GitBranch size={32} class="mx-auto text-gray-400 dark:text-gray-500 mb-3" />
                <p class="text-sm text-gray-600 dark:text-gray-400">
                  No workflow steps yet.<br>
                  Add steps to build your workflow.
                </p>
              </div>
            {:else}
              <div class="space-y-2">
                {#each selectedCollection.workflow as step, index}
                  {@const StepIcon = getStepIcon(step.operation)}
                  <button
                    onclick={() => selectedStep = step}
                    class="w-full p-3 text-left rounded-lg border border-gray-200 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors
                           {selectedStep?.id === step.id ? 'ring-2 ring-blue-500 bg-blue-50 dark:bg-blue-900/20' : ''}"
                  >
                    <div class="flex items-center space-x-3">
                      <div class="flex items-center justify-center w-8 h-8 rounded-full bg-gray-100 dark:bg-gray-600">
                        <span class="text-sm font-medium text-gray-600 dark:text-gray-300">
                          {index + 1}
                        </span>
                      </div>
                      <div class="flex-1">
                        <div class="flex items-center space-x-2">
                          <StepIcon size={14} class="text-gray-500 dark:text-gray-400" />
                          <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                            {step.name}
                          </span>
                        </div>
                        <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">
                          {step.operation.type}
                          {#if step.operation.type === 'tool'}
                            • {step.operation.tool_name || 'Not configured'}
                          {:else if step.operation.type === 'resource'}
                            • {step.operation.resource_uri || 'Not configured'}
                          {:else if step.operation.type === 'prompt'}
                            • {step.operation.prompt_name || 'Not configured'}
                          {/if}
                        </p>
                      </div>
                      {#if !step.enabled}
                        <div class="w-2 h-2 rounded-full bg-gray-400" title="Disabled"></div>
                      {/if}
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </div>

        <!-- Step Editor Panel -->
        <div class="flex-1 bg-white dark:bg-gray-800">
          {#if selectedStep}
            <div class="h-full flex flex-col">
              <div class="p-4 border-b border-gray-200 dark:border-gray-700">
                <h4 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                  Step Configuration
                </h4>
              </div>
              <div class="flex-1 overflow-y-auto p-4">
                <!-- Basic Step Settings -->
                <div class="space-y-4">
                  <!-- Step Name -->
                  <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                      Step Name
                    </label>
                    <input
                      bind:value={selectedStep.name}
                      placeholder="Enter step name..."
                      class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>

                  <!-- Step Description -->
                  <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                      Description
                    </label>
                    <textarea
                      bind:value={selectedStep.description}
                      placeholder="Describe what this step does..."
                      rows="2"
                      class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    ></textarea>
                  </div>

                  <!-- Step Options -->
                  <div class="flex items-center space-x-6">
                    <label class="flex items-center space-x-2">
                      <input
                        type="checkbox"
                        bind:checked={selectedStep.enabled}
                        class="rounded border-gray-300 dark:border-gray-600 text-blue-600 focus:ring-blue-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">Enabled</span>
                    </label>
                    <label class="flex items-center space-x-2">
                      <input
                        type="checkbox"
                        bind:checked={selectedStep.continue_on_error}
                        class="rounded border-gray-300 dark:border-gray-600 text-blue-600 focus:ring-blue-500"
                      />
                      <span class="text-sm text-gray-700 dark:text-gray-300">Continue on Error</span>
                    </label>
                  </div>

                  <!-- Timeout -->
                  <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                      Timeout (ms)
                    </label>
                    <input
                      type="number"
                      bind:value={selectedStep.timeout_ms}
                      min="0"
                      step="1000"
                      class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>

                  <div class="border-t border-gray-200 dark:border-gray-700 pt-4">
                    <h5 class="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">
                      Operation Configuration
                    </h5>

                    <!-- NEW: Composed operation-specific components -->
                    {#if selectedStep.operation.type === 'tool'}
                      <ToolStepConfig operation={selectedStep.operation} {servers} />
                    {:else if selectedStep.operation.type === 'resource'}
                      <ResourceStepConfig operation={selectedStep.operation} {servers} />
                    {:else if selectedStep.operation.type === 'sampling'}
                      <SamplingStepConfig operation={selectedStep.operation} {servers} />
                    {:else if selectedStep.operation.type === 'prompt'}
                      <PromptStepConfig operation={selectedStep.operation} {servers} />
                    {:else if selectedStep.operation.type === 'elicitation'}
                      <!-- TODO: Create ElicitationStepConfig component -->
                      <div class="p-3 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded">
                        <p class="text-sm text-yellow-700 dark:text-yellow-300">
                          Elicitation operation configuration coming soon - will compose ElicitationFlow logic
                        </p>
                      </div>
                    {:else}
                      <div class="p-3 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded">
                        <p class="text-sm text-gray-600 dark:text-gray-400">
                          Operation type: {selectedStep.operation.type}
                        </p>
                      </div>
                    {/if}
                  </div>

                  <!-- NEW: Variable Extraction -->
                  <VariableExtractor bind:extracts={selectedStep.variable_extracts} />

                  <!-- NEW: Assertions -->
                  <AssertionEditor bind:assertions={selectedStep.assertions} />

                  <!-- Delete Step Button -->
                  <div class="border-t border-gray-200 dark:border-gray-700 pt-4 mt-6">
                    <button
                      onclick={() => {
                        if (confirm('Delete this step?')) {
                          selectedCollection!.workflow = selectedCollection!.workflow.filter(s => s.id !== selectedStep!.id);
                          selectedStep = null;
                        }
                      }}
                      class="w-full btn-secondary text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 justify-center"
                    >
                      <Trash2 size={14} class="mr-2" />
                      Delete Step
                    </button>
                  </div>
                </div>
              </div>
            </div>
          {:else}
            <div class="flex items-center justify-center h-full">
              <div class="text-center">
                <Settings size={48} class="mx-auto text-gray-400 dark:text-gray-500 mb-4" />
                <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">
                  Select a Step to Configure
                </h3>
                <p class="text-gray-600 dark:text-gray-400">
                  Choose a workflow step from the left panel to edit its configuration
                </p>
              </div>
            </div>
          {/if}
        </div>
      </div>

    {:else if viewMode === 'execution' && currentExecution}
      <!-- Real-Time Execution Monitor -->
      <ExecutionMonitor executionId={currentExecution.id} />
    {/if}
  </div>
</div>

<!-- Import Dialog -->
{#if showImportDialog}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-800 rounded-lg p-6 w-[600px] max-h-[80vh] flex flex-col">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
        Import Collection
      </h3>

      <div class="flex-1 overflow-hidden">
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          Collection JSON Data
        </label>
        <textarea
          bind:value={importData}
          placeholder="Paste your collection JSON here..."
          class="w-full h-[300px] p-3 border border-gray-300 dark:border-gray-600 rounded-lg
                 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100
                 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400
                 font-mono text-sm resize-none"
        ></textarea>

        <div class="mt-3 text-xs text-gray-500 dark:text-gray-400">
          <p>• JSON should contain MCP Studio collection format</p>
          <p>• Collections will be imported with new IDs to avoid conflicts</p>
          <p>• Server configurations may need to be adjusted after import</p>
        </div>
      </div>

      <div class="flex justify-end space-x-3 mt-6">
        <button
          onclick={() => { showImportDialog = false; importData = ''; }}
          class="btn-secondary"
        >
          Cancel
        </button>
        <button
          onclick={importFromJSON}
          class="btn-primary"
          disabled={!importData.trim()}
        >
          <Upload size={14} class="mr-1" />
          Import Collection
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Templates Dialog -->
{#if showTemplates}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-800 rounded-lg p-6 w-[700px] max-h-[80vh] flex flex-col">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
        Collection Templates
      </h3>

      <div class="flex-1 overflow-hidden flex flex-col">
        {#if templates.length === 0}
          <div class="flex items-center justify-center h-32">
            <div class="text-center">
              <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600 mx-auto mb-2"></div>
              <p class="text-sm text-gray-600 dark:text-gray-400">Loading templates...</p>
            </div>
          </div>
        {:else}
          <div class="grid grid-cols-1 gap-3 mb-4 max-h-[300px] overflow-y-auto">
            {#each templates as template}
              <button
                onclick={() => selectedTemplate = template}
                class="text-left p-4 rounded-lg border border-gray-200 dark:border-gray-600
                       hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors
                       {selectedTemplate?.template_id === template.template_id ? 'ring-2 ring-blue-500 bg-blue-50 dark:bg-blue-900/20' : ''}"
              >
                <div class="flex items-start space-x-3">
                  <div class="flex-shrink-0">
                    {#if template.template_id === 'filesystem-explorer'}
                      <FolderOpen size={20} class="text-blue-600 dark:text-blue-400" />
                    {:else if template.template_id === 'api-testing-suite'}
                      <Zap size={20} class="text-green-600 dark:text-green-400" />
                    {:else if template.template_id === 'database-query-chain'}
                      <Database size={20} class="text-purple-600 dark:text-purple-400" />
                    {:else}
                      <FileText size={20} class="text-gray-600 dark:text-gray-400" />
                    {/if}
                  </div>
                  <div class="flex-1">
                    <h4 class="font-medium text-gray-900 dark:text-gray-100">
                      {template.name}
                    </h4>
                    <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
                      {template.description}
                    </p>
                    <div class="flex flex-wrap gap-1 mt-2">
                      {#each template.tags as tag}
                        <span class="text-xs bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 px-2 py-1 rounded">
                          {tag}
                        </span>
                      {/each}
                    </div>
                  </div>
                </div>
              </button>
            {/each}
          </div>

          {#if selectedTemplate}
            <div class="border-t border-gray-200 dark:border-gray-700 pt-4">
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Collection Name
              </label>
              <input
                bind:value={templateName}
                placeholder="Enter collection name..."
                class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg
                       bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100
                       focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400"
              />

              {#if selectedTemplate.variables && Object.keys(selectedTemplate.variables).length > 0}
                <div class="mt-4">
                  <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    Template Variables
                  </label>
                  <div class="space-y-2 max-h-[120px] overflow-y-auto">
                    {#each Object.entries(selectedTemplate.variables) as [key, defaultValue]}
                      {@const value = templateVariables[key] ?? String(defaultValue ?? '')}
                      <div>
                        <label class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
                          {key}
                        </label>
                        <input
                          value={value}
                          oninput={(e) => templateVariables[key] = (e.target as HTMLInputElement).value}
                          placeholder={String(defaultValue ?? '')}
                          class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded text-sm
                                 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100
                                 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:focus:ring-blue-400"
                        />
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        {/if}
      </div>

      <div class="flex justify-end space-x-3 mt-6">
        <button
          onclick={() => {
            showTemplates = false;
            selectedTemplate = null;
            templateName = '';
            templateVariables = {};
          }}
          class="btn-secondary"
        >
          Cancel
        </button>
        {#if selectedTemplate}
          <button
            onclick={createFromTemplate}
            class="btn-primary"
            disabled={!templateName.trim()}
          >
            <Plus size={14} class="mr-1" />
            Create Collection
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}