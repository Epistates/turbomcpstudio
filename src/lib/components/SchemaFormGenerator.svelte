<script lang="ts">
  import { Folder, AlertCircle, Info, Eye, EyeOff } from 'lucide-svelte';
  import { open } from '@tauri-apps/plugin-dialog';

  // @ts-ignore - Tauri plugin types
  const dialog = { open };

  // Props
  export let schema: any;
  export let serverName: string;
  export let values: Record<string, any> = {};
  export let errors: Record<string, string> = {};
  export let onValueChange: (name: string, value: any) => void = () => {};

  // Extract properties and required fields from schema
  $: properties = schema?.properties || {};
  $: required = schema?.required || [];

  // Show/hide password fields
  let passwordVisibility: Record<string, boolean> = {};

  function togglePasswordVisibility(name: string) {
    passwordVisibility[name] = !passwordVisibility[name];
    passwordVisibility = { ...passwordVisibility };
  }

  function handleInputChange(name: string, value: any, propertyType: string) {
    // Type coercion
    let typedValue = value;

    if (propertyType === 'integer' || propertyType === 'number') {
      typedValue = value === '' ? null : Number(value);
    } else if (propertyType === 'boolean') {
      typedValue = value === 'true' || value === true;
    } else if (propertyType === 'array') {
      // Arrays can be comma-separated strings or actual arrays
      if (typeof value === 'string') {
        typedValue = value.split(',').map((v) => v.trim()).filter((v) => v);
      }
    }

    onValueChange(name, typedValue);
  }

  async function handleFilePicker(name: string, isDirectory: boolean = false) {
    try {
      const selected = await dialog.open({
        directory: isDirectory,
        multiple: false,
      });

      if (selected) {
        onValueChange(name, selected);
      }
    } catch (error) {
      console.error('File picker error:', error);
    }
  }

  function isRequired(name: string): boolean {
    return required.includes(name);
  }

  function getInputType(propertyType: string, name: string): string {
    // Check if it's a password-like field
    if (
      name.toLowerCase().includes('password') ||
      name.toLowerCase().includes('secret') ||
      name.toLowerCase().includes('token') ||
      name.toLowerCase().includes('key')
    ) {
      return passwordVisibility[name] ? 'text' : 'password';
    }

    // Map JSON schema types to HTML input types
    switch (propertyType) {
      case 'integer':
      case 'number':
        return 'number';
      case 'string':
      default:
        return 'text';
    }
  }

  function isPathField(name: string, description?: string): boolean {
    const nameLower = name.toLowerCase();
    const descLower = (description || '').toLowerCase();

    return (
      nameLower.includes('path') ||
      nameLower.includes('dir') ||
      nameLower.includes('directory') ||
      nameLower.includes('folder') ||
      descLower.includes('path') ||
      descLower.includes('directory')
    );
  }

  function isArrayField(property: any): boolean {
    return property.type === 'array';
  }
</script>

<div class="space-y-4">
  {#if Object.keys(properties).length === 0}
    <div class="text-center py-8 text-gray-500 dark:text-gray-400">
      <Info class="mx-auto mb-2" size={32} />
      <p>No configuration required for this server</p>
    </div>
  {:else}
    {#each Object.entries(properties) as [name, property]}
      {@const propertyType = property.type}
      {@const description = property.description}
      {@const defaultValue = property.default}
      {@const isReq = isRequired(name)}
      {@const isPath = isPathField(name, description)}
      {@const isArray = isArrayField(property)}
      {@const isSecret = name.toLowerCase().includes('password') || name.toLowerCase().includes('secret') || name.toLowerCase().includes('token')}

      <div class="form-field">
        <!-- Label -->
        <label for={name} class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
          {name.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase())}
          {#if isReq}
            <span class="text-red-500">*</span>
          {/if}
        </label>

        <!-- Description -->
        {#if description}
          <p class="text-xs text-gray-500 dark:text-gray-400 mb-2">{description}</p>
        {/if}

        <!-- Input based on type -->
        <div class="relative">
          {#if propertyType === 'boolean'}
            <!-- Boolean (Checkbox) -->
            <div class="flex items-center gap-2">
              <input
                type="checkbox"
                id={name}
                checked={values[name] ?? defaultValue ?? false}
                onchange={(e) => handleInputChange(name, e.currentTarget.checked, propertyType)}
                class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600"
              />
              <span class="text-sm text-gray-600 dark:text-gray-400">
                {defaultValue ? 'Enabled by default' : 'Disabled by default'}
              </span>
            </div>

          {:else if isArray}
            <!-- Array (Textarea or comma-separated) -->
            <textarea
              id={name}
              value={Array.isArray(values[name]) ? values[name].join(', ') : values[name] || (Array.isArray(defaultValue) ? defaultValue.join(', ') : '')}
              oninput={(e) => handleInputChange(name, e.currentTarget.value, propertyType)}
              placeholder={defaultValue ? `Default: ${Array.isArray(defaultValue) ? defaultValue.join(', ') : defaultValue}` : 'Comma-separated values'}
              class={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                errors[name]
                  ? 'border-red-500 dark:border-red-500'
                  : 'border-gray-300 dark:border-gray-600'
              }`}
              rows="3"
            />

          {:else if propertyType === 'integer' || propertyType === 'number'}
            <!-- Number -->
            <input
              type="number"
              id={name}
              value={values[name] ?? defaultValue ?? ''}
              oninput={(e) => handleInputChange(name, e.currentTarget.value, propertyType)}
              placeholder={defaultValue !== undefined ? `Default: ${defaultValue}` : ''}
              class={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                errors[name]
                  ? 'border-red-500 dark:border-red-500'
                  : 'border-gray-300 dark:border-gray-600'
              }`}
              step={propertyType === 'integer' ? '1' : 'any'}
            />

          {:else if isPath}
            <!-- Path/Directory with file picker -->
            <div class="flex gap-2">
              <input
                type="text"
                id={name}
                value={values[name] ?? defaultValue ?? ''}
                oninput={(e) => handleInputChange(name, e.currentTarget.value, propertyType)}
                placeholder={defaultValue || 'Enter path or click browse'}
                class={`flex-1 px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                  errors[name]
                    ? 'border-red-500 dark:border-red-500'
                    : 'border-gray-300 dark:border-gray-600'
                }`}
              />
              <button
                type="button"
                onclick={() => handleFilePicker(name, name.toLowerCase().includes('dir') || name.toLowerCase().includes('folder'))}
                class="px-3 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors flex items-center gap-2"
                title="Browse for {name.toLowerCase().includes('dir') || name.toLowerCase().includes('folder') ? 'directory' : 'file'}"
              >
                <Folder size={16} />
                Browse
              </button>
            </div>

          {:else}
            <!-- String (with password visibility toggle if secret) -->
            <div class="relative">
              <input
                type={getInputType(propertyType, name)}
                id={name}
                value={values[name] ?? defaultValue ?? ''}
                oninput={(e) => handleInputChange(name, e.currentTarget.value, propertyType)}
                placeholder={defaultValue || `Enter ${name}`}
                class={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                  isSecret ? 'pr-10' : ''
                } ${
                  errors[name]
                    ? 'border-red-500 dark:border-red-500'
                    : 'border-gray-300 dark:border-gray-600'
                }`}
              />
              {#if isSecret}
                <button
                  type="button"
                  onclick={() => togglePasswordVisibility(name)}
                  class="absolute right-2 top-1/2 transform -translate-y-1/2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
                  title={passwordVisibility[name] ? 'Hide' : 'Show'}
                >
                  {#if passwordVisibility[name]}
                    <EyeOff size={18} />
                  {:else}
                    <Eye size={18} />
                  {/if}
                </button>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Error Message -->
        {#if errors[name]}
          <div class="flex items-center gap-1 mt-1 text-sm text-red-600 dark:text-red-400">
            <AlertCircle size={14} />
            {errors[name]}
          </div>
        {/if}

        <!-- Default Value Hint -->
        {#if defaultValue !== undefined && !isArray && propertyType !== 'boolean'}
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
            Default: {defaultValue}
          </p>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .form-field {
    @apply bg-gray-50 dark:bg-gray-800/50 p-3 rounded-lg;
  }
</style>
