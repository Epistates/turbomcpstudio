/**
 * Optimized Server Store Utilities
 * Custom stores and reactive patterns for efficient server management
 */

import { derived } from 'svelte/store';
import { serverStore } from '$lib/stores/serverStore';
import { filterServersByCapability, getConnectedServers, type McpCapability } from './serverCapabilities';
import type { ServerInfo } from '$lib/stores/serverStore';

/**
 * Reactive store for servers filtered by capability
 * More efficient than filtering in each component
 */
export function createCapabilityStore(capability: McpCapability) {
  return derived(serverStore, ($serverStore) => {
    const allServers = $serverStore.servers;
    const connectedServers = getConnectedServers(allServers);
    const capableServers = filterServersByCapability(allServers, capability);

    return {
      servers: capableServers,
      totalServers: allServers.length,
      connectedServers: connectedServers.length,
      capableServers: capableServers.length,
      hasCapableServers: capableServers.length > 0,
      selectedServerId: $serverStore.selectedServerId
    };
  });
}

/**
 * Reactive store for all connected servers (no capability filtering)
 */
export const connectedServersStore = derived(serverStore, ($serverStore) => {
  const allServers = $serverStore.servers;
  const connectedServers = getConnectedServers(allServers);

  return {
    servers: connectedServers,
    totalServers: allServers.length,
    connectedServers: connectedServers.length,
    hasConnectedServers: connectedServers.length > 0,
    selectedServerId: $serverStore.selectedServerId
  };
});

/**
 * Reactive store for server statistics across all capabilities
 */
export const serverStatsStore = derived(serverStore, ($serverStore) => {
  const allServers = $serverStore.servers;
  const connectedServers = getConnectedServers(allServers);

  const capabilityStats = {
    tools: filterServersByCapability(allServers, 'tools').length,
    resources: filterServersByCapability(allServers, 'resources').length,
    prompts: filterServersByCapability(allServers, 'prompts').length,
    sampling: filterServersByCapability(allServers, 'sampling').length,
    elicitation: filterServersByCapability(allServers, 'elicitation').length,
  };

  return {
    total: allServers.length,
    connected: connectedServers.length,
    disconnected: allServers.length - connectedServers.length,
    capabilities: capabilityStats,
    selectedServerId: $serverStore.selectedServerId
  };
});

/**
 * Custom Svelte 5 rune for server capability management
 * Replaces manual store subscriptions with optimized reactivity
 */
export function useServerCapability(capability: McpCapability) {
  const store = createCapabilityStore(capability);

  // Return a reactive object that automatically updates
  return {
    get servers() { return store; },
    get value() {
      let value: any;
      const unsubscribe = store.subscribe(v => value = v);
      unsubscribe(); // Immediately cleanup
      return value;
    }
  };
}

/**
 * Helper for managing server selection state
 * Provides consistent server selection logic across components
 */
export class ServerSelectionManager {
  private capability: McpCapability;
  private capabilityStore: any;

  constructor(capability: McpCapability) {
    this.capability = capability;
    this.capabilityStore = createCapabilityStore(capability);
  }

  /**
   * Select a server and validate it supports the capability
   */
  selectServer(serverId: string | undefined): boolean {
    if (!serverId) {
      serverStore.selectServer(undefined);
      return true;
    }

    // Check if server exists and supports capability
    let isValid = false;
    const unsubscribe = this.capabilityStore.subscribe(state => {
      isValid = state.servers.some((s: ServerInfo) => s.id === serverId);
    });
    unsubscribe(); // Immediately cleanup

    if (isValid) {
      serverStore.selectServer(serverId);
      return true;
    }

    return false;
  }

  /**
   * Auto-select first available server that supports capability
   */
  autoSelectServer(): string | undefined {
    let selectedId: string | undefined;

    const unsubscribe = this.capabilityStore.subscribe(state => {
      if (state.servers.length > 0) {
        selectedId = state.servers[0].id;
        serverStore.selectServer(selectedId);
      }
    });
    unsubscribe(); // Immediately cleanup

    return selectedId;
  }

  /**
   * Get current selection state
   */
  getSelectionState() {
    let state: any;
    const unsubscribe = this.capabilityStore.subscribe(s => state = s);
    unsubscribe(); // Immediately cleanup

    const currentlySelected = state.servers.find((s: ServerInfo) => s.id === state.selectedServerId);

    return {
      selectedServer: currentlySelected,
      isValidSelection: !!currentlySelected,
      availableServers: state.servers,
      hasOptions: state.servers.length > 0
    };
  }
}