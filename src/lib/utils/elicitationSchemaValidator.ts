/**
 * Elicitation Schema Validator
 *
 * Validates elicitation schemas against MCP 2025-06-18 specification
 * MCP elicitation schemas are RESTRICTED to primitive types only
 */

export interface SchemaValidationError {
  path: string;
  field: string;
  error: string;
  severity: 'error' | 'warning' | 'info';
  suggestion?: string;
}

export interface ValidationResult {
  valid: boolean;
  errors: SchemaValidationError[];
  warnings: SchemaValidationError[];
}

/**
 * Validate an elicitation schema against MCP specification
 */
export function validateElicitationSchema(schema: any): ValidationResult {
  const errors: SchemaValidationError[] = [];
  const warnings: SchemaValidationError[] = [];

  if (!schema) {
    errors.push({
      path: 'schema',
      field: 'root',
      error: 'Schema is required',
      severity: 'error',
      suggestion: 'Provide a valid JSON Schema object'
    });
    return { valid: false, errors, warnings };
  }

  // ========================================
  // ROOT SCHEMA VALIDATION
  // ========================================

  // MCP spec requires type: "object"
  if (schema.type !== 'object') {
    errors.push({
      path: 'type',
      field: 'type',
      error: `Schema type must be "object", got "${schema.type}"`,
      severity: 'error',
      suggestion: 'MCP elicitation schemas must be type "object"'
    });
  }

  // Must have properties
  if (!schema.properties || typeof schema.properties !== 'object') {
    errors.push({
      path: 'properties',
      field: 'properties',
      error: 'Schema must have a "properties" object',
      severity: 'error',
      suggestion: 'Add at least one property to collect user input'
    });
    return { valid: false, errors, warnings };
  }

  if (Object.keys(schema.properties).length === 0) {
    warnings.push({
      path: 'properties',
      field: 'properties',
      error: 'Schema has no properties',
      severity: 'warning',
      suggestion: 'Add properties to collect user input'
    });
  }

  // ========================================
  // PROPERTY VALIDATION
  // ========================================

  Object.entries(schema.properties).forEach(([propName, propDef]: [string, any]) => {
    const propPath = `properties.${propName}`;

    // Must have type
    if (!propDef.type) {
      errors.push({
        path: propPath,
        field: `${propName}.type`,
        error: 'Property must have a "type" field',
        severity: 'error',
        suggestion: 'Specify type: "string", "number", "integer", or "boolean"'
      });
      return;
    }

    // MCP CRITICAL: Only primitives allowed
    const allowedTypes = ['string', 'number', 'integer', 'boolean'];
    if (!allowedTypes.includes(propDef.type)) {
      errors.push({
        path: propPath,
        field: `${propName}.type`,
        error: `Type "${propDef.type}" not allowed in MCP elicitation schemas`,
        severity: 'error',
        suggestion: `MCP only supports primitive types: ${allowedTypes.join(', ')}`
      });

      // Specific guidance for common mistakes
      if (propDef.type === 'object') {
        errors.push({
          path: propPath,
          field: `${propName}`,
          error: 'Nested objects are not allowed',
          severity: 'error',
          suggestion: 'Flatten the schema to use only primitive types'
        });
      }
      if (propDef.type === 'array') {
        errors.push({
          path: propPath,
          field: `${propName}`,
          error: 'Arrays are not allowed',
          severity: 'error',
          suggestion: 'Use multiple individual fields or a comma-separated string'
        });
      }
    }

    // String-specific validation
    if (propDef.type === 'string') {
      validateStringProperty(propName, propDef, propPath, errors, warnings);
    }

    // Number-specific validation
    if (propDef.type === 'number' || propDef.type === 'integer') {
      validateNumberProperty(propName, propDef, propPath, errors, warnings);
    }

    // Check for unsupported JSON Schema features
    validateUnsupportedFeatures(propName, propDef, propPath, warnings);
  });

  // ========================================
  // REQUIRED FIELDS VALIDATION
  // ========================================

  if (schema.required) {
    if (!Array.isArray(schema.required)) {
      errors.push({
        path: 'required',
        field: 'required',
        error: '"required" must be an array',
        severity: 'error',
        suggestion: 'Use an array of property names: ["field1", "field2"]'
      });
    } else {
      schema.required.forEach((fieldName: string) => {
        if (!schema.properties[fieldName]) {
          errors.push({
            path: 'required',
            field: fieldName,
            error: `Required field "${fieldName}" does not exist in properties`,
            severity: 'error',
            suggestion: `Add "${fieldName}" to properties or remove from required array`
          });
        }
      });
    }
  }

  // ========================================
  // USABILITY WARNINGS
  // ========================================

  // Warn if no required fields
  if (!schema.required || schema.required.length === 0) {
    warnings.push({
      path: 'required',
      field: 'required',
      error: 'No required fields',
      severity: 'info',
      suggestion: 'Consider marking important fields as required'
    });
  }

  // Warn if no titles/descriptions
  let missingDescriptions = 0;
  Object.entries(schema.properties).forEach(([propName, propDef]: [string, any]) => {
    if (!propDef.title && !propDef.description) {
      missingDescriptions++;
    }
  });

  if (missingDescriptions > 0) {
    warnings.push({
      path: 'properties',
      field: 'descriptions',
      error: `${missingDescriptions} field(s) missing title or description`,
      severity: 'info',
      suggestion: 'Add titles and descriptions to help users understand what to enter'
    });
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings
  };
}

/**
 * Validate string property constraints
 */
function validateStringProperty(
  propName: string,
  propDef: any,
  propPath: string,
  errors: SchemaValidationError[],
  warnings: SchemaValidationError[]
) {
  // Format validation
  if (propDef.format) {
    const allowedFormats = ['email', 'uri', 'date', 'date-time'];
    if (!allowedFormats.includes(propDef.format)) {
      warnings.push({
        path: propPath,
        field: `${propName}.format`,
        error: `Format "${propDef.format}" may not be supported`,
        severity: 'warning',
        suggestion: `Supported formats: ${allowedFormats.join(', ')}`
      });
    }
  }

  // Length constraints
  if (propDef.minLength !== undefined && propDef.maxLength !== undefined) {
    if (propDef.minLength > propDef.maxLength) {
      errors.push({
        path: propPath,
        field: `${propName}.minLength`,
        error: 'minLength cannot be greater than maxLength',
        severity: 'error'
      });
    }
  }

  // Enum validation
  if (propDef.enum) {
    if (!Array.isArray(propDef.enum)) {
      errors.push({
        path: propPath,
        field: `${propName}.enum`,
        error: '"enum" must be an array',
        severity: 'error',
        suggestion: 'Use an array of string values: ["option1", "option2"]'
      });
    } else if (propDef.enum.length === 0) {
      warnings.push({
        path: propPath,
        field: `${propName}.enum`,
        error: 'Enum has no options',
        severity: 'warning',
        suggestion: 'Add at least one option to the enum array'
      });
    }

    // Validate enum values are strings
    if (Array.isArray(propDef.enum)) {
      propDef.enum.forEach((value: any, index: number) => {
        if (typeof value !== 'string') {
          errors.push({
            path: `${propPath}.enum[${index}]`,
            field: `${propName}.enum`,
            error: `Enum value must be a string, got ${typeof value}`,
            severity: 'error',
            suggestion: 'All enum values must be strings'
          });
        }
      });
    }
  }
}

/**
 * Validate number/integer property constraints
 */
function validateNumberProperty(
  propName: string,
  propDef: any,
  propPath: string,
  errors: SchemaValidationError[],
  warnings: SchemaValidationError[]
) {
  // Range validation
  if (propDef.minimum !== undefined && propDef.maximum !== undefined) {
    if (propDef.minimum > propDef.maximum) {
      errors.push({
        path: propPath,
        field: `${propName}.minimum`,
        error: 'minimum cannot be greater than maximum',
        severity: 'error'
      });
    }
  }

  // Integer validation
  if (propDef.type === 'integer') {
    if (propDef.minimum !== undefined && !Number.isInteger(propDef.minimum)) {
      warnings.push({
        path: propPath,
        field: `${propName}.minimum`,
        error: 'minimum should be an integer for integer type',
        severity: 'warning'
      });
    }
    if (propDef.maximum !== undefined && !Number.isInteger(propDef.maximum)) {
      warnings.push({
        path: propPath,
        field: `${propName}.maximum`,
        error: 'maximum should be an integer for integer type',
        severity: 'warning'
      });
    }
  }
}

/**
 * Check for unsupported JSON Schema features
 */
function validateUnsupportedFeatures(
  propName: string,
  propDef: any,
  propPath: string,
  warnings: SchemaValidationError[]
) {
  const unsupportedFeatures = [
    'allOf', 'anyOf', 'oneOf', 'not',
    'if', 'then', 'else',
    'dependencies', 'dependentRequired', 'dependentSchemas',
    '$ref', 'definitions', '$defs'
  ];

  unsupportedFeatures.forEach(feature => {
    if (propDef[feature] !== undefined) {
      warnings.push({
        path: propPath,
        field: `${propName}.${feature}`,
        error: `"${feature}" is not supported in MCP elicitation schemas`,
        severity: 'warning',
        suggestion: 'Simplify schema to use only primitive types with basic constraints'
      });
    }
  });

  // Check for nested properties
  if (propDef.properties) {
    warnings.push({
      path: propPath,
      field: `${propName}.properties`,
      error: 'Nested properties are not supported',
      severity: 'warning',
      suggestion: 'Flatten the schema to use only top-level primitive properties'
    });
  }

  // Check for items (array schema)
  if (propDef.items) {
    warnings.push({
      path: propPath,
      field: `${propName}.items`,
      error: 'Array items schema is not supported',
      severity: 'warning',
      suggestion: 'Arrays are not allowed in MCP elicitation schemas'
    });
  }
}

/**
 * Get a friendly error summary
 */
export function getValidationSummary(result: ValidationResult): string {
  if (result.valid && result.warnings.length === 0) {
    return '✓ Schema is valid';
  }

  const parts: string[] = [];

  if (result.errors.length > 0) {
    parts.push(`${result.errors.length} error${result.errors.length > 1 ? 's' : ''}`);
  }

  if (result.warnings.length > 0) {
    parts.push(`${result.warnings.length} warning${result.warnings.length > 1 ? 's' : ''}`);
  }

  return parts.join(', ');
}

/**
 * Example valid schema for reference
 */
export const EXAMPLE_VALID_SCHEMA = {
  type: 'object',
  properties: {
    environment: {
      type: 'string',
      enum: ['dev', 'staging', 'prod'],
      title: 'Environment',
      description: 'Deployment environment'
    },
    replicas: {
      type: 'integer',
      title: 'Number of Replicas',
      description: 'How many instances to deploy',
      minimum: 1,
      maximum: 10
    },
    enableLogging: {
      type: 'boolean',
      title: 'Enable Logging',
      description: 'Turn on verbose logging',
      default: true
    },
    apiKey: {
      type: 'string',
      title: 'API Key',
      description: 'Your API key for authentication',
      minLength: 10
    }
  },
  required: ['environment', 'replicas']
};
