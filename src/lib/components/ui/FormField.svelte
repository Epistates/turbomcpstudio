<script lang="ts">
  /**
   * Accessible Form Field Component
   *
   * Provides proper label-control association, error states, and help text
   * Follows WCAG 2.1 Level AA guidelines
   */

  import { createEventDispatcher } from 'svelte';

  interface Props {
    label: string;
    id?: string;
    type?: 'text' | 'email' | 'password' | 'number' | 'url' | 'tel' | 'search' | 'textarea' | 'select';
    value?: string | number;
    placeholder?: string;
    required?: boolean;
    disabled?: boolean;
    readonly?: boolean;
    error?: string;
    helpText?: string;
    options?: Array<{value: string | number, label: string}>; // For select
    rows?: number; // For textarea
    class?: string;
    inputClass?: string;
  }

  let {
    label,
    id = `field-${Math.random().toString(36).substr(2, 9)}`,
    type = 'text',
    value = $bindable(''),
    placeholder,
    required = false,
    disabled = false,
    readonly = false,
    error,
    helpText,
    options,
    rows = 3,
    class: className = '',
    inputClass = '',
  }: Props = $props();

  const dispatch = createEventDispatcher();

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;
    value = target.value;
    dispatch('input', value);
  }

  function handleChange(e: Event) {
    const target = e.target as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;
    value = target.value;
    dispatch('change', value);
  }

  const baseInputClass = 'form-input';
  const errorClass = error ? 'border-red-500 focus:border-red-500 focus:ring-red-500' : '';
  const fullInputClass = `${baseInputClass} ${inputClass} ${errorClass}`.trim();
</script>

<div class="form-group {className}">
  <label for={id} class="form-label">
    {label}
    {#if required}
      <span class="text-red-500" aria-label="required">*</span>
    {/if}
  </label>

  {#if type === 'textarea'}
    <textarea
      {id}
      bind:value
      {placeholder}
      {required}
      {disabled}
      {readonly}
      {rows}
      class={fullInputClass}
      aria-describedby={helpText || error ? `${id}-help` : undefined}
      aria-invalid={error ? 'true' : undefined}
      oninput={handleInput}
      onchange={handleChange}
    />
  {:else if type === 'select' && options}
    <select
      {id}
      bind:value
      {required}
      {disabled}
      class={fullInputClass}
      aria-describedby={helpText || error ? `${id}-help` : undefined}
      aria-invalid={error ? 'true' : undefined}
      onchange={handleChange}
    >
      {#if placeholder}
        <option value="" disabled selected>{placeholder}</option>
      {/if}
      {#each options as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  {:else}
    <input
      {id}
      {type}
      bind:value
      {placeholder}
      {required}
      {disabled}
      {readonly}
      class={fullInputClass}
      aria-describedby={helpText || error ? `${id}-help` : undefined}
      aria-invalid={error ? 'true' : undefined}
      oninput={handleInput}
      onchange={handleChange}
    />
  {/if}

  {#if error || helpText}
    <p id="{id}-help" class="text-xs mt-1 {error ? 'text-red-500' : 'text-gray-500'}">
      {error || helpText}
    </p>
  {/if}
</div>

<style>
  .form-group {
    margin-bottom: 1rem;
  }

  .form-label {
    display: block;
    font-size: 0.875rem;
    font-weight: 500;
    margin-bottom: 0.375rem;
    color: var(--mcp-text-primary);
  }

  .form-input {
    width: 100%;
    padding: 0.5rem 0.75rem;
    font-size: 0.875rem;
    line-height: 1.5;
    color: var(--mcp-text-primary);
    background-color: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    transition: border-color 0.15s ease-in-out, box-shadow 0.15s ease-in-out;
  }

  .form-input:focus {
    outline: none;
    border-color: var(--mcp-primary);
    box-shadow: 0 0 0 3px rgba(var(--mcp-primary-rgb), 0.1);
  }

  .form-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .form-input::placeholder {
    color: var(--mcp-text-tertiary);
  }

  select.form-input {
    cursor: pointer;
  }

  textarea.form-input {
    resize: vertical;
    min-height: 80px;
  }
</style>
