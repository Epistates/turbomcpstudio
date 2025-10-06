<!--
  ElicitationStepConfig - Component for configuring elicitation operations in Collections

  Elicitation allows workflows to request user input dynamically.
  Unlike other operations, this doesn't query a server - it configures the form structure.
-->
<script lang="ts">
  import type { ServerInfo } from '$lib/stores/serverStore';
  import type { ElicitationOperation, PrimitiveSchemaDefinition } from '$lib/types/collections';
  import { AlertCircle, Plus, Trash2 } from 'lucide-svelte';

  interface Props {
    operation: ElicitationOperation;
    servers: ServerInfo[];
    availableVariables?: string[];
  }

  let { operation, servers, availableVariables = [] }: Props = $props();

  // Property being edited
  let editingProperty = $state<string | null>(null);
  let newPropertyName = $state('');
  let newPropertyType = $state<'string' | 'number' | 'boolean'>('string');

  // Add a new property to the schema
  function addProperty() {
    if (!newPropertyName.trim()) return;

    let propertyDef: PrimitiveSchemaDefinition;

    if (newPropertyType === 'string') {
      propertyDef = { type: 'string', description: '' };
    } else if (newPropertyType === 'number') {
      propertyDef = { type: 'number', description: '' };
    } else {
      propertyDef = { type: 'boolean', description: '' };
    }

    if (!operation.requested_schema.properties) {
      operation.requested_schema.properties = {};
    }

    operation.requested_schema.properties[newPropertyName] = propertyDef;
    newPropertyName = '';
    newPropertyType = 'string';
  }

  // Remove a property from the schema
  function removeProperty(name: string) {
    if (operation.requested_schema.properties) {
      delete operation.requested_schema.properties[name];
      operation.requested_schema.properties = { ...operation.requested_schema.properties };
    }

    // Remove from required list if present
    if (operation.requested_schema.required) {
      operation.requested_schema.required = operation.requested_schema.required.filter(r => r !== name);
    }
  }

  // Toggle required status
  function toggleRequired(name: string) {
    if (!operation.requested_schema.required) {
      operation.requested_schema.required = [];
    }

    const index = operation.requested_schema.required.indexOf(name);
    if (index > -1) {
      operation.requested_schema.required.splice(index, 1);
    } else {
      operation.requested_schema.required.push(name);
    }
    operation.requested_schema.required = [...operation.requested_schema.required];
  }

  // Check if property is required
  function isRequired(name: string): boolean {
    return operation.requested_schema.required?.includes(name) ?? false;
  }

  // Get property list
  const properties = $derived(
    Object.entries(operation.requested_schema.properties || {})
  );
</script>

<div class="space-y-4">
  <!-- Server Selection -->
  <div>
    <label for="elicitation-server" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
      Server *
    </label>
    <select
      id="elicitation-server"
      bind:value={operation.server_alias}
      class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
      required
    >
      <option value="">Select a server...</option>
      {#each servers as server}
        <option value={server.id}>{server.config.name}</option>
      {/each}
    </select>
  </div>

  <!-- Message -->
  <div>
    <label for="elicitation-message" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
      Message *
    </label>
    <textarea
      id="elicitation-message"
      bind:value={operation.message}
      placeholder="What information would you like from the user?"
      class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white h-20 resize-none"
      required
    ></textarea>
    <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
      This message will be shown to the user when requesting input
    </p>
  </div>

  <!-- Schema Builder -->
  <div>
    <div class="flex items-center justify-between mb-2">
      <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
        Form Fields
      </label>
      <button
        type="button"
        onclick={() => editingProperty = 'new'}
        class="text-sm text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 flex items-center gap-1"
      >
        <Plus size={14} />
        Add Field
      </button>
    </div>

    {#if properties.length === 0}
      <div class="border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-lg p-4 text-center">
        <AlertCircle size={24} class="mx-auto text-gray-400 mb-2" />
        <p class="text-sm text-gray-600 dark:text-gray-400">
          No form fields defined. Click "Add Field" to create one.
        </p>
      </div>
    {:else}
      <div class="space-y-2">
        {#each properties as [name, definition]}
          <div class="border border-gray-200 dark:border-gray-700 rounded-lg p-3 bg-gray-50 dark:bg-gray-800">
            <div class="flex items-center justify-between mb-2">
              <div class="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={isRequired(name)}
                  onchange={() => toggleRequired(name)}
                  class="rounded text-blue-600 focus:ring-blue-500"
                  id="required-{name}"
                />
                <label for="required-{name}" class="text-sm font-medium text-gray-900 dark:text-white">
                  {name}
                </label>
                <span class="text-xs px-2 py-0.5 rounded bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300">
                  {definition.type}
                </span>
              </div>
              <button
                type="button"
                onclick={() => removeProperty(name)}
                class="text-red-600 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
                title="Remove field"
              >
                <Trash2 size={14} />
              </button>
            </div>

            <input
              type="text"
              bind:value={definition.description}
              placeholder="Field description (optional)"
              class="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-900 text-gray-900 dark:text-white"
            />
          </div>
        {/each}
      </div>
    {/if}

    <!-- Add Field Form -->
    {#if editingProperty === 'new'}
      <div class="mt-3 border border-blue-200 dark:border-blue-800 rounded-lg p-3 bg-blue-50 dark:bg-blue-900/20">
        <div class="space-y-2">
          <div>
            <label for="new-field-name" class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
              Field Name
            </label>
            <input
              id="new-field-name"
              type="text"
              bind:value={newPropertyName}
              placeholder="field_name"
              class="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
            />
          </div>

          <div>
            <label for="new-field-type" class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
              Field Type
            </label>
            <select
              id="new-field-type"
              bind:value={newPropertyType}
              class="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
            >
              <option value="string">String</option>
              <option value="number">Number</option>
              <option value="boolean">Boolean</option>
            </select>
          </div>

          <div class="flex gap-2">
            <button
              type="button"
              onclick={() => { addProperty(); editingProperty = null; }}
              disabled={!newPropertyName.trim()}
              class="flex-1 px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Add
            </button>
            <button
              type="button"
              onclick={() => { editingProperty = null; newPropertyName = ''; }}
              class="flex-1 px-3 py-1 text-sm bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-300 dark:hover:bg-gray-600"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    {/if}
  </div>

  <!-- Auto-approve -->
  <div class="flex items-center gap-2">
    <input
      id="elicitation-auto-approve"
      type="checkbox"
      bind:checked={operation.auto_approve}
      class="rounded text-blue-600 focus:ring-blue-500"
    />
    <label for="elicitation-auto-approve" class="text-sm text-gray-700 dark:text-gray-300">
      Auto-approve for automated testing
    </label>
  </div>
</div>
