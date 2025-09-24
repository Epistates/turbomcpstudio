/**
 * Server Capability Utilities
 * Centralized logic for checking MCP server capabilities and filtering
 */

import type { ServerInfo } from '$lib/stores/serverStore';

export type McpCapability = 'tools' | 'resources' | 'prompts' | 'sampling' | 'elicitation';

/**
 * Check if a server supports a specific MCP capability
 */
export function serverSupportsCapability(server: ServerInfo, capability: McpCapability): boolean {
  if (!server.capabilities) return false;

  switch (capability) {
    case 'tools':
      return !!server.capabilities.tools;
    case 'resources':
      return !!server.capabilities.resources;
    case 'prompts':
      return !!server.capabilities.prompts;
    case 'sampling':
      return !!server.capabilities.sampling;
    case 'elicitation':
      return !!server.capabilities.elicitation;
    default:
      return false;
  }
}

/**
 * Filter servers by connection status and capability support
 */
export function filterServersByCapability(
  servers: ServerInfo[],
  capability: McpCapability,
  requireConnected = true
): ServerInfo[] {
  return servers.filter(server => {
    // Check connection status if required
    if (requireConnected && server.status?.toLowerCase() !== 'connected') {
      return false;
    }

    // Check capability support
    return serverSupportsCapability(server, capability);
  });
}

/**
 * Get all connected servers (regardless of capabilities)
 */
export function getConnectedServers(servers: ServerInfo[]): ServerInfo[] {
  return servers.filter(server => server.status?.toLowerCase() === 'connected');
}

/**
 * Get servers that support multiple capabilities
 */
export function filterServersByMultipleCapabilities(
  servers: ServerInfo[],
  capabilities: McpCapability[],
  requireConnected = true
): ServerInfo[] {
  return servers.filter(server => {
    // Check connection status if required
    if (requireConnected && server.status?.toLowerCase() !== 'connected') {
      return false;
    }

    // Check if server supports ALL specified capabilities
    return capabilities.every(capability => serverSupportsCapability(server, capability));
  });
}

/**
 * Check if any servers support a given capability
 */
export function hasServersWithCapability(
  servers: ServerInfo[],
  capability: McpCapability,
  requireConnected = true
): boolean {
  return filterServersByCapability(servers, capability, requireConnected).length > 0;
}

/**
 * Get capability support summary for a server
 */
export function getServerCapabilitySummary(server: ServerInfo): {
  tools: boolean;
  resources: boolean;
  prompts: boolean;
  sampling: boolean;
  elicitation: boolean;
  total: number;
} {
  const capabilities = {
    tools: serverSupportsCapability(server, 'tools'),
    resources: serverSupportsCapability(server, 'resources'),
    prompts: serverSupportsCapability(server, 'prompts'),
    sampling: serverSupportsCapability(server, 'sampling'),
    elicitation: serverSupportsCapability(server, 'elicitation'),
  };

  return {
    ...capabilities,
    total: Object.values(capabilities).filter(Boolean).length
  };
}