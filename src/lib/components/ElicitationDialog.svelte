<script lang="ts">
  import Button from './ui/Button.svelte';
  import SchemaValidator from './ui/SchemaValidator.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { JsonSchema } from '$lib/utils/schemaValidation';
  import { validateElicitationSchema } from '$lib/utils/elicitationSchemaValidator';
  import { uiStore } from '$lib/stores/uiStore';
  import { ExternalLink } from 'lucide-svelte';

  interface ElicitationRequest {
    id: string;
    protocolMessageId?: string;  // For Protocol Inspector correlation
    serverId: string;
    serverName?: string;
    message: string;
    requestedSchema: JsonSchema;
  }

  // Component props using Svelte 5 runes mode
  const {
    visible = false,
    request = null,
    onResponse = () => {},
    onClose = () => {}
  }: {
    visible?: boolean;
    request?: ElicitationRequest | null;
    onResponse?: (data: any) => void;
    onClose?: () => void;
  } = $props();

  // Local state using Svelte 5 runes
  let formData = $state<Record<string, any>>({});
  let validationErrors = $state<Record<string, string>>({});
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let showSchemaValidation = $state(false);

  // Validate schema when request changes
  const schemaValidationResult = $derived(
    request?.requestedSchema ? validateElicitationSchema(request.requestedSchema) : null
  );

  // Reset form data when request changes
  $effect(() => {
    if (request?.requestedSchema) {
      const schema = request.requestedSchema;
      const initialData: Record<string, any> = {};
      
      // Initialize form data with defaults from schema
      if (schema.properties) {
        Object.entries(schema.properties).forEach(([key, prop]) => {
          if (prop.default !== undefined) {
            initialData[key] = prop.default;
          } else {
            switch (prop.type) {
              case 'string':
                initialData[key] = '';
                break;
              case 'number':
              case 'integer':
                initialData[key] = null;
                break;
              case 'boolean':
                initialData[key] = false;
                break;
              default:
                initialData[key] = null;
            }
          }
        });
      }
      
      formData = initialData;
      validationErrors = {};
      error = null;
    }
  });

  function validateForm() {
    if (!request?.requestedSchema) return true;

    const schema = request.requestedSchema;
    const errors: Record<string, string> = {};
    let isValid = true;

    // Check required fields
    if (schema.required) {
      schema.required.forEach(field => {
        if (!formData[field] || 
            (typeof formData[field] === 'string' && formData[field].trim() === '')) {
          errors[field] = 'This field is required';
          isValid = false;
        }
      });
    }

    // Validate individual fields
    if (schema.properties) {
      Object.entries(schema.properties).forEach(([key, prop]) => {
        const value = formData[key];
        
        if (value !== null && value !== undefined && value !== '') {
          // String validation
          if (prop.type === 'string') {
            if (prop.minLength && value.length < prop.minLength) {
              errors[key] = `Must be at least ${prop.minLength} characters`;
              isValid = false;
            }
            if (prop.maxLength && value.length > prop.maxLength) {
              errors[key] = `Must not exceed ${prop.maxLength} characters`;
              isValid = false;
            }
            if (prop.format) {
              let formatValid = true;
              switch (prop.format) {
                case 'email':
                  formatValid = /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
                  if (!formatValid) errors[key] = 'Must be a valid email address';
                  break;
                case 'uri':
                  try {
                    new URL(value);
                  } catch {
                    formatValid = false;
                    errors[key] = 'Must be a valid URL';
                  }
                  break;
                case 'date':
                  formatValid = /^\d{4}-\d{2}-\d{2}$/.test(value);
                  if (!formatValid) errors[key] = 'Must be a valid date (YYYY-MM-DD)';
                  break;
                case 'date-time':
                  formatValid = !isNaN(Date.parse(value));
                  if (!formatValid) errors[key] = 'Must be a valid date-time';
                  break;
              }
              if (!formatValid) isValid = false;
            }
          }
          
          // Number validation
          if (prop.type === 'number' || prop.type === 'integer') {
            const numValue = parseFloat(value);
            if (isNaN(numValue)) {
              errors[key] = 'Must be a valid number';
              isValid = false;
            } else {
              if (prop.minimum !== undefined && numValue < prop.minimum) {
                errors[key] = `Must be at least ${prop.minimum}`;
                isValid = false;
              }
              if (prop.maximum !== undefined && numValue > prop.maximum) {
                errors[key] = `Must not exceed ${prop.maximum}`;
                isValid = false;
              }
              if (prop.type === 'integer' && !Number.isInteger(numValue)) {
                errors[key] = 'Must be a whole number';
                isValid = false;
              }
            }
          }
        }
      });
    }

    validationErrors = errors;
    return isValid;
  }

  async function handleSubmit() {
    if (!validateForm()) {
      return;
    }

    submitting = true;
    error = null;

    try {
      await onResponse({
        action: 'accept',
        content: formData
      });
      onClose();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to submit response';
    } finally {
      submitting = false;
    }
  }

  async function handleDecline() {
    submitting = true;
    error = null;

    try {
      await onResponse({
        action: 'decline'
      });
      onClose();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to decline request';
    } finally {
      submitting = false;
    }
  }

  async function handleCancel() {
    submitting = true;
    error = null;

    try {
      await onResponse({
        action: 'cancel'
      });
      onClose();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to cancel request';
    } finally {
      submitting = false;
    }
  }

  function renderFormField(key: string, prop: any) {
    const hasError = validationErrors[key];
    const errorClass = hasError ? 'border-red-500 dark:border-red-400' : 'border-gray-300 dark:border-gray-600';
    const inputClass = `w-full px-3 py-2 border ${errorClass} rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-blue-500`;

    return {
      key,
      prop,
      hasError,
      errorClass,
      inputClass
    };
  }
</script>

{#if visible && request}
<div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-md w-full max-h-[90vh] overflow-hidden">
    
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded bg-blue-100 dark:bg-blue-900 flex items-center justify-center">
          <svg class="w-4 h-4 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
          </svg>
        </div>
        <div>
          <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Information Request</h2>
          <p class="text-sm text-gray-600 dark:text-gray-400">
            {request.serverName || 'Server'} is requesting information
          </p>
        </div>
      </div>
      <div class="flex items-center gap-3">
        {#if request.protocolMessageId}
          <button
            onclick={() => {
              uiStore.jumpToProtocolInspector(request.protocolMessageId);
              handleCancel();
            }}
            class="px-3 py-1.5 text-sm font-medium text-blue-700 dark:text-blue-300 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-lg hover:bg-blue-100 dark:hover:bg-blue-900/50 transition-colors flex items-center gap-2"
            title="View in Protocol Inspector"
          >
            <ExternalLink size={16} />
            <span>View Protocol</span>
          </button>
        {/if}
        <Button variant="ghost" onclick={handleCancel}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
          </svg>
        </Button>
      </div>
    </div>

    <div class="overflow-auto max-h-[calc(90vh-120px)]">
      <div class="p-6">

        <!-- Schema Validation (Collapsible) -->
        {#if schemaValidationResult && (!schemaValidationResult.valid || schemaValidationResult.warnings.length > 0)}
          <div class="mb-4">
            <button
              onclick={() => showSchemaValidation = !showSchemaValidation}
              class="text-xs text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors"
            >
              {showSchemaValidation ? '▼' : '▶'} Schema Validation
              {#if !schemaValidationResult.valid}
                <span class="text-red-600 dark:text-red-400">(has errors)</span>
              {:else if schemaValidationResult.warnings.length > 0}
                <span class="text-yellow-600 dark:text-yellow-400">(has warnings)</span>
              {/if}
            </button>

            {#if showSchemaValidation}
              <div class="mt-2">
                <SchemaValidator result={schemaValidationResult} compact />
              </div>
            {/if}
          </div>
        {/if}

        <!-- Request Message -->
        <div class="mb-6">
          <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
            <p class="text-blue-800 dark:text-blue-200 text-sm">
              {request.message || 'Please provide the requested information'}
            </p>
          </div>
        </div>

        <!-- Form Fields -->
        {#if request.requestedSchema?.properties}
        <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="space-y-4">
          {#each Object.entries(request.requestedSchema.properties) as [key, prop]}
            {@const field = renderFormField(key, prop)}
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1" for={key}>
                {prop.title || key}
                {#if request.requestedSchema.required?.includes(key)}
                  <span class="text-red-500 ml-1">*</span>
                {/if}
              </label>
              
              {#if prop.description}
                <p class="text-xs text-gray-500 dark:text-gray-400 mb-2">{prop.description}</p>
              {/if}

              <!-- String Input -->
              {#if prop.type === 'string'}
                {#if prop.enum}
                  <!-- Enum Select -->
                  <select bind:value={formData[key]} id={key} class={field.inputClass}>
                    <option value="">Select an option...</option>
                    {#each prop.enum as option, index}
                      <option value={option}>
                        {prop.enumNames?.[index] || option}
                      </option>
                    {/each}
                  </select>
                {:else if prop.format === 'date'}
                  <!-- Date Input -->
                  <input
                    type="date"
                    bind:value={formData[key]}
                    id={key}
                    class={field.inputClass}
                    min={prop.minimum}
                    max={prop.maximum}
                  >
                {:else if prop.format === 'date-time'}
                  <!-- DateTime Input -->
                  <input
                    type="datetime-local"
                    bind:value={formData[key]}
                    id={key}
                    class={field.inputClass}
                  >
                {:else if prop.format === 'email'}
                  <!-- Email Input -->
                  <input
                    type="email"
                    bind:value={formData[key]}
                    id={key}
                    class={field.inputClass}
                    placeholder={prop.description || 'Enter email address'}
                  >
                {:else if prop.format === 'uri'}
                  <!-- URL Input -->
                  <input
                    type="url"
                    bind:value={formData[key]}
                    id={key}
                    class={field.inputClass}
                    placeholder={prop.description || 'Enter URL'}
                  >
                {:else}
                  <!-- Text Input -->
                  <input
                    type="text"
                    bind:value={formData[key]}
                    id={key}
                    class={field.inputClass}
                    placeholder={prop.description || ''}
                    minlength={prop.minLength}
                    maxlength={prop.maxLength}
                  >
                {/if}
              {:else if prop.type === 'number' || prop.type === 'integer'}
                <!-- Number Input -->
                <input
                  type="number"
                  bind:value={formData[key]}
                  id={key}
                  class={field.inputClass}
                  min={prop.minimum}
                  max={prop.maximum}
                  step={prop.type === 'integer' ? '1' : 'any'}
                  placeholder={prop.description || ''}
                >
              {:else if prop.type === 'boolean'}
                <!-- Boolean Checkbox -->
                <div class="flex items-center">
                  <input
                    type="checkbox"
                    bind:checked={formData[key]}
                    id={key}
                    class="h-4 w-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                  >
                  <label for={key} class="ml-2 text-sm text-gray-700 dark:text-gray-300">
                    {prop.description || 'Enable this option'}
                  </label>
                </div>
              {/if}

              <!-- Validation Error -->
              {#if field.hasError}
                <p class="text-red-500 dark:text-red-400 text-xs mt-1">{validationErrors[key]}</p>
              {/if}
            </div>
          {/each}
        </form>
        {/if}

        <!-- Error Display -->
        {#if error}
          <div class="mt-4 border border-red-200 dark:border-red-800 rounded-lg p-4 bg-red-50 dark:bg-red-900/20">
            <div class="flex items-center gap-2 mb-2">
              <svg class="w-5 h-5 text-red-600 dark:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
              </svg>
              <span class="font-medium text-red-800 dark:text-red-200">Error</span>
            </div>
            <p class="text-red-700 dark:text-red-300 text-sm">{error}</p>
          </div>
        {/if}
      </div>
    </div>

    <!-- Footer -->
    <div class="flex items-center justify-between p-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
      <div class="flex gap-2">
        <Button variant="outline" onclick={handleDecline} disabled={submitting}>
          Decline
        </Button>
        <Button variant="ghost" onclick={handleCancel} disabled={submitting}>
          Cancel
        </Button>
      </div>

      <Button
        variant="primary"
        onclick={handleSubmit}
        disabled={submitting}
      >
        {#if submitting}
          <svg class="animate-spin -ml-1 mr-3 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          Submitting...
        {:else}
          Submit
        {/if}
      </Button>
    </div>
  </div>
</div>
{/if}