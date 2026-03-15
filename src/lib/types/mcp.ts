/**
 * MCP Protocol Types
 * 
 * Core type definitions for Model Context Protocol primitives.
 */

export interface Prompt {
  name: string;
  description?: string;
  arguments?: PromptArgument[];
}

export interface PromptArgument {
  name: string;
  description?: string;
  required?: boolean;
}

export interface Resource {
  uri: string;
  name: string;
  description?: string;
  mimeType?: string;
  annotations?: Record<string, string>;
}

/**
 * JSON Schema definition for tool inputs and other structured data.
 */
export interface JsonSchema {
  type?: 'string' | 'number' | 'integer' | 'boolean' | 'object' | 'array' | 'null';
  description?: string;
  default?: any;
  required?: string[];
  properties?: Record<string, JsonSchema>;
  items?: JsonSchema;
  enum?: any[];
  anyOf?: JsonSchema[];
  allOf?: JsonSchema[];
  oneOf?: JsonSchema[];
  format?: string;
  [key: string]: any; // For other JSON Schema properties
}

export interface Tool {
  name: string;
  description?: string;
  inputSchema: JsonSchema;
}

export interface GetPromptResult {
  description?: string;
  messages: Array<{
    role: 'user' | 'assistant';
    content: {
      type: 'text' | 'resource';
      text?: string;
      resource?: {
        uri: string;
        mimeType?: string;
        text?: string;
        blob?: string;
      };
    };
  }>;
}

export interface ReadResourceResult {
  contents: Array<{
    uri: string;
    mimeType?: string;
    text?: string;
    blob?: string;
  }>;
}
