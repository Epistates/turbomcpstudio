import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export interface ServerConfig {
  id: string;
  name: string;
  description?: string;
  transport_config: TransportConfig;
  environment_variables: Record<string, string>;
  created_at: string;
  updated_at: string;
}

export type TransportConfig =
  | { type: 'stdio'; command: string; args: string[]; working_directory?: string }
  | { type: 'http'; url: string; headers: Record<string, string> }
  | { type: 'websocket'; url: string; headers: Record<string, string> }
  | { type: 'tcp'; host: string; port: number }
  | { type: 'unix'; path: string };

export interface ServerInfo {
  id: string;
  config: ServerConfig;
  status: 'connected' | 'disconnected' | 'connecting' | 'error';
  capabilities?: ServerCapabilities;
  process_info?: ProcessInfo;
  metrics: ConnectionMetrics;
  last_seen?: string;
}

export interface ServerCapabilities {
  tools?: { list_changed?: boolean };
  resources?: { subscribe?: boolean; list_changed?: boolean };
  prompts?: { list_changed?: boolean };
  sampling?: {};
  elicitation?: {};
}

export interface ProcessInfo {
  pid: number;
  command: string;
  args: string[];
  started_at: string;
  cpu_usage: number;
  memory_usage: number;
  status: 'running' | 'stopped' | 'crashed';
}

export interface ConnectionMetrics {
  connected_at?: string;
  requests_sent: number;
  responses_received: number;
  avg_response_time_ms: number;
  error_count: number;
  last_error?: string;
  bytes_sent: number;
  bytes_received: number;
  uptime_seconds: number;
}

export interface ToolDefinition {
  name: string;
  description?: string;
  input_schema: any;
}

export interface ToolExecution {
  id: string;
  serverId: string;
  serverName: string;
  tool: string;
  parameters: any;
  result: any;
  timestamp: string;
  duration?: number;
  status: 'success' | 'error';
  error?: string;
}

interface ServerStoreState {
  servers: ServerInfo[];
  selectedServerId?: string;
  templates: ServerConfig[];
  loading: boolean;
  initializing: boolean;
  error?: string;
  toolExecutions: ToolExecution[];
}

const initialState: ServerStoreState = {
  servers: [],
  selectedServerId: undefined,
  templates: [],
  loading: false,
  initializing: false,
  error: undefined,
  toolExecutions: [],
};

function createServerStore() {
  const { subscribe, set, update } = writable<ServerStoreState>(initialState);

  return {
    subscribe,

    // Load servers from backend
    async loadServers() {
      update(state => ({ ...state, loading: true, error: undefined }));
      try {
        const servers: ServerInfo[] = await invoke('list_servers');
        update(state => ({ ...state, servers, loading: false }));
      } catch (error) {
        console.error('Failed to load servers:', error);
        update(state => ({
          ...state,
          loading: false,
          error: `Failed to load servers: ${error}`
        }));
      }
    },

    // Load saved server configurations from database on startup
    async loadSavedConfigurations() {
      try {
        const savedConfigs: ServerConfig[] = await invoke('load_server_configs');

        // Convert saved configs to ServerInfo format with disconnected status
        const savedServers: ServerInfo[] = savedConfigs.map(config => ({
          id: config.id,
          config,
          status: 'disconnected' as const,
          metrics: {
            connected_at: undefined,
            requests_sent: 0,
            responses_received: 0,
            avg_response_time_ms: 0,
            error_count: 0,
            last_error: undefined,
            bytes_sent: 0,
            bytes_received: 0,
            uptime_seconds: 0,
          },
        }));

        update(state => {
          // Merge saved servers with existing ones, avoiding duplicates
          const existingServerIds = new Set(state.servers.map(s => s.id));
          const newServers = savedServers.filter(server => !existingServerIds.has(server.id));

          return {
            ...state,
            servers: [...state.servers, ...newServers]
          };
        });
      } catch (error) {
        // If database isn't ready yet, just log and continue
        // The app-ready event will trigger another attempt
        if (typeof error === 'string' && error.includes('Database not yet initialized')) {
        } else {
          console.error('Failed to load saved configurations:', error);
        }
      }
    },

    // Retry loading configurations (called when database becomes ready)
    async retryLoadConfigurations() {
      await this.loadSavedConfigurations();
    },

    // Load server templates
    async loadTemplates() {
      try {
        const templates: ServerConfig[] = await invoke('get_server_templates');
        update(state => ({ ...state, templates }));
      } catch (error) {
        console.error('Failed to load templates:', error);
        update(state => ({ 
          ...state, 
          error: `Failed to load templates: ${error}` 
        }));
      }
    },

    // Connect to a server
    async connectServer(config: ServerConfig) {
      update(state => ({ ...state, loading: true, error: undefined }));

      // Immediately set connecting status to prevent UI freezing
      update(state => ({
        ...state,
        servers: state.servers.map(s =>
          s.config.name === config.name ? { ...s, status: 'connecting' as const } : s
        )
      }));

      try {
        const serverInfo: ServerInfo = await invoke('connect_server', { serverConfig: config });
        update(state => ({
          ...state,
          servers: [...state.servers.filter(s => s.id !== serverInfo.id), serverInfo],
          loading: false,
          selectedServerId: serverInfo.id,
        }));
        return serverInfo;
      } catch (error) {
        console.error('Failed to connect server:', error);

        // Set error status on failure to prevent UI freezing
        // Find the server by config name and update its status to error
        update(state => ({
          ...state,
          servers: state.servers.map(s =>
            s.config.name === config.name ? { ...s, status: 'error' as const } : s
          ),
          loading: false,
          error: `Failed to connect: ${error}`
        }));
        throw error;
      }
    },

    // Disconnect from a server
    async disconnectServer(serverId: string) {
      try {
        await invoke('disconnect_server', { serverId });
        update(state => ({
          ...state,
          servers: state.servers.map(s =>
            s.id === serverId ? { ...s, status: 'disconnected' as const } : s
          ),
          selectedServerId: state.selectedServerId === serverId ? undefined : state.selectedServerId,
        }));
      } catch (error) {
        console.error('Failed to disconnect server:', error);
        update(state => ({ 
          ...state, 
          error: `Failed to disconnect: ${error}` 
        }));
        throw error;
      }
    },

    // Create a new server configuration
    async createServerConfig(
      name: string,
      description: string | undefined,
      transportConfig: TransportConfig,
      environmentVariables: Record<string, string>
    ) {
      try {
        const config: ServerConfig = await invoke('create_server_config', {
          request: {
            name,
            description,
            transport: transportConfig,
            environment_variables: environmentVariables,
          }
        });

        // Add the new configuration to the server list as disconnected
        const newServerInfo: ServerInfo = {
          id: config.id,
          config,
          status: 'disconnected',
          metrics: {
            connected_at: undefined,
            requests_sent: 0,
            responses_received: 0,
            avg_response_time_ms: 0,
            error_count: 0,
            last_error: undefined,
            bytes_sent: 0,
            bytes_received: 0,
            uptime_seconds: 0,
          },
        };

        update(state => ({
          ...state,
          servers: [...state.servers.filter(s => s.id !== config.id), newServerInfo]
        }));

        return config;
      } catch (error) {
        console.error('Failed to create server config:', error);
        update(state => ({
          ...state,
          error: `Failed to create configuration: ${error}`
        }));
        throw error;
      }
    },

    // Save an existing server configuration
    async saveServerConfig(config: ServerConfig) {
      try {
        await invoke('save_server_config', { config });

        // Update the server in the store
        update(state => ({
          ...state,
          servers: state.servers.map(s =>
            s.id === config.id ? { ...s, config } : s
          )
        }));
      } catch (error) {
        console.error('Failed to save server config:', error);
        update(state => ({
          ...state,
          error: `Failed to save configuration: ${error}`
        }));
        throw error;
      }
    },

    // Update an existing server configuration
    async updateServerConfig(
      id: string,
      name: string,
      description: string | undefined,
      transport: TransportConfig,
      environment_variables: Record<string, string>
    ) {
      try {
        const updatedConfig: ServerConfig = await invoke('update_server_config', {
          request: {
            id,
            name,
            description,
            transport,
            environment_variables
          }
        });

        // Update the server in the store with the returned config
        update(state => ({
          ...state,
          servers: state.servers.map(s =>
            s.id === id ? { ...s, config: updatedConfig } : s
          )
        }));

        return updatedConfig;
      } catch (error) {
        console.error('Failed to update server config:', error);
        update(state => ({
          ...state,
          error: `Failed to update configuration: ${error}`
        }));
        throw error;
      }
    },

    // Delete a server configuration
    async deleteServerConfig(serverId: string) {
      try {
        await invoke('delete_server_config', { serverId });

        // Remove from the store
        update(state => ({
          ...state,
          servers: state.servers.filter(s => s.id !== serverId),
          selectedServerId: state.selectedServerId === serverId ? undefined : state.selectedServerId
        }));
      } catch (error) {
        console.error('Failed to delete server config:', error);
        update(state => ({
          ...state,
          error: `Failed to delete configuration: ${error}`
        }));
        throw error;
      }
    },

    // Test a server configuration
    async testServerConfig(config: ServerConfig) {
      try {
        const request = {
          name: config.name,
          description: config.description,
          transport: config.transport_config,
          environment_variables: config.environment_variables,
        };
        const result: boolean = await invoke('test_server_config', { request });
        return result;
      } catch (error) {
        console.error('Failed to test server config:', error);
        throw error;
      }
    },

    // Call a tool on a server
    async callTool(serverId: string, toolName: string, parameters: any) {
      const startTime = Date.now();
      const executionId = crypto.randomUUID();

      // Find server for context - use centralized method
      const serverName = this.getServerName(serverId);

      try {
        const result = await invoke('call_tool', {
          serverId,
          toolName,
          parameters,
        });

        const execution: ToolExecution = {
          id: executionId,
          serverId,
          serverName,
          tool: toolName,
          parameters,
          result,
          timestamp: new Date().toISOString(),
          duration: Date.now() - startTime,
          status: 'success'
        };

        // Add to execution history
        update(state => ({
          ...state,
          toolExecutions: [execution, ...state.toolExecutions].slice(0, 100) // Keep last 100 executions
        }));

        return result;
      } catch (error) {
        const execution: ToolExecution = {
          id: executionId,
          serverId,
          serverName,
          tool: toolName,
          parameters,
          result: null,
          timestamp: new Date().toISOString(),
          duration: Date.now() - startTime,
          status: 'error',
          error: String(error)
        };

        // Add to execution history
        update(state => ({
          ...state,
          toolExecutions: [execution, ...state.toolExecutions].slice(0, 100) // Keep last 100 executions
        }));

        console.error('Failed to call tool:', error);
        throw error;
      }
    },

    // List tools for a server
    async listTools(serverId: string) {
      try {
        const tools: ToolDefinition[] = await invoke('list_tools', { serverId });
        return tools;
      } catch (error) {
        console.error('Failed to list tools:', error);
        throw error;
      }
    },

    async listPrompts(serverId: string) {
      try {
        const prompts: any[] = await invoke('list_prompts', { serverId });
        return prompts;
      } catch (error) {
        console.error('Failed to list prompts:', error);
        throw error;
      }
    },

    async listResources(serverId: string) {
      try {
        const resources: any[] = await invoke('list_resources', { serverId });
        return resources;
      } catch (error) {
        console.error('Failed to list resources:', error);
        throw error;
      }
    },

    // Get server name by ID (utility method)
    getServerName(serverId: string): string {
      let currentState: ServerStoreState;
      subscribe(state => currentState = state)(); // Get current state without subscribing

      const server = currentState!.servers.find(s => s.id === serverId);
      return server?.config.name || 'Unknown Server';
    },

    // Add prompt execution to history (unified with tool executions)
    addPromptExecution(
      serverId: string,
      promptName: string,
      parameters: any,
      result: any,
      duration: number,
      status: 'success' | 'error',
      error?: string
    ) {
      const execution: ToolExecution = {
        id: crypto.randomUUID(),
        serverId,
        serverName: this.getServerName(serverId),
        tool: `prompt:${promptName}`, // Prefix to distinguish from regular tools
        parameters,
        result,
        timestamp: new Date().toISOString(),
        duration,
        status,
        error
      };

      // Add to execution history (same as tool executions)
      update(state => ({
        ...state,
        toolExecutions: [execution, ...state.toolExecutions].slice(0, 100) // Keep last 100 executions
      }));

      return execution;
    },

    // Handle MCP events from backend
    handleMcpEvent(event: any) {

      // Handle Rust enum serialization - the variant name becomes the key
      if (event.StatusChanged) {
        // Validate status field to prevent runtime errors (handle case sensitivity)
        const statusValue = event.StatusChanged.status;
        const statusLower = typeof statusValue === 'string' ? statusValue.toLowerCase() : 'error';
        const validStatus: 'connected' | 'disconnected' | 'connecting' | 'error' =
          ['connected', 'disconnected', 'connecting', 'error'].includes(statusLower)
            ? statusLower as 'connected' | 'disconnected' | 'connecting' | 'error'
            : 'error';

        update(state => ({
          ...state,
          servers: state.servers.map(s =>
            s.id === event.StatusChanged.server_id ? { ...s, status: validStatus } : s
          ),
        }));
      } else if (event.CapabilitiesUpdated) {
        update(state => ({
          ...state,
          servers: state.servers.map(s =>
            s.id === event.CapabilitiesUpdated.server_id ? { ...s, capabilities: event.CapabilitiesUpdated.capabilities } : s
          ),
        }));
      } else if (event.MetricsUpdated) {
        update(state => ({
          ...state,
          servers: state.servers.map(s =>
            s.id === event.MetricsUpdated.server_id ? { ...s, metrics: event.MetricsUpdated.metrics } : s
          ),
        }));
      } else if (event.ProcessUpdated) {
        update(state => ({
          ...state,
          servers: state.servers.map(s =>
            s.id === event.ProcessUpdated.server_id ? { ...s, process_info: event.ProcessUpdated.process_info } : s
          ),
        }));
      } else if (event.Error) {
        console.error('MCP Connection Error:', event.Error);
        console.error('MCP Error details:', JSON.stringify(event.Error, null, 2));
        update(state => ({
          ...state,
          servers: state.servers.map(s =>
            s.id === event.Error.server_id ? { ...s, status: 'error' as const } : s
          ),
          error: `MCP Error: ${event.Error.error}`
        }));
      } else {
      }
    },

    // Select a server
    selectServer(serverId: string | undefined) {
      update(state => ({ ...state, selectedServerId: serverId }));
    },

    // Clear error
    clearError() {
      update(state => ({ ...state, error: undefined }));
    },

    // Get selected server
    getSelectedServer(state: ServerStoreState) {
      return state.servers.find(s => s.id === state.selectedServerId);
    },

    // Get tool execution history (non-reactive read-only access)
    getToolExecutions(serverId?: string) {
      let executions: ToolExecution[] = [];
      subscribe(state => {
        executions = serverId
          ? state.toolExecutions.filter(e => e.serverId === serverId)
          : state.toolExecutions;
      })(); // Immediately unsubscribe
      return executions;
    },

    // Clear execution history
    clearExecutionHistory(serverId?: string) {
      update(state => ({
        ...state,
        toolExecutions: serverId
          ? state.toolExecutions.filter(e => e.serverId !== serverId)
          : []
      }));
    },

    // Initialize the store by loading servers and templates
    async initialize() {

      // Check if already initializing to prevent race conditions
      let currentState: ServerStoreState;
      subscribe(state => currentState = state)(); // Get current state without subscribing

      if (currentState!.initializing) {
        return; // Already initializing, skip
      }

      update(state => ({ ...state, initializing: true }));

      try {
        // Load all data in parallel for better performance
        await Promise.allSettled([
          this.loadServers(), // Now returns ALL servers (connected and disconnected)
          this.loadTemplates()
        ]);
      } catch (error) {
        console.error('❌ Server store initialization failed:', error);
        throw error;
      } finally {
        update(state => ({ ...state, initializing: false }));
      }
    },
  };
}

export const serverStore = createServerStore();

// Utility function to safely get server status for UI display
export function getServerStatus(server: ServerInfo | undefined): string {
  if (!server?.status) return 'disconnected';
  return ['connected', 'disconnected', 'connecting', 'error'].includes(server.status)
    ? server.status
    : 'error';
}