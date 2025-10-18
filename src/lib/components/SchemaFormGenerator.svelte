<script lang="ts">
  import { Folder, AlertCircle, Info, Eye, EyeOff, ChevronDown, ChevronRight } from 'lucide-svelte';
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
  $: anyOf = schema?.anyOf || [];
  $: hasConditionalRequirements = anyOf.length > 0;

  // Show/hide password fields
  let passwordVisibility: Record<string, boolean> = {};

  // Collapsed state for nested objects
  let collapsedSections: Record<string, boolean> = {};

  function togglePasswordVisibility(name: string) {
    passwordVisibility[name] = !passwordVisibility[name];
    passwordVisibility = { ...passwordVisibility };
  }

  function toggleSection(name: string) {
    collapsedSections[name] = !collapsedSections[name];
    collapsedSections = { ...collapsedSections };
  }

  function handleInputChange(path: string, value: any, propertyType: string) {
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

    onValueChange(path, typedValue);
  }

  async function handleFilePicker(path: string, isDirectory: boolean = false) {
    try {
      const selected = await dialog.open({
        directory: isDirectory,
        multiple: false,
      });

      if (selected) {
        onValueChange(path, selected);
      }
    } catch (error) {
      console.error('File picker error:', error);
    }
  }

  function isRequired(name: string): boolean {
    return required.includes(name);
  }

  function getNestedValue(obj: any, path: string): any {
    const keys = path.split('.');
    let current = obj;
    for (const key of keys) {
      if (current && typeof current === 'object' && key in current) {
        current = current[key];
      } else {
        return undefined;
      }
    }
    return current;
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

  function isObjectField(property: any): boolean {
    return property.type === 'object' && property.properties;
  }

  function formatLabel(name: string): string {
    return name.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase());
  }

  function getConditionalRequirementHint(): string {
    if (anyOf.length === 0) return '';

    const requiredGroups = anyOf.map((group: any) => {
      if (group.required && Array.isArray(group.required)) {
        return group.required.map(formatLabel).join(' + ');
      }
      return '';
    }).filter(Boolean);

    if (requiredGroups.length > 0) {
      return `Required: ${requiredGroups.join(' OR ')}`;
    }

    return '';
  }

  // Render a single field (can be nested)
  function renderField(
    name: string,
    property: any,
    basePath: string = '',
    level: number = 0
  ): { component: any; path: string } {
    const fullPath = basePath ? `${basePath}.${name}` : name;
    return { component: property, path: fullPath };
  }
</script>

<div class="space-y-4">
  <!-- Conditional Requirements Info -->
  {#if hasConditionalRequirements}
    <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-3">
      <div class="flex items-start gap-2">
        <Info class="text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" size={16} />
        <div>
          <p class="text-sm font-medium text-blue-900 dark:text-blue-100">
            Conditional Configuration
          </p>
          <p class="text-xs text-blue-700 dark:text-blue-300 mt-1">
            {getConditionalRequirementHint()}
          </p>
        </div>
      </div>
    </div>
  {/if}

  {#if Object.keys(properties).length === 0}
    <div class="text-center py-8 text-gray-500 dark:text-gray-400">
      <Info class="mx-auto mb-2" size={32} />
      <p>No configuration required for this server</p>
    </div>
  {:else}
    {#each Object.entries(properties) as [name, property]}
      {@const prop = property as any}
      {@const propertyType = prop.type}
      {@const description = prop.description}
      {@const defaultValue = prop.default}
      {@const isReq = isRequired(name)}
      {@const isPath = isPathField(name, description)}
      {@const isArray = isArrayField(prop)}
      {@const isObject = isObjectField(prop)}
      {@const isSecret = name.toLowerCase().includes('password') || name.toLowerCase().includes('secret') || name.toLowerCase().includes('token')}
      {@const isCollapsed = collapsedSections[name] ?? false}

      <div class="form-field">
        {#if isObject}
          <!-- Nested Object - Collapsible Section -->
          <div class="border border-gray-300 dark:border-gray-600 rounded-lg">
            <!-- Section Header -->
            <button
              type="button"
              onclick={() => toggleSection(name)}
              class="w-full flex items-center justify-between p-3 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors rounded-t-lg"
            >
              <div class="flex items-center gap-2 text-left flex-1">
                {#if isCollapsed}
                  <ChevronRight class="text-gray-500" size={18} />
                {:else}
                  <ChevronDown class="text-gray-500" size={18} />
                {/if}
                <div>
                  <span class="font-medium text-gray-900 dark:text-white">
                    {formatLabel(name)}
                    {#if isReq}
                      <span class="text-red-500">*</span>
                    {/if}
                  </span>
                  {#if description}
                    <p class="text-xs text-gray-500 dark:text-gray-400">{description}</p>
                  {/if}
                </div>
              </div>
              {#if isReq}
                <span class="px-2 py-0.5 bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 text-xs rounded-full">
                  Required
                </span>
              {:else}
                <span class="px-2 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 text-xs rounded-full">
                  Optional
                </span>
              {/if}
            </button>

            <!-- Nested Properties -->
            {#if !isCollapsed}
              <div class="p-3 pt-0 space-y-3 border-t border-gray-200 dark:border-gray-700">
                {#each Object.entries(prop.properties) as [nestedName, nestedProperty]}
                  {@const nestedProp = nestedProperty as any}
                  {@const nestedPath = `${name}.${nestedName}`}
                  {@const nestedType = nestedProp.type}
                  {@const nestedDescription = nestedProp.description}
                  {@const nestedDefault = nestedProp.default}
                  {@const nestedRequired = prop.required?.includes(nestedName) || false}
                  {@const nestedIsPath = isPathField(nestedName, nestedDescription)}
                  {@const nestedIsSecret = nestedName.toLowerCase().includes('password') || nestedName.toLowerCase().includes('secret') || nestedName.toLowerCase().includes('token')}
                  {@const nestedValue = getNestedValue(values, nestedPath)}

                  <div class="nested-field">
                    <!-- Nested Field Label -->
                    <label for={nestedPath} class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                      {formatLabel(nestedName)}
                      {#if nestedRequired}
                        <span class="text-red-500">*</span>
                      {/if}
                    </label>

                    <!-- Nested Field Description -->
                    {#if nestedDescription}
                      <p class="text-xs text-gray-500 dark:text-gray-400 mb-2">{nestedDescription}</p>
                    {/if}

                    <!-- Nested Field Input -->
                    <div class="relative">
                      {#if nestedType === 'boolean'}
                        <!-- Boolean -->
                        <div class="flex items-center gap-2">
                          <input
                            type="checkbox"
                            id={nestedPath}
                            checked={nestedValue ?? nestedDefault ?? false}
                            onchange={(e) => handleInputChange(nestedPath, e.currentTarget.checked, nestedType)}
                            class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
                          />
                          <span class="text-sm text-gray-600 dark:text-gray-400">
                            {nestedDefault ? 'Enabled by default' : 'Disabled by default'}
                          </span>
                        </div>

                      {:else if nestedType === 'integer' || nestedType === 'number'}
                        <!-- Number -->
                        <input
                          type="number"
                          id={nestedPath}
                          value={nestedValue ?? nestedDefault ?? ''}
                          oninput={(e) => handleInputChange(nestedPath, e.currentTarget.value, nestedType)}
                          placeholder={nestedDefault !== undefined ? `Default: ${nestedDefault}` : ''}
                          class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                          step={nestedType === 'integer' ? '1' : 'any'}
                        />

                      {:else if nestedIsPath}
                        <!-- Path with file picker -->
                        <div class="flex gap-2">
                          <input
                            type="text"
                            id={nestedPath}
                            value={nestedValue ?? nestedDefault ?? ''}
                            oninput={(e) => handleInputChange(nestedPath, e.currentTarget.value, nestedType)}
                            placeholder={nestedDefault || 'Enter path or click browse'}
                            class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                          />
                          <button
                            type="button"
                            onclick={() => handleFilePicker(nestedPath, nestedName.toLowerCase().includes('dir'))}
                            class="px-3 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 flex items-center gap-2"
                          >
                            <Folder size={16} />
                            Browse
                          </button>
                        </div>

                      {:else}
                        <!-- String (with password visibility) -->
                        <div class="relative">
                          <input
                            type={getInputType(nestedType, nestedName)}
                            id={nestedPath}
                            value={nestedValue ?? nestedDefault ?? ''}
                            oninput={(e) => handleInputChange(nestedPath, e.currentTarget.value, nestedType)}
                            placeholder={nestedDefault || `Enter ${nestedName}`}
                            class={`w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${nestedIsSecret ? 'pr-10' : ''}`}
                          />
                          {#if nestedIsSecret}
                            <button
                              type="button"
                              onclick={() => togglePasswordVisibility(nestedPath)}
                              class="absolute right-2 top-1/2 transform -translate-y-1/2 text-gray-500 hover:text-gray-700 dark:text-gray-400"
                            >
                              {#if passwordVisibility[nestedPath]}
                                <EyeOff size={18} />
                              {:else}
                                <Eye size={18} />
                              {/if}
                            </button>
                          {/if}
                        </div>
                      {/if}
                    </div>

                    <!-- Nested Default Value Hint -->
                    {#if nestedDefault !== undefined && nestedType !== 'boolean'}
                      <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        Default: {nestedDefault}
                      </p>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          </div>

        {:else}
          <!-- Regular Field (non-nested) -->
          <!-- Label -->
          <label for={name} class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            {formatLabel(name)}
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
                  class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
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
                class={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                  errors[name] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'
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
                class={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                  errors[name] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'
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
                  class={`flex-1 px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                    errors[name] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'
                  }`}
                />
                <button
                  type="button"
                  onclick={() => handleFilePicker(name, name.toLowerCase().includes('dir') || name.toLowerCase().includes('folder'))}
                  class="px-3 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 flex items-center gap-2"
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
                  class={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                    isSecret ? 'pr-10' : ''
                  } ${
                    errors[name] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'
                  }`}
                />
                {#if isSecret}
                  <button
                    type="button"
                    onclick={() => togglePasswordVisibility(name)}
                    class="absolute right-2 top-1/2 transform -translate-y-1/2 text-gray-500 hover:text-gray-700 dark:text-gray-400"
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
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .form-field {
    @apply bg-gray-50 dark:bg-gray-800/50 p-3 rounded-lg;
  }

  .nested-field {
    @apply bg-white dark:bg-gray-800 p-3 rounded-lg border border-gray-200 dark:border-gray-700;
  }
</style>
