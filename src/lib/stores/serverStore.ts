import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { withTimeout, globalRequestManager } from '$lib/utils/asyncHelpers';
import { createLogger } from '$lib/utils/logger';

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

/**
 * ✅ NEW: Server store state with Map-based storage for O(1) operations
 */
export interface ServerStoreState {
  // ✅ NEW: Map instead of array for O(1) lookups
  servers: Map<string, ServerInfo>;
  selectedServerId?: string;
  templates: ServerConfig[];
  loading: boolean;
  initializing: boolean;
  error?: string;
  toolExecutions: ToolExecution[];
}

const initialState: ServerStoreState = {
  // ✅ NEW: Initialize with Map
  servers: new Map(),
  selectedServerId: undefined,
  templates: [],
  loading: false,
  initializing: false,
  error: undefined,
  toolExecutions: [],
};

function createServerStore() {
  const { subscribe, set, update } = writable<ServerStoreState>(initialState);
  const logger = createLogger('ServerStore');

  // ✅ FIXED: Helper to convert Map to array for UI compatibility (no memory leak)
  const getServersArray = (): ServerInfo[] => {
    const state = get({ subscribe });
    return Array.from(state.servers.values());
  };

  // ✅ FIXED: Helper to get a single server by ID (no memory leak)
  const getServer = (serverId: string): ServerInfo | undefined => {
    const state = get({ subscribe });
    return state.servers.get(serverId);
  };

  return {
    subscribe,

    // Export helper for UI
    getServersArray,
    getServer,

    // Load servers from backend
    async loadServers() {
      update(state => ({ ...state, loading: true, error: undefined }));
      try {
        const serversArray: ServerInfo[] = await withTimeout(
          invoke('list_servers'),
          10000,
          'Failed to load servers: timeout'
        );

        // ✅ NEW: Convert array to Map
        const serversMap = new Map<string, ServerInfo>();
        serversArray.forEach(server => {
          serversMap.set(server.id, server);
        });

        update(state => ({ ...state, servers: serversMap, loading: false }));
      } catch (error) {
        logger.error('Failed to load servers:', error);
        update(state => ({
          ...state,
          loading: false,
          error: `Failed to load servers: ${error}`
        }));
      }
    },

    // ✅ FIXED: Load saved server configurations from database (with Map)
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

        // ✅ NEW: Merge into Map, avoiding duplicates
        update(state => {
          const newServers = new Map(state.servers);

          // Add saved servers that don't already exist
          savedServers.forEach(server => {
            if (!newServers.has(server.id)) {
              newServers.set(server.id, server);
            }
          });

          return { ...state, servers: newServers };
        });
      } catch (error) {
        // If database isn't ready yet, just log and continue
        if (typeof error === 'string' && error.includes('Database not yet initialized')) {
          // Silent - expected during initialization
        } else {
          logger.error('❌ Failed to load saved configurations:', error);
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
        logger.error('Failed to load templates:', error);
        update(state => ({ 
          ...state, 
          error: `Failed to load templates: ${error}` 
        }));
      }
    },

    // ✅ FIXED: Connect to a server (handles both new server creation and reconnection)
    async connectServer(config: ServerConfig) {
      // ✅ NEW: Use request manager to prevent duplicate connections
      return globalRequestManager.execute(
        `connect-server-${config.name}`,
        async () => {
          update(state => ({ ...state, loading: true, error: undefined }));

          try {
            // Check if server already exists in database by name
            let serverConfig: ServerConfig;
            const existingServer = getServer(config.id);

            if (existingServer) {
              // ✅ Server exists, reconnect to it
              console.log(`♻️ Reconnecting to existing server: ${config.name}`);
              serverConfig = existingServer.config;

              // Update status to connecting
              update(state => {
                const newServers = new Map(state.servers);
                newServers.set(existingServer.id, {
                  ...existingServer,
                  status: 'connecting' as const
                });
                return { ...state, servers: newServers };
              });
            } else {
              // ✅ NEW: New server - create config in database first
              console.log(`✨ Creating new server: ${config.name}`);

              serverConfig = await withTimeout(
                invoke('create_server_config', {
                  request: {
                    name: config.name,
                    description: config.description,
                    transport: config.transport_config,
                    environment_variables: config.environment_variables,
                  }
                }),
                30000,
                'Server creation timed out'
              );

              // Add to store as connecting (before actual connection)
              const newServerInfo: ServerInfo = {
                id: serverConfig.id,
                config: serverConfig,
                status: 'connecting' as const,
                metrics: {
                  requests_sent: 0,
                  responses_received: 0,
                  avg_response_time_ms: 0,
                  error_count: 0,
                  bytes_sent: 0,
                  bytes_received: 0,
                  uptime_seconds: 0,
                },
              };

              update(state => {
                const newServers = new Map(state.servers);
                newServers.set(serverConfig.id, newServerInfo);
                return { ...state, servers: newServers };
              });
            }

            // ✅ Now connect to the server (both new and existing go through here)
            console.log(`🔌 Connecting to server: ${serverConfig.name}`);
            const serverInfo: ServerInfo = await withTimeout(
              invoke('connect_server', { serverConfig }),
              30000,
              'Server connection timed out'
            );

            // ✅ Single update - replace the server with connected info
            update(state => {
              const newServers = new Map(state.servers);
              newServers.set(serverInfo.id, serverInfo);
              return {
                ...state,
                servers: newServers,
                loading: false,
                selectedServerId: serverInfo.id,
              };
            });

            console.log(`✅ Successfully connected to: ${serverConfig.name}`);
            return serverInfo;
          } catch (error) {
            logger.error('❌ Failed to connect server:', error);

            // ✅ Update the server to error status (by ID if possible, name as fallback)
            update(state => {
              const newServers = new Map(state.servers);
              // Try to find by ID first
              const serverId = config.id || Array.from(state.servers.values()).find(s => s.config.name === config.name)?.id;

              if (serverId) {
                const server = newServers.get(serverId);
                if (server) {
                  newServers.set(serverId, {
                    ...server,
                    status: 'error' as const
                  });
                }
              }

              return {
                ...state,
                servers: newServers,
                loading: false,
                error: `Failed to connect: ${error}`
              };
            });

            throw error;
          }
        },
        30000 // 30 second timeout for entire operation
      );
    },

    // ✅ FIXED: Disconnect from a server with timeout
    async disconnectServer(serverId: string) {
      try {
        await withTimeout(
          invoke('disconnect_server', { serverId }),
          15000,
          'Disconnect timed out'
        );

        // ✅ NEW: Update Map
        update(state => {
          const newServers = new Map(state.servers);
          const server = newServers.get(serverId);
          if (server) {
            newServers.set(serverId, {
              ...server,
              status: 'disconnected' as const
            });
          }
          return {
            ...state,
            servers: newServers,
            selectedServerId: state.selectedServerId === serverId ? undefined : state.selectedServerId,
          };
        });
      } catch (error) {
        logger.error('❌ Failed to disconnect server:', error);
        update(state => ({
          ...state,
          error: `Failed to disconnect: ${error}`
        }));
        throw error;
      }
    },

    // ✅ FIXED: Create a new server configuration (now with Map and timeout)
    async createServerConfig(
      name: string,
      description: string | undefined,
      transportConfig: TransportConfig,
      environmentVariables: Record<string, string>
    ) {
      try {
        const config: ServerConfig = await withTimeout(
          invoke('create_server_config', {
            request: {
              name,
              description,
              transport: transportConfig,
              environment_variables: environmentVariables,
            }
          }),
          30000,
          'Server creation timed out'
        );

        // ✅ NEW: Add to Map as disconnected
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

        update(state => {
          const newServers = new Map(state.servers);
          newServers.set(config.id, newServerInfo);
          return { ...state, servers: newServers };
        });

        console.log(`✅ Created server config: ${name}`);
        return config;
      } catch (error) {
        logger.error('❌ Failed to create server config:', error);
        update(state => ({
          ...state,
          error: `Failed to create configuration: ${error}`
        }));
        throw error;
      }
    },

    // ✅ FIXED: Save an existing server configuration (with Map and timeout)
    async saveServerConfig(config: ServerConfig) {
      try {
        await withTimeout(
          invoke('save_server_config', { config }),
          30000,
          'Save operation timed out'
        );

        // ✅ NEW: Update in Map
        update(state => {
          const newServers = new Map(state.servers);
          const server = newServers.get(config.id);
          if (server) {
            newServers.set(config.id, { ...server, config });
          }
          return { ...state, servers: newServers };
        });
      } catch (error) {
        logger.error('❌ Failed to save server config:', error);
        update(state => ({
          ...state,
          error: `Failed to save configuration: ${error}`
        }));
        throw error;
      }
    },

    // ✅ FIXED: Update an existing server configuration (with Map and timeout)
    async updateServerConfig(
      id: string,
      name: string,
      description: string | undefined,
      transport: TransportConfig,
      environment_variables: Record<string, string>
    ) {
      try {
        const updatedConfig: ServerConfig = await withTimeout(
          invoke('update_server_config', {
            request: {
              id,
              name,
              description,
              transport,
              environment_variables
            }
          }),
          30000,
          'Update operation timed out'
        );

        // ✅ NEW: Update in Map
        update(state => {
          const newServers = new Map(state.servers);
          const server = newServers.get(id);
          if (server) {
            newServers.set(id, { ...server, config: updatedConfig });
          }
          return { ...state, servers: newServers };
        });

        console.log(`✅ Updated server config: ${name}`);
        return updatedConfig;
      } catch (error) {
        logger.error('❌ Failed to update server config:', error);
        update(state => ({
          ...state,
          error: `Failed to update configuration: ${error}`
        }));
        throw error;
      }
    },

    // ✅ FIXED: Delete a server configuration with timeout
    async deleteServerConfig(serverId: string) {
      try {
        logger.debug(`🗑️ deleteServerConfig called for serverId: ${serverId}`);

        // Call Tauri backend to delete from database
        logger.debug('📡 Invoking Tauri command: delete_server_config');
        await withTimeout(
          invoke('delete_server_config', { serverId }),
          30000,
          'Delete operation timed out'
        );
        logger.debug('✅ Tauri command completed successfully');

        // ✅ NEW: Remove from Map
        logger.debug('🗺️ Removing server from local store Map...');
        update(state => {
          const newServers = new Map(state.servers);
          const existed = newServers.has(serverId);
          newServers.delete(serverId);
          logger.debug(`Server ${existed ? 'existed and was' : 'did not exist, not'} removed from Map`);
          logger.debug(`Map size: ${state.servers.size} → ${newServers.size}`);
          return {
            ...state,
            servers: newServers,
            selectedServerId: state.selectedServerId === serverId ? undefined : state.selectedServerId
          };
        });

        logger.info(`✅ Successfully deleted server: ${serverId}`);
      } catch (error) {
        logger.error('❌ Failed to delete server config:', error);
        logger.error('Error type:', typeof error);
        logger.error('Error details:', JSON.stringify(error, null, 2));
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
        logger.error('Failed to test server config:', error);
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

        logger.error('Failed to call tool:', error);
        throw error;
      }
    },

    // List tools for a server
    async listTools(serverId: string) {
      try {
        const tools: ToolDefinition[] = await invoke('list_tools', { serverId });
        return tools;
      } catch (error) {
        logger.error('Failed to list tools:', error);
        throw error;
      }
    },

    async listPrompts(serverId: string) {
      try {
        const prompts: any[] = await invoke('list_prompts', { serverId });
        return prompts;
      } catch (error) {
        logger.error('Failed to list prompts:', error);
        throw error;
      }
    },

    async listResources(serverId: string) {
      try {
        const resources: any[] = await invoke('list_resources', { serverId });
        return resources;
      } catch (error) {
        logger.error('Failed to list resources:', error);
        throw error;
      }
    },

    // Get server name by ID (utility method)
    getServerName(serverId: string): string {
      // ✅ FIXED: Use get() helper to avoid memory leak
      const state = get({ subscribe });
      const server = state.servers.get(serverId);
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

    // ✅ FIXED: Handle MCP events from backend (with Map)
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

        update(state => {
          const newServers = new Map(state.servers);
          const server = newServers.get(event.StatusChanged.server_id);
          if (server) {
            newServers.set(event.StatusChanged.server_id, { ...server, status: validStatus });
          }
          return { ...state, servers: newServers };
        });
      } else if (event.CapabilitiesUpdated) {
        update(state => {
          const newServers = new Map(state.servers);
          const server = newServers.get(event.CapabilitiesUpdated.server_id);
          if (server) {
            newServers.set(event.CapabilitiesUpdated.server_id, {
              ...server,
              capabilities: event.CapabilitiesUpdated.capabilities
            });
          }
          return { ...state, servers: newServers };
        });
      } else if (event.MetricsUpdated) {
        update(state => {
          const newServers = new Map(state.servers);
          const server = newServers.get(event.MetricsUpdated.server_id);
          if (server) {
            newServers.set(event.MetricsUpdated.server_id, {
              ...server,
              metrics: event.MetricsUpdated.metrics
            });
          }
          return { ...state, servers: newServers };
        });
      } else if (event.ProcessUpdated) {
        update(state => {
          const newServers = new Map(state.servers);
          const server = newServers.get(event.ProcessUpdated.server_id);
          if (server) {
            newServers.set(event.ProcessUpdated.server_id, {
              ...server,
              process_info: event.ProcessUpdated.process_info
            });
          }
          return { ...state, servers: newServers };
        });
      } else if (event.Error) {
        logger.error('❌ MCP Connection Error:', event.Error);
        logger.error('MCP Error details:', JSON.stringify(event.Error, null, 2));
        update(state => {
          const newServers = new Map(state.servers);
          const server = newServers.get(event.Error.server_id);
          if (server) {
            newServers.set(event.Error.server_id, { ...server, status: 'error' as const });
          }
          return {
            ...state,
            servers: newServers,
            error: `MCP Error: ${event.Error.error}`
          };
        });
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

    // ✅ FIXED: Get selected server (with Map)
    getSelectedServer(state: ServerStoreState) {
      return state.selectedServerId ? state.servers.get(state.selectedServerId) : undefined;
    },

    // Get tool execution history (non-reactive read-only access)
    getToolExecutions(serverId?: string) {
      // ✅ FIXED: Use get() helper to avoid memory leak
      const state = get({ subscribe });
      return serverId
        ? state.toolExecutions.filter(e => e.serverId === serverId)
        : state.toolExecutions;
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

      // ✅ FIXED: Check if already initializing to prevent race conditions (no memory leak)
      const currentState = get({ subscribe });

      if (currentState.initializing) {
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
        logger.error('❌ Server store initialization failed:', error);
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