/**
 * Server Context Store
 *
 * Centralized state management for server selection and operational context.
 * This store provides the "WHAT am I working with?" context for all operational tabs.
 *
 * Design Philosophy:
 * - Single source of truth for server selection across all tabs
 * - Automatic synchronization with serverStore
 * - Smart defaults and connection state handling
 * - Profile-aware server grouping
 */

import { writable, derived, get } from 'svelte/store';
import { serverStore, type ServerInfo } from './serverStore';
import { profileStore } from './profileStore';
import { createLogger } from '$lib/utils/logger';

const logger = createLogger('ContextStore');

export interface ServerContext {
  // Currently selected server for operations
  selectedServerId: string | null;

  // Full server info for the selected server
  selectedServer: ServerInfo | null;

  // All available servers (connected + capable for current view)
  availableServers: ServerInfo[];

  // Active profile context (if any)
  activeProfile: {
    id: string;
    name: string;
    icon?: string;
  } | null;

  // Connection status of selected server
  connectionStatus: 'connected' | 'connecting' | 'error' | 'disconnected';

  // Whether the selected server is from the active profile
  isFromActiveProfile: boolean;

  // Last updated timestamp
  lastUpdated: number;
}

interface ContextStoreState {
  // Persisted server selection (survives tab switches)
  selectedServerId: string | null;

  // User preference: remember last selection
  rememberSelection: boolean;
}

const initialState: ContextStoreState = {
  selectedServerId: null,
  rememberSelection: true,
};

function createContextStore() {
  const { subscribe, set, update } = writable<ContextStoreState>(initialState);

  /**
   * Derived context that computes full server context from current state
   */
  const context = derived(
    [{ subscribe }, serverStore, profileStore],
    ([$state, $servers, $profiles]) => {
      const servers = $servers.servers instanceof Map
        ? Array.from($servers.servers.values())
        : [];

      const connectedServers = servers.filter(s => s.status === 'connected');

      // Find selected server
      const selectedServer = $state.selectedServerId
        ? connectedServers.find(s => s.id === $state.selectedServerId) || null
        : null;

      // Get active profile context
      const activeProfile = $profiles.activeProfile?.profile
        ? {
            id: $profiles.activeProfile.profile.id,
            name: $profiles.activeProfile.profile.name,
            icon: $profiles.activeProfile.profile.icon,
          }
        : null;

      // Check if selected server is from active profile
      const isFromActiveProfile = !!(
        selectedServer &&
        activeProfile &&
        $profiles.activeProfile?.servers?.some(ps => ps.server_id === selectedServer.id)
      );

      // Determine connection status
      const connectionStatus = selectedServer?.status === 'connected' ? 'connected' :
                              selectedServer?.status === 'connecting' ? 'connecting' :
                              selectedServer?.status === 'error' ? 'error' :
                              'disconnected';

      return {
        selectedServerId: $state.selectedServerId,
        selectedServer,
        availableServers: connectedServers,
        activeProfile,
        connectionStatus,
        isFromActiveProfile,
        lastUpdated: Date.now(),
      } as ServerContext;
    }
  );

  return {
    subscribe: context.subscribe,

    /**
     * Select a server for operations
     */
    selectServer(serverId: string | null) {
      logger.debug('Selecting server:', serverId);

      update(state => ({
        ...state,
        selectedServerId: serverId,
      }));

      // Also update global serverStore for backward compatibility
      if (serverId) {
        serverStore.selectServer(serverId);
      }
    },

    /**
     * Clear server selection
     */
    clearSelection() {
      logger.debug('Clearing server selection');
      update(state => ({
        ...state,
        selectedServerId: null,
      }));
    },

    /**
     * Auto-select a server based on smart defaults
     * Priority:
     * 1. Current selection (if still valid)
     * 2. First server from active profile
     * 3. First connected server
     *
     * @param mode - 'selector' (default) auto-selects a server, 'filter' respects null selection
     */
    autoSelectServer(mode: 'selector' | 'filter' = 'selector') {
      const currentContext = get(context);

      // In filter mode, respect user's decision to view all servers (don't force selection)
      if (mode === 'filter') {
        // If current selection is valid, keep it (user explicitly chose to filter)
        if (currentContext.selectedServer && currentContext.connectionStatus === 'connected') {
          logger.debug('[Filter Mode] Keeping current selection:', currentContext.selectedServer.config.name);
          return;
        }

        // Otherwise, clear selection to show all servers
        logger.debug('[Filter Mode] Allowing null selection to show all servers');
        this.clearSelection();
        return;
      }

      // Selector mode: Auto-select a server (required for operational tabs)

      // If current selection is valid, keep it
      if (currentContext.selectedServer && currentContext.connectionStatus === 'connected') {
        logger.debug('[Selector Mode] Keeping current selection:', currentContext.selectedServer.config.name);
        return;
      }

      // Try to select first server from active profile
      if (currentContext.activeProfile) {
        const profileStore$ = get(profileStore);
        const profileServers = profileStore$.activeProfile?.servers || [];

        if (profileServers.length > 0) {
          const firstProfileServerId = profileServers[0].server_id;
          const firstProfileServer = currentContext.availableServers.find(s => s.id === firstProfileServerId);

          if (firstProfileServer) {
            logger.debug('[Selector Mode] Auto-selecting first server from profile:', firstProfileServer.config.name);
            this.selectServer(firstProfileServer.id);
            return;
          }
        }
      }

      // Fall back to first connected server
      if (currentContext.availableServers.length > 0) {
        const firstServer = currentContext.availableServers[0];
        logger.debug('[Selector Mode] Auto-selecting first connected server:', firstServer.config.name);
        this.selectServer(firstServer.id);
        return;
      }

      // No servers available
      logger.debug('[Selector Mode] No servers available for auto-selection');
      this.clearSelection();
    },

    /**
     * Handle server disconnection
     */
    handleServerDisconnected(serverId: string) {
      const currentContext = get(context);

      if (currentContext.selectedServerId === serverId) {
        logger.warn('Selected server disconnected, auto-selecting another');
        this.autoSelectServer();
      }
    },

    /**
     * Toggle remember selection preference
     */
    toggleRememberSelection() {
      update(state => ({
        ...state,
        rememberSelection: !state.rememberSelection,
      }));
    },

    /**
     * Get current context (non-reactive)
     */
    getCurrentContext(): ServerContext {
      return get(context);
    },
  };
}

export const contextStore = createContextStore();

// Export helper to check if server is capable for a specific operation
export function serverHasCapability(server: ServerInfo | null, capability: string): boolean {
  if (!server?.capabilities) return false;

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
