export interface JsonSchema {
  type?: string;
  properties?: Record<string, JsonSchema>;
  required?: string[];
  enum?: any[];
  format?: string;
  minimum?: number;
  maximum?: number;
  exclusiveMinimum?: number;
  exclusiveMaximum?: number;
  minLength?: number;
  maxLength?: number;
  pattern?: string;
  items?: JsonSchema | JsonSchema[];
  additionalItems?: boolean | JsonSchema;
  minItems?: number;
  maxItems?: number;
  uniqueItems?: boolean;
  additionalProperties?: boolean | JsonSchema;
  minProperties?: number;
  maxProperties?: number;
  description?: string;
  examples?: any[];
  default?: any;
  title?: string;
  anyOf?: JsonSchema[];
  oneOf?: JsonSchema[];
  allOf?: JsonSchema[];
  not?: JsonSchema;
  const?: any;
  if?: JsonSchema;
  then?: JsonSchema;
  else?: JsonSchema;
}

export interface ValidationError {
  path: string;
  message: string;
  value?: any;
  constraint?: any;
}

export interface ValidationResult {
  isValid: boolean;
  errors: ValidationError[];
  warnings?: ValidationError[];
}

export class SchemaValidator {
  private static readonly FORMAT_VALIDATORS: Record<string, RegExp> = {
    email: /^[^\s@]+@[^\s@]+\.[^\s@]+$/,
    uri: /^[a-zA-Z][a-zA-Z0-9+.-]*:/,
    uuid: /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
    'date-time': /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{3})?Z?$/,
    date: /^\d{4}-\d{2}-\d{2}$/,
    time: /^\d{2}:\d{2}:\d{2}$/,
    ipv4: /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/,
    ipv6: /^(?:[0-9a-f]{1,4}:){7}[0-9a-f]{1,4}$/i,
  };

  /**
   * Validates a value against a JSON schema
   */
  static validate(
    value: any,
    schema: JsonSchema,
    path: string = 'root'
  ): ValidationResult {
    const errors: ValidationError[] = [];

    try {
      this.validateValue(value, schema, path, errors);
    } catch (error) {
      errors.push({
        path,
        message: `Validation error: ${error instanceof Error ? error.message : String(error)}`,
        value,
      });
    }

    return {
      isValid: errors.length === 0,
      errors,
    };
  }

  /**
   * Validates multiple values against a schema with properties
   */
  static validateObject(
    values: Record<string, any>,
    schema: JsonSchema
  ): ValidationResult {
    const errors: ValidationError[] = [];

    if (schema.properties) {
      // Validate each property
      Object.entries(schema.properties).forEach(([key, propertySchema]) => {
        const value = values[key];
        const propertyErrors = this.validate(value, propertySchema, key);
        errors.push(...propertyErrors.errors);
      });

      // Check for required properties
      if (schema.required) {
        schema.required.forEach(requiredKey => {
          if (!(requiredKey in values) || values[requiredKey] === undefined || values[requiredKey] === null || values[requiredKey] === '') {
            errors.push({
              path: requiredKey,
              message: `Property '${requiredKey}' is required`,
              value: values[requiredKey],
            });
          }
        });
      }

      // Check for additional properties
      if (schema.additionalProperties === false) {
        Object.keys(values).forEach(key => {
          if (!schema.properties![key]) {
            errors.push({
              path: key,
              message: `Additional property '${key}' is not allowed`,
              value: values[key],
            });
          }
        });
      }

      // Validate property count constraints
      const propertyCount = Object.keys(values).length;
      if (schema.minProperties && propertyCount < schema.minProperties) {
        errors.push({
          path: 'root',
          message: `Object must have at least ${schema.minProperties} properties`,
          value: propertyCount,
          constraint: schema.minProperties,
        });
      }

      if (schema.maxProperties && propertyCount > schema.maxProperties) {
        errors.push({
          path: 'root',
          message: `Object must have no more than ${schema.maxProperties} properties`,
          value: propertyCount,
          constraint: schema.maxProperties,
        });
      }
    }

    return {
      isValid: errors.length === 0,
      errors,
    };
  }

  private static validateValue(
    value: any,
    schema: JsonSchema,
    path: string,
    errors: ValidationError[]
  ): void {
    // Handle null/undefined values
    if (value === null || value === undefined) {
      // Only validate if not optional
      return;
    }

    // Type validation
    if (schema.type) {
      this.validateType(value, schema.type, path, errors);
    }

    // Const validation
    if (schema.const !== undefined) {
      if (value !== schema.const) {
        errors.push({
          path,
          message: `Value must be equal to ${JSON.stringify(schema.const)}`,
          value,
          constraint: schema.const,
        });
      }
    }

    // Enum validation
    if (schema.enum) {
      if (!schema.enum.includes(value)) {
        errors.push({
          path,
          message: `Value must be one of: ${schema.enum.map(v => JSON.stringify(v)).join(', ')}`,
          value,
          constraint: schema.enum,
        });
      }
    }

    // Type-specific validation
    switch (schema.type) {
      case 'string':
        this.validateString(value, schema, path, errors);
        break;
      case 'number':
      case 'integer':
        this.validateNumber(value, schema, path, errors);
        break;
      case 'array':
        this.validateArray(value, schema, path, errors);
        break;
      case 'object':
        this.validateObjectType(value, schema, path, errors);
        break;
    }

    // Logical schema validation (anyOf, oneOf, allOf, not)
    this.validateLogicalSchemas(value, schema, path, errors);
  }

  private static validateType(
    value: any,
    expectedType: string,
    path: string,
    errors: ValidationError[]
  ): void {
    const actualType = this.getValueType(value);

    if (expectedType === 'integer') {
      if (actualType !== 'number' || !Number.isInteger(value)) {
        errors.push({
          path,
          message: `Expected integer, got ${actualType}`,
          value,
          constraint: expectedType,
        });
      }
    } else if (actualType !== expectedType) {
      errors.push({
        path,
        message: `Expected ${expectedType}, got ${actualType}`,
        value,
        constraint: expectedType,
      });
    }
  }

  private static validateString(
    value: string,
    schema: JsonSchema,
    path: string,
    errors: ValidationError[]
  ): void {
    if (typeof value !== 'string') return;

    // Length validation
    if (schema.minLength !== undefined && value.length < schema.minLength) {
      errors.push({
        path,
        message: `String must be at least ${schema.minLength} characters long`,
        value: value.length,
        constraint: schema.minLength,
      });
    }

    if (schema.maxLength !== undefined && value.length > schema.maxLength) {
      errors.push({
        path,
        message: `String must be no more than ${schema.maxLength} characters long`,
        value: value.length,
        constraint: schema.maxLength,
      });
    }

    // Pattern validation
    if (schema.pattern) {
      try {
        const regex = new RegExp(schema.pattern);
        if (!regex.test(value)) {
          errors.push({
            path,
            message: `String does not match required pattern`,
            value,
            constraint: schema.pattern,
          });
        }
      } catch (error) {
        errors.push({
          path,
          message: `Invalid pattern in schema: ${schema.pattern}`,
          value,
          constraint: schema.pattern,
        });
      }
    }

    // Format validation
    if (schema.format) {
      this.validateFormat(value, schema.format, path, errors);
    }
  }

  private static validateNumber(
    value: number,
    schema: JsonSchema,
    path: string,
    errors: ValidationError[]
  ): void {
    if (typeof value !== 'number') return;

    // Range validation
    if (schema.minimum !== undefined && value < schema.minimum) {
      errors.push({
        path,
        message: `Number must be at least ${schema.minimum}`,
        value,
        constraint: schema.minimum,
      });
    }

    if (schema.maximum !== undefined && value > schema.maximum) {
      errors.push({
        path,
        message: `Number must be no more than ${schema.maximum}`,
        value,
        constraint: schema.maximum,
      });
    }

    if (schema.exclusiveMinimum !== undefined && value <= schema.exclusiveMinimum) {
      errors.push({
        path,
        message: `Number must be greater than ${schema.exclusiveMinimum}`,
        value,
        constraint: schema.exclusiveMinimum,
      });
    }

    if (schema.exclusiveMaximum !== undefined && value >= schema.exclusiveMaximum) {
      errors.push({
        path,
        message: `Number must be less than ${schema.exclusiveMaximum}`,
        value,
        constraint: schema.exclusiveMaximum,
      });
    }
  }

  private static validateArray(
    value: any[],
    schema: JsonSchema,
    path: string,
    errors: ValidationError[]
  ): void {
    if (!Array.isArray(value)) return;

    // Length validation
    if (schema.minItems !== undefined && value.length < schema.minItems) {
      errors.push({
        path,
        message: `Array must have at least ${schema.minItems} items`,
        value: value.length,
        constraint: schema.minItems,
      });
    }

    if (schema.maxItems !== undefined && value.length > schema.maxItems) {
      errors.push({
        path,
        message: `Array must have no more than ${schema.maxItems} items`,
        value: value.length,
        constraint: schema.maxItems,
      });
    }

    // Unique items validation
    if (schema.uniqueItems) {
      const seen = new Set();
      value.forEach((item, index) => {
        const itemStr = JSON.stringify(item);
        if (seen.has(itemStr)) {
          errors.push({
            path: `${path}[${index}]`,
            message: `Array items must be unique`,
            value: item,
          });
        }
        seen.add(itemStr);
      });
    }

    // Items validation
    if (schema.items) {
      if (Array.isArray(schema.items)) {
        // Tuple validation
        value.forEach((item, index) => {
          if (index < schema.items!.length) {
            this.validateValue(item, (schema.items as JsonSchema[])[index], `${path}[${index}]`, errors);
          }
        });
      } else {
        // All items validation
        value.forEach((item, index) => {
          this.validateValue(item, schema.items as JsonSchema, `${path}[${index}]`, errors);
        });
      }
    }
  }

  private static validateObjectType(
    value: any,
    schema: JsonSchema,
    path: string,
    errors: ValidationError[]
  ): void {
    if (typeof value !== 'object' || value === null || Array.isArray(value)) return;

    if (schema.properties) {
      Object.entries(schema.properties).forEach(([key, propertySchema]) => {
        if (key in value) {
          this.validateValue(value[key], propertySchema, `${path}.${key}`, errors);
        }
      });
    }
  }

  private static validateFormat(
    value: string,
    format: string,
    path: string,
    errors: ValidationError[]
  ): void {
    const validator = this.FORMAT_VALIDATORS[format];

    if (validator && !validator.test(value)) {
      errors.push({
        path,
        message: `String does not match format '${format}'`,
        value,
        constraint: format,
      });
    } else if (!validator) {
      // Custom format validation for known formats
      switch (format) {
        case 'url':
          try {
            new URL(value);
          } catch {
            errors.push({
              path,
              message: `String is not a valid URL`,
              value,
              constraint: format,
            });
          }
          break;
      }
    }
  }

  private static validateLogicalSchemas(
    value: any,
    schema: JsonSchema,
    path: string,
    errors: ValidationError[]
  ): void {
    // anyOf validation
    if (schema.anyOf) {
      const anyOfValid = schema.anyOf.some(subSchema => {
        const result = this.validate(value, subSchema, path);
        return result.isValid;
      });

      if (!anyOfValid) {
        errors.push({
          path,
          message: `Value does not match any of the allowed schemas`,
          value,
        });
      }
    }

    // oneOf validation
    if (schema.oneOf) {
      const validSchemas = schema.oneOf.filter(subSchema => {
        const result = this.validate(value, subSchema, path);
        return result.isValid;
      });

      if (validSchemas.length === 0) {
        errors.push({
          path,
          message: `Value does not match any of the required schemas`,
          value,
        });
      } else if (validSchemas.length > 1) {
        errors.push({
          path,
          message: `Value matches more than one schema (should match exactly one)`,
          value,
        });
      }
    }

    // allOf validation
    if (schema.allOf) {
      schema.allOf.forEach(subSchema => {
        this.validateValue(value, subSchema, path, errors);
      });
    }

    // not validation
    if (schema.not) {
      const result = this.validate(value, schema.not, path);
      if (result.isValid) {
        errors.push({
          path,
          message: `Value must not match the specified schema`,
          value,
        });
      }
    }
  }

  private static getValueType(value: any): string {
    if (value === null) return 'null';
    if (Array.isArray(value)) return 'array';
    return typeof value;
  }

  /**
   * Generates default values from a schema
   */
  static generateDefaults(schema: JsonSchema): any {
    if (schema.default !== undefined) {
      return schema.default;
    }

    if (schema.const !== undefined) {
      return schema.const;
    }

    if (schema.enum && schema.enum.length > 0) {
      return schema.enum[0];
    }

    switch (schema.type) {
      case 'string':
        return '';
      case 'number':
      case 'integer':
        return schema.minimum ?? 0;
      case 'boolean':
        return false;
      case 'array':
        return [];
      case 'object':
        if (schema.properties) {
          const obj: Record<string, any> = {};
          Object.entries(schema.properties).forEach(([key, propertySchema]) => {
            obj[key] = this.generateDefaults(propertySchema);
          });
          return obj;
        }
        return {};
      case 'null':
        return null;
      default:
        return undefined;
    }
  }

  /**
   * Checks if a field is required
   */
  static isRequired(fieldName: string, schema: JsonSchema): boolean {
    return schema.required?.includes(fieldName) ?? false;
  }

  /**
   * Gets human-readable validation constraints for display
   */
  static getConstraintDescription(schema: JsonSchema): string[] {
    const constraints: string[] = [];

    switch (schema.type) {
      case 'string':
        if (schema.minLength !== undefined || schema.maxLength !== undefined) {
          constraints.push(`Length: ${schema.minLength ?? 0}-${schema.maxLength ?? '∞'} characters`);
        }
        if (schema.pattern) {
          constraints.push(`Pattern: ${schema.pattern}`);
        }
        if (schema.format) {
          constraints.push(`Format: ${schema.format}`);
        }
        break;

      case 'number':
      case 'integer':
        if (schema.minimum !== undefined || schema.maximum !== undefined) {
          constraints.push(`Range: ${schema.minimum ?? '-∞'} to ${schema.maximum ?? '∞'}`);
        }
        break;

      case 'array':
        if (schema.minItems !== undefined || schema.maxItems !== undefined) {
          constraints.push(`Items: ${schema.minItems ?? 0}-${schema.maxItems ?? '∞'}`);
        }
        if (schema.uniqueItems) {
          constraints.push('Items must be unique');
        }
        break;
    }

    if (schema.enum) {
      constraints.push(`Must be one of: ${schema.enum.join(', ')}`);
    }

    return constraints;
  }
}

/**
 * Utility function for common validation scenarios
 */
export function validateToolParameters(
  parameters: Record<string, any>,
  schema: JsonSchema
): ValidationResult {
  return SchemaValidator.validateObject(parameters, schema);
}

/**
 * Generates default parameter values from tool schema
 */
export function generateDefaultParameters(schema: JsonSchema): Record<string, any> {
  if (schema.properties) {
    const defaults: Record<string, any> = {};
    Object.entries(schema.properties).forEach(([key, propertySchema]) => {
      defaults[key] = SchemaValidator.generateDefaults(propertySchema);
    });
    return defaults;
  }
  return {};
}