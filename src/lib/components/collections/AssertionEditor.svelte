<!--
  AssertionEditor - Configure assertions for step result validation

  Allows users to add validations like:
  - Response contains specific text
  - Response equals expected value
  - JSONPath matches value
  - Timing requirements

  Results shown in ExecutionMonitor during collection runs.
-->
<script lang="ts">
  import { Plus, Trash2, Info, CheckCircle, AlertCircle } from 'lucide-svelte';
  import type { Assertion, AssertionType } from '$lib/types/collections';

  interface Props {
    assertions: Assertion[];
  }

  let { assertions = $bindable([]) }: Props = $props();

  function addAssertion() {
    assertions.push({
      id: crypto.randomUUID(),
      name: 'New assertion',
      type: 'response_contains',
      condition: {
        operator: 'contains',
        expected_value: ''
      },
      severity: 'error',
      continue_on_failure: false
    });
  }

  function removeAssertion(index: number) {
    assertions.splice(index, 1);
  }

  // Get operator options based on assertion type
  function getOperatorOptions(type: AssertionType): Array<{value: string, label: string}> {
    switch (type) {
      case 'response_contains':
      case 'response_equals':
        return [
          { value: 'contains', label: 'Contains' },
          { value: 'not_contains', label: 'Not Contains' },
          { value: 'equals', label: 'Equals' },
          { value: 'not_equals', label: 'Not Equals' }
        ];
      case 'timing':
        return [
          { value: 'less_than', label: 'Less Than (ms)' },
          { value: 'greater_than', label: 'Greater Than (ms)' }
        ];
      case 'response_json_path':
        return [
          { value: 'equals', label: 'Equals' },
          { value: 'not_equals', label: 'Not Equals' },
          { value: 'contains', label: 'Contains' },
          { value: 'greater_than', label: 'Greater Than' },
          { value: 'less_than', label: 'Less Than' }
        ];
      default:
        return [
          { value: 'equals', label: 'Equals' },
          { value: 'not_equals', label: 'Not Equals' }
        ];
    }
  }
</script>

<div class="border-t border-gray-200 dark:border-gray-700 pt-4 mt-4">
  <!-- Header -->
  <div class="flex items-center justify-between mb-3">
    <div class="flex items-center gap-2">
      <h4 class="text-sm font-semibold text-gray-900 dark:text-gray-100">
        Assertions
      </h4>
      <button
        type="button"
        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
        title="Validate this step's response meets expectations"
      >
        <Info size={14} />
      </button>
    </div>
    <button
      type="button"
      onclick={addAssertion}
      class="flex items-center gap-1 px-2 py-1 text-sm bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 hover:bg-green-100 dark:hover:bg-green-900/30 rounded border border-green-200 dark:border-green-800"
    >
      <Plus size={14} />
      <span>Add</span>
    </button>
  </div>

  <!-- Info message when no assertions -->
  {#if assertions.length === 0}
    <div class="p-3 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
      <p class="text-sm text-gray-600 dark:text-gray-400">
        Add assertions to validate this step's response.
        <br />
        <span class="text-xs mt-1 inline-block">
          Example: Assert response contains "success" or timing is less than 1000ms
        </span>
      </p>
    </div>
  {/if}

  <!-- Assertion list -->
  {#each assertions as assertion, i}
    <div class="border border-gray-200 dark:border-gray-700 rounded-lg p-3 mb-3 bg-white dark:bg-gray-800">
      <!-- Assertion header -->
      <div class="flex items-start justify-between mb-3">
        <input
          type="text"
          bind:value={assertion.name}
          placeholder="Assertion name"
          class="flex-1 px-2 py-1 text-sm font-semibold border border-transparent hover:border-gray-300 dark:hover:border-gray-600 rounded bg-transparent text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
        />
        <button
          type="button"
          onclick={() => removeAssertion(i)}
          class="ml-2 p-1 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded"
          title="Remove assertion"
        >
          <Trash2 size={14} />
        </button>
      </div>

      <!-- Assertion configuration -->
      <div class="grid grid-cols-3 gap-2 mb-3">
        <!-- Type selector -->
        <div>
          <label for="assertion-type-{i}" class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
            Type
          </label>
          <select
            id="assertion-type-{i}"
            bind:value={assertion.type}
            class="w-full p-2 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="response_contains">Contains</option>
            <option value="response_equals">Equals</option>
            <option value="response_json_path">JSON Path</option>
            <option value="timing">Timing</option>
            <option value="response_status">Status</option>
            <option value="variable_value">Variable Value</option>
          </select>
        </div>

        <!-- Operator selector -->
        <div>
          <label for="assertion-operator-{i}" class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
            Operator
          </label>
          <select
            id="assertion-operator-{i}"
            bind:value={assertion.condition.operator}
            class="w-full p-2 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            {#each getOperatorOptions(assertion.type) as option}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        </div>

        <!-- Severity selector -->
        <div>
          <label for="assertion-severity-{i}" class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
            Severity
          </label>
          <select
            id="assertion-severity-{i}"
            bind:value={assertion.severity}
            class="w-full p-2 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="error">Error</option>
            <option value="warning">Warning</option>
            <option value="info">Info</option>
          </select>
        </div>
      </div>

      <!-- Expected value input -->
      <div class="mb-3">
        <label for="assertion-value-{i}" class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
          Expected Value
          {#if assertion.type === 'response_json_path'}
            <span class="text-gray-500 ml-1">(JSONPath: e.g., $.result.status)</span>
          {:else if assertion.type === 'timing'}
            <span class="text-gray-500 ml-1">(milliseconds)</span>
          {/if}
        </label>
        <input
          id="assertion-value-{i}"
          type="text"
          bind:value={assertion.condition.expected_value}
          placeholder={assertion.type === 'timing' ? '1000' : 'Expected value...'}
          class="w-full p-2 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      <!-- JSONPath input (for json_path assertions) -->
      {#if assertion.type === 'response_json_path'}
        <div class="mb-3">
          <label for="assertion-path-{i}" class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
            JSONPath
          </label>
          <input
            id="assertion-path-{i}"
            type="text"
            bind:value={assertion.condition.actual_path}
            placeholder="$.result.status"
            class="w-full p-2 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 font-mono focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>
      {/if}

      <!-- Continue on failure checkbox -->
      <label class="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          bind:checked={assertion.continue_on_failure}
          class="rounded border-gray-300 dark:border-gray-600 text-blue-600 focus:ring-blue-500"
        />
        <span class="text-gray-700 dark:text-gray-300">Continue collection execution if this assertion fails</span>
      </label>

      <!-- Description input -->
      {#if assertion.description !== undefined}
        <div class="mt-2">
          <label for="assertion-desc-{i}" class="block text-xs text-gray-600 dark:text-gray-400 mb-1">
            Description (optional)
          </label>
          <textarea
            id="assertion-desc-{i}"
            bind:value={assertion.description}
            placeholder="Explain what this assertion validates..."
            rows="2"
            class="w-full p-2 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          ></textarea>
        </div>
      {/if}
    </div>
  {/each}

  <!-- Summary -->
  {#if assertions.length > 0}
    <div class="mt-3 p-2 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded text-xs flex items-center gap-2">
      <CheckCircle size={14} class="text-green-600 dark:text-green-400" />
      <span class="text-green-700 dark:text-green-300">
        {assertions.length} assertion{assertions.length === 1 ? '' : 's'} configured
        ({assertions.filter(a => a.severity === 'error').length} error,
        {assertions.filter(a => a.severity === 'warning').length} warning,
        {assertions.filter(a => a.severity === 'info').length} info)
      </span>
    </div>
  {/if}
</div>
