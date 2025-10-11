<script lang="ts">
  import { createLogger } from '$lib/utils/logger';
  import { AlertCircle, Info } from 'lucide-svelte';

  // Initialize scoped logger
  const logger = createLogger('DynamicForm');

  interface ValidationResult {
    isValid: boolean;
    errors: Record<string, string[]>;
  }

  interface JsonSchema {
    type?: string;
    properties?: Record<string, any>;
    required?: string[];
    enum?: any[];
    format?: string;
    minimum?: number;
    maximum?: number;
    minLength?: number;
    maxLength?: number;
    pattern?: string;
    items?: any;
    description?: string;
    examples?: any[];
    default?: any;
    title?: string;
  }

  interface Props {
    schema: JsonSchema;
    values: Record<string, any>;
    onValuesChange: (values: Record<string, any>) => void;
    onValidationChange?: (validation: ValidationResult) => void;
    disabled?: boolean;
  }

  const { schema, values, onValuesChange, onValidationChange, disabled = false }: Props = $props();

  let validationErrors = $state<Record<string, string[]>>({});
  let touched = $state<Record<string, boolean>>({});

  // Initialize values from schema defaults
  $effect(() => {
    if (schema.properties) {
      const newValues = { ...values };
      let hasChanges = false;

      Object.entries(schema.properties).forEach(([key, fieldSchema]) => {
        if (!(key in newValues)) {
          newValues[key] = getDefaultValue(fieldSchema);
          hasChanges = true;
        }
      });

      if (hasChanges) {
        onValuesChange(newValues);
      }
    }
  });

  // Validate values when they change
  $effect(() => {
    const validation = validateValues(values);
    validationErrors = validation.errors;

    if (onValidationChange) {
      onValidationChange(validation);
    }
  });

  function getDefaultValue(fieldSchema: any): any {
    if (fieldSchema.default !== undefined) {
      return fieldSchema.default;
    }

    switch (fieldSchema.type) {
      case 'string':
        return '';
      case 'number':
      case 'integer':
        return fieldSchema.minimum || 0;
      case 'boolean':
        return false;
      case 'array':
        return [];
      case 'object':
        return {};
      default:
        return '';
    }
  }

  function getFieldType(fieldSchema: any): string {
    if (fieldSchema.enum) return 'select';

    switch (fieldSchema.type) {
      case 'string':
        if (fieldSchema.format === 'textarea' || (fieldSchema.maxLength && fieldSchema.maxLength > 100)) {
          return 'textarea';
        }
        if (fieldSchema.format === 'email') return 'email';
        if (fieldSchema.format === 'url') return 'url';
        if (fieldSchema.format === 'password') return 'password';
        if (fieldSchema.format === 'date') return 'date';
        if (fieldSchema.format === 'time') return 'time';
        if (fieldSchema.format === 'datetime-local') return 'datetime-local';
        return 'text';
      case 'number':
        return 'number';
      case 'integer':
        return 'number';
      case 'boolean':
        return 'checkbox';
      case 'array':
        return 'array';
      case 'object':
        return 'object';
      default:
        return 'text';
    }
  }

  function isRequired(fieldName: string): boolean {
    return schema.required?.includes(fieldName) || false;
  }

  function getFieldHint(fieldName: string, fieldSchema: any): string | null {
    // Don't show hints if there's already a description
    if (fieldSchema.description && fieldSchema.description.length > 20) {
      return null;
    }

    const name = fieldName.toLowerCase();
    const type = fieldSchema.type;

    // Common field name patterns with helpful hints
    const hints: Record<string, string> = {
      // File and path related
      'path': 'File or directory path (e.g., /path/to/file.txt)',
      'file': 'File name or path',
      'filename': 'Name of the file including extension',
      'directory': 'Directory path',
      'folder': 'Folder path',

      // Text and content
      'content': 'Text content or data to process',
      'text': 'Text input or message',
      'message': 'Message or text to send',
      'query': 'Search query or question',
      'search': 'Search term or keyword',
      'input': 'Input data or text to process',
      'data': 'Data to process or analyze',
      'prompt': 'Prompt or instruction text',

      // Identifiers
      'id': 'Unique identifier',
      'name': 'Name or identifier',
      'key': 'Key or identifier',
      'token': 'Authentication token or key',
      'code': 'Code snippet or identifier',

      // URLs and network
      'url': 'Web URL (e.g., https://example.com)',
      'uri': 'URI or resource identifier',
      'endpoint': 'API endpoint URL',
      'host': 'Hostname or IP address',
      'port': 'Port number (e.g., 8080)',

      // Time and date
      'date': 'Date (YYYY-MM-DD)',
      'time': 'Time (HH:MM:SS)',
      'timestamp': 'Unix timestamp or ISO date',
      'duration': 'Duration in seconds or milliseconds',

      // Numbers and measurements
      'count': 'Number of items',
      'limit': 'Maximum number or limit',
      'size': 'Size in bytes or count',
      'length': 'Length or duration',
      'amount': 'Numeric amount or quantity',
      'number': 'Numeric value',

      // Configuration
      'config': 'Configuration settings',
      'options': 'Configuration options',
      'settings': 'Settings or preferences',
      'format': 'Output format (e.g., json, xml, csv)',
      'mode': 'Operation mode or type',
      'type': 'Type or category',

      // Common generic names
      'value': 'Value to use or process',
      'parameter': 'Parameter value',
      'argument': 'Argument or parameter',
      'option': 'Option or choice'
    };

    // Check for direct matches
    for (const [pattern, hint] of Object.entries(hints)) {
      if (name.includes(pattern)) {
        return hint;
      }
    }

    // Type-specific hints when no description exists
    if (!fieldSchema.description) {
      switch (type) {
        case 'string':
          if (fieldSchema.format === 'email') return 'Valid email address';
          if (fieldSchema.format === 'url') return 'Valid URL starting with http:// or https://';
          if (fieldSchema.format === 'date') return 'Date in YYYY-MM-DD format';
          if (fieldSchema.minLength || fieldSchema.maxLength) {
            const min = fieldSchema.minLength || 0;
            const max = fieldSchema.maxLength || '∞';
            return `Text between ${min} and ${max} characters`;
          }
          return 'Text input';
        case 'number':
        case 'integer':
          if (fieldSchema.minimum !== undefined || fieldSchema.maximum !== undefined) {
            const min = fieldSchema.minimum || '-∞';
            const max = fieldSchema.maximum || '∞';
            return `Number between ${min} and ${max}`;
          }
          return type === 'integer' ? 'Whole number' : 'Numeric value';
        case 'boolean':
          return 'True or false';
        case 'array':
          return 'List of values (comma-separated)';
        case 'object':
          return 'JSON object or structured data';
      }
    }

    return null;
  }

  function getPlaceholderText(fieldName: string, fieldSchema: any): string {
    // Use explicit example if available
    if (fieldSchema.examples && fieldSchema.examples.length > 0) {
      return fieldSchema.examples[0];
    }

    const name = fieldName.toLowerCase();
    const type = fieldSchema.type;

    // Smart placeholders based on field names
    const placeholders: Record<string, string> = {
      'path': '/path/to/file.txt',
      'file': 'document.pdf',
      'filename': 'document.pdf',
      'directory': '/home/user/documents',
      'folder': '/home/user/documents',
      'url': 'https://example.com',
      'uri': 'https://example.com/resource',
      'host': 'localhost',
      'hostname': 'example.com',
      'port': '8080',
      'email': 'user@example.com',
      'name': 'John Doe',
      'query': 'search term',
      'search': 'search term',
      'message': 'Enter your message here',
      'text': 'Enter text here',
      'content': 'Content to process',
      'input': 'Input data',
      'data': 'Data to process',
      'id': 'unique-id-123',
      'token': 'your-api-token',
      'key': 'api-key-here',
      'date': '2024-01-01',
      'time': '14:30:00',
      'count': '10',
      'limit': '100',
      'size': '1024',
      'amount': '50',
      'number': '42',
      'value': 'Enter value',
    };

    // Check for matches
    for (const [pattern, placeholder] of Object.entries(placeholders)) {
      if (name.includes(pattern)) {
        return placeholder;
      }
    }

    // Type-based fallbacks
    switch (type) {
      case 'string':
        if (fieldSchema.format === 'email') return 'user@example.com';
        if (fieldSchema.format === 'url') return 'https://example.com';
        if (fieldSchema.format === 'date') return '2024-01-01';
        if (fieldSchema.format === 'time') return '14:30:00';
        return 'Enter text...';
      case 'number':
      case 'integer':
        return fieldSchema.minimum ? String(fieldSchema.minimum) : '0';
      case 'array':
        return 'item1, item2, item3';
      case 'object':
        return '{"key": "value"}';
      default:
        return '';
    }
  }

  function validateField(fieldName: string, value: any, fieldSchema: any): string[] {
    const errors: string[] = [];

    // Required validation
    if (isRequired(fieldName) && (value === undefined || value === null || value === '')) {
      errors.push(`${fieldName} is required`);
      return errors; // Return early for required fields
    }

    // Skip validation if value is empty and not required
    if (value === undefined || value === null || value === '') {
      return errors;
    }

    // Type-specific validation
    switch (fieldSchema.type) {
      case 'string':
        if (typeof value !== 'string') {
          errors.push(`${fieldName} must be a string`);
          break;
        }

        if (fieldSchema.minLength && value.length < fieldSchema.minLength) {
          errors.push(`${fieldName} must be at least ${fieldSchema.minLength} characters`);
        }

        if (fieldSchema.maxLength && value.length > fieldSchema.maxLength) {
          errors.push(`${fieldName} must be no more than ${fieldSchema.maxLength} characters`);
        }

        if (fieldSchema.pattern) {
          try {
            const regex = new RegExp(fieldSchema.pattern);
            if (!regex.test(value)) {
              errors.push(`${fieldName} format is invalid`);
            }
          } catch (e) {
            logger.warn('Invalid regex pattern in schema:', fieldSchema.pattern);
          }
        }

        // Format validation
        if (fieldSchema.format) {
          switch (fieldSchema.format) {
            case 'email':
              const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
              if (!emailRegex.test(value)) {
                errors.push(`${fieldName} must be a valid email address`);
              }
              break;
            case 'url':
              try {
                new URL(value);
              } catch {
                errors.push(`${fieldName} must be a valid URL`);
              }
              break;
          }
        }
        break;

      case 'number':
      case 'integer':
        const numValue = typeof value === 'string' ? parseFloat(value) : value;

        if (isNaN(numValue)) {
          errors.push(`${fieldName} must be a valid number`);
          break;
        }

        if (fieldSchema.type === 'integer' && !Number.isInteger(numValue)) {
          errors.push(`${fieldName} must be an integer`);
        }

        if (fieldSchema.minimum !== undefined && numValue < fieldSchema.minimum) {
          errors.push(`${fieldName} must be at least ${fieldSchema.minimum}`);
        }

        if (fieldSchema.maximum !== undefined && numValue > fieldSchema.maximum) {
          errors.push(`${fieldName} must be no more than ${fieldSchema.maximum}`);
        }
        break;

      case 'array':
        if (!Array.isArray(value)) {
          errors.push(`${fieldName} must be an array`);
          break;
        }

        if (fieldSchema.minItems && value.length < fieldSchema.minItems) {
          errors.push(`${fieldName} must have at least ${fieldSchema.minItems} items`);
        }

        if (fieldSchema.maxItems && value.length > fieldSchema.maxItems) {
          errors.push(`${fieldName} must have no more than ${fieldSchema.maxItems} items`);
        }
        break;

      case 'object':
        if (typeof value !== 'object' || Array.isArray(value)) {
          errors.push(`${fieldName} must be an object`);
        }
        break;
    }

    // Enum validation
    if (fieldSchema.enum && !fieldSchema.enum.includes(value)) {
      errors.push(`${fieldName} must be one of: ${fieldSchema.enum.join(', ')}`);
    }

    return errors;
  }

  function validateValues(values: Record<string, any>): ValidationResult {
    const errors: Record<string, string[]> = {};
    let isValid = true;

    if (schema.properties) {
      Object.entries(schema.properties).forEach(([fieldName, fieldSchema]) => {
        const fieldErrors = validateField(fieldName, values[fieldName], fieldSchema);
        if (fieldErrors.length > 0) {
          errors[fieldName] = fieldErrors;
          isValid = false;
        }
      });
    }

    return { isValid, errors };
  }

  function updateValue(fieldName: string, value: any) {
    const newValues = { ...values, [fieldName]: value };
    onValuesChange(newValues);

    // Mark field as touched
    touched[fieldName] = true;
  }

  function parseArrayValue(value: string): any[] {
    if (!value.trim()) return [];

    try {
      // Try parsing as JSON first
      const parsed = JSON.parse(value);
      return Array.isArray(parsed) ? parsed : [parsed];
    } catch {
      // Fall back to comma-separated values
      return value.split(',').map(item => item.trim()).filter(item => item);
    }
  }

  function parseObjectValue(value: string): any {
    if (!value.trim()) return {};

    try {
      const parsed = JSON.parse(value);
      return typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
    } catch {
      return {};
    }
  }

  function getDisplayValue(value: any, fieldType: string): string {
    if (value === undefined || value === null) return '';

    if (fieldType === 'array' || fieldType === 'object') {
      return JSON.stringify(value, null, 2);
    }

    return String(value);
  }
</script>

<div class="space-y-4">
  {#if schema.properties}
    {#each Object.entries(schema.properties) as [fieldName, fieldSchemaRaw]}
      {@const fieldSchema = fieldSchemaRaw as any}
      {@const fieldType = getFieldType(fieldSchema)}
      {@const fieldErrors = validationErrors[fieldName] || []}
      {@const hasError = fieldErrors.length > 0 && touched[fieldName]}

      <div class="form-group">
        <label class="form-label" for={`field-${fieldName}`}>
          {fieldSchema.title || fieldName}
          {#if isRequired(fieldName)}
            <span class="text-red-500">*</span>
          {/if}
        </label>

        {#if fieldSchema.description || getFieldHint(fieldName, fieldSchema)}
          <div class="flex items-start gap-2 text-xs text-gray-600 mb-2">
            <Info size={12} class="mt-0.5 flex-shrink-0" />
            <div>
              {#if fieldSchema.description}
                <p class="mb-1">{fieldSchema.description}</p>
              {/if}
              {#if getFieldHint(fieldName, fieldSchema)}
                <p class="text-gray-500 italic">{getFieldHint(fieldName, fieldSchema)}</p>
              {/if}
              {#if fieldSchema.examples && fieldSchema.examples.length > 0}
                <p class="text-gray-500 mt-1">
                  <span class="font-medium">Examples:</span>
                  {fieldSchema.examples.slice(0, 3).join(', ')}
                  {#if fieldSchema.examples.length > 3}...{/if}
                </p>
              {/if}
            </div>
          </div>
        {/if}

        <!-- Form field rendering -->
        {#if fieldType === 'select'}
          <select
            bind:value={values[fieldName]}
            onchange={(e) => updateValue(fieldName, (e.target as HTMLSelectElement).value)}
            class="form-input {hasError ? 'border-red-500 focus:border-red-500' : ''}"
            id={`field-${fieldName}`}
            {disabled}
          >
            <option value="">Select an option...</option>
            {#each fieldSchema.enum as option}
              <option value={option}>{option}</option>
            {/each}
          </select>

        {:else if fieldType === 'textarea'}
          <textarea
            value={getDisplayValue(values[fieldName], fieldType)}
            oninput={(e) => updateValue(fieldName, (e.target as HTMLTextAreaElement).value)}
            class="form-input h-32 resize-vertical {hasError ? 'border-red-500 focus:border-red-500' : ''}"
            placeholder={getPlaceholderText(fieldName, fieldSchema)}
            id={`field-${fieldName}`}
            {disabled}
          ></textarea>

        {:else if fieldType === 'checkbox'}
          <label class="flex items-center">
            <input
              type="checkbox"
              bind:checked={values[fieldName]}
              onchange={(e) => updateValue(fieldName, (e.target as HTMLInputElement).checked)}
              class="rounded border-gray-300 text-mcp-primary-600 {hasError ? 'border-red-500' : ''}"
              id={`field-${fieldName}`}
              {disabled}
            />
            <span class="ml-2 text-sm text-gray-700">
              {fieldSchema.title || `Enable ${fieldName}`}
            </span>
          </label>

        {:else if fieldType === 'array'}
          <div class="space-y-2">
            <textarea
              value={getDisplayValue(values[fieldName], fieldType)}
              oninput={(e) => updateValue(fieldName, parseArrayValue((e.target as HTMLTextAreaElement).value))}
              class="form-input h-20 resize-vertical font-mono text-sm {hasError ? 'border-red-500 focus:border-red-500' : ''}"
              placeholder={getPlaceholderText(fieldName, fieldSchema)}
              id={`field-${fieldName}`}
              {disabled}
            ></textarea>
            <p class="text-xs text-gray-500">
              Enter as JSON array (e.g., ["item1", "item2"]) or comma-separated values
            </p>
          </div>

        {:else if fieldType === 'object'}
          <div class="space-y-2">
            <textarea
              value={getDisplayValue(values[fieldName], fieldType)}
              oninput={(e) => updateValue(fieldName, parseObjectValue((e.target as HTMLTextAreaElement).value))}
              class="form-input h-24 resize-vertical font-mono text-sm {hasError ? 'border-red-500 focus:border-red-500' : ''}"
              placeholder={getPlaceholderText(fieldName, fieldSchema)}
              id={`field-${fieldName}`}
              {disabled}
            ></textarea>
            <p class="text-xs text-gray-500">
              Enter as JSON object (e.g., {`{"key": "value"}`})
            </p>
          </div>

        {:else}
          <input
            type={fieldType}
            value={getDisplayValue(values[fieldName], fieldType)}
            oninput={(e) => {
              const target = e.target as HTMLInputElement;
              let value: any = target.value;

              // Convert number inputs
              if (fieldType === 'number' && value !== '') {
                value = fieldSchema.type === 'integer' ? parseInt(value) : parseFloat(value);
              }

              updateValue(fieldName, value);
            }}
            class="form-input {hasError ? 'border-red-500 focus:border-red-500' : ''}"
            placeholder={getPlaceholderText(fieldName, fieldSchema)}
            min={fieldSchema.minimum}
            max={fieldSchema.maximum}
            minlength={fieldSchema.minLength}
            maxlength={fieldSchema.maxLength}
            pattern={fieldSchema.pattern}
            required={isRequired(fieldName)}
            id={`field-${fieldName}`}
            {disabled}
          />
        {/if}

        <!-- Field errors -->
        {#if hasError}
          <div class="mt-1 space-y-1">
            {#each fieldErrors as error}
              <div class="flex items-center gap-1 text-xs text-red-600">
                <AlertCircle size={12} />
                <span>{error}</span>
              </div>
            {/each}
          </div>
        {/if}

        <!-- Field hints -->
        {#if fieldSchema.examples && fieldSchema.examples.length > 0 && !hasError}
          <p class="text-xs text-gray-500 mt-1">
            Example: {Array.isArray(fieldSchema.examples[0]) ? JSON.stringify(fieldSchema.examples[0]) : fieldSchema.examples[0]}
          </p>
        {/if}

        <!-- Field constraints -->
        {#if !hasError && (fieldSchema.minimum !== undefined || fieldSchema.maximum !== undefined || fieldSchema.minLength || fieldSchema.maxLength)}
          <div class="text-xs text-gray-500 mt-1 space-y-1">
            {#if fieldSchema.type === 'string'}
              {#if fieldSchema.minLength || fieldSchema.maxLength}
                <p>Length: {fieldSchema.minLength || 0} - {fieldSchema.maxLength || '∞'} characters</p>
              {/if}
            {:else if fieldSchema.type === 'number' || fieldSchema.type === 'integer'}
              {#if fieldSchema.minimum !== undefined || fieldSchema.maximum !== undefined}
                <p>Range: {fieldSchema.minimum ?? '-∞'} - {fieldSchema.maximum ?? '∞'}</p>
              {/if}
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  {:else}
    <div class="text-center py-8">
      <Info size={32} class="mx-auto text-gray-400 mb-2" />
      <p class="text-sm text-gray-600">No parameters defined in schema</p>
    </div>
  {/if}
</div>

<style>
  .form-group {
    @apply space-y-2;
  }
</style>
