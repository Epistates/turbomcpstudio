<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { profileStore } from '$lib/stores/profileStore';
  import { serverStore, getServerStatus, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import { withTimeout } from '$lib/utils/asyncHelpers';
  import { createLogger } from '$lib/utils/logger';
  import { TIMEOUTS } from '$lib/constants/timeouts';
  import Button from './ui/Button.svelte';
  import ProfileEditor from './ProfileEditor.svelte';
  import InstallClientModal from './InstallClientModal.svelte';
  import RegistryBrowser from './RegistryBrowser.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import {
    Plus,
    Play,
    Square,
    Edit,
    Trash2,
    CheckCircle,
    Server as ServerIcon,
    Settings,
    Zap,
    AlertCircle,
    ArrowRight,
    X,
    Copy,
    LayoutGrid,
    List,
    Download,
    Upload,
    Package,
  } from 'lucide-svelte';

  // Initialize scoped logger
  const logger = createLogger('ServerManagement');

  // Reactive store subscriptions
  const profileState = $derived($profileStore);
  const serverState = $derived($serverStore);
  const uiState = $derived($uiStore);

  const activeProfile = $derived(profileState.activeProfile);
  const profiles = $derived(profileState.profiles);
  const loading = $derived(profileState.loading);
  // ✅ FIXED: Convert Map to array for UI compatibility
  const servers = $derived(
    serverState.servers instanceof Map
      ? Array.from(serverState.servers.values())
      : []
  );

  const showProfileEditor = $derived(uiState.modals.profileEditor.open);
  const editingProfile = $derived(uiState.editingProfileId);

  // Selection state for highlighting
  let selectedProfileId = $state<string | null>(null);
  // Dropdown state for Manage Profiles
  let showProfileDropdown = $state<string | null>(null);
  let profileSearchQuery = $state<string>('');
  let togglingProfile = $state<string | null>(null); // Track which profile is being toggled

  // View mode: 'card' or 'list' (compact)
  let viewMode = $state<'card' | 'list'>('card');

  // ✅ NEW: Track delete operations to prevent concurrent deletes
  let deletingServers = $state<Set<string>>(new Set());

  // Copy animation state for export all button
  let exportAllCopied = $state(false);

  function resetExportAllCopyState() {
    exportAllCopied = false;
  }

  // Copy animation state for export profile button (keyed by profileId)
  let exportProfileCopied = $state<Map<string, boolean>>(new Map());

  function setProfileCopied(profileId: string) {
    exportProfileCopied = new Map(exportProfileCopied);
    exportProfileCopied.set(profileId, true);
    setTimeout(() => {
      exportProfileCopied = new Map(exportProfileCopied);
      exportProfileCopied.delete(profileId);
    }, 2000);
  }

  // Copy animation state for individual server copy buttons (keyed by serverId)
  let serverCopied = $state<Map<string, boolean>>(new Map());

  function setServerCopied(serverId: string) {
    serverCopied = new Map(serverCopied);
    serverCopied.set(serverId, true);
    setTimeout(() => {
      serverCopied = new Map(serverCopied);
      serverCopied.delete(serverId);
    }, 2000);
  }

  // Install client modal state
  let showInstallModal = $state(false);

  // Registry browser modal state
  let showRegistryBrowser = $state(false);

  function openInstallModal() {
    showInstallModal = true;
  }

  function closeInstallModal() {
    showInstallModal = false;
  }

  onMount(async () => {
    await Promise.all([
      profileStore.loadProfiles(),
      profileStore.loadActiveProfile(),
      serverStore.loadServers(),
      loadAllProfileServerRelationships(), // Load ALL relationships on mount
    ]);
  });

  // Compute server-to-profile mapping
  // TODO: This needs backend support to load full profile with server details
  const serverProfileMap = $derived(() => {
    const map = new Map<string, string | null>();

    // Initialize all servers as unassigned
    servers.forEach(server => {
      map.set(server.id, null);
    });

    // If we have active profile, map its servers
    if (activeProfile?.servers) {
      activeProfile.servers.forEach(ps => {
        map.set(ps.server_id, activeProfile.profile?.id || null);
      });
    }

    return map;
  });

  // Get ALL profiles for a server (for multi-profile display)
  function getServerProfiles(serverId: string) {
    const serverProfiles = [];
    for (const [profileId, serverIds] of localProfileServerMap.entries()) {
      if (serverIds.has(serverId)) {
        const profile = profiles.find(p => p.id === profileId);
        if (profile) {
          serverProfiles.push(profile);
        }
      }
    }
    return serverProfiles;
  }

  // Get profile for a server (returns first profile, for backward compatibility)
  function getServerProfile(serverId: string) {
    const serverProfiles = getServerProfiles(serverId);
    return serverProfiles.length > 0 ? serverProfiles[0] : null;
  }

  // Get servers not in ANY profile (checks ALL profiles)
  const unassignedServers = $derived(() => {
    return servers.filter(server => {
      // Check if server is in ANY profile in localProfileServerMap
      for (const serverIds of localProfileServerMap.values()) {
        if (serverIds.has(server.id)) {
          return false; // Server is in at least one profile
        }
      }
      return true; // Server not in any profile
    });
  });

  // Check if server should be highlighted (checks ALL profiles)
  function isServerHighlighted(serverId: string): boolean {
    if (!selectedProfileId) return false;
    // Check localProfileServerMap for the selected profile
    const serverIds = localProfileServerMap.get(selectedProfileId);
    return serverIds ? serverIds.has(serverId) : false;
  }

  // Profile actions
  async function handleActivateProfile(profileId: string) {
    const success = await profileStore.activateProfile(profileId);
    if (success) {
      await profileStore.loadProfiles();
    }
  }

  async function handleDeactivateProfile() {
    const success = await profileStore.deactivateProfile();
    if (success) {
      await profileStore.loadProfiles();
    }
  }

  function handleEditProfile(profileId: string) {
    uiStore.openProfileEditor(profileId);
  }

  function handleCreateProfile() {
    uiStore.openProfileEditor(null);
  }

  async function handleDeleteProfile(profileId: string, profileName: string, isActive: boolean) {
    if (isActive) {
      uiStore.showError('Cannot delete the active profile. Deactivate it first.');
      return;
    }

    if (confirm(`Are you sure you want to delete profile "${profileName}"? This action cannot be undone.`)) {
      try {
        const success = await profileStore.deleteProfile(profileId);
        if (success) {
          uiStore.showSuccess(`Profile "${profileName}" deleted successfully`);
        } else {
          uiStore.showError('Failed to delete profile');
        }
      } catch (error) {
        logger.error('Failed to delete profile:', error);
        uiStore.showError(`Failed to delete profile: ${error}`);
      }
    }
  }

  // Server actions
  async function handleConnectServer(serverId: string) {
    const server = servers.find(s => s.id === serverId);
    if (server) {
      // ✅ FIXED: Clean up stale connection states before reconnecting
      const status = getServerStatus(server);
      if (status === 'error' || status === 'connecting') {
        try {
          await serverStore.disconnectServer(serverId);
        } catch (disconnectError) {
          // Ignore disconnect errors - server might already be disconnected
          logger.warn('Failed to disconnect before reconnecting:', disconnectError);
        }
      }

      await serverStore.connectServer(server.config);
    }
  }

  async function handleDisconnectServer(serverId: string) {
    await serverStore.disconnectServer(serverId);
  }

  function handleConfigureServer(serverId: string) {
    serverStore.selectServer(serverId);
    uiStore.openModal('serverConfig');
  }

  // Helper to extract just the MCP config object (without mcpServers wrapper)
  function extractMcpConfig(server: ServerInfo): Record<string, any> {
    const config = server.config;
    const mcpConfig: any = {};

    if (config.transport_config?.type === 'stdio') {
      mcpConfig.command = config.transport_config.command;
      if (config.transport_config.args && config.transport_config.args.length > 0) {
        mcpConfig.args = config.transport_config.args;
      }
      if (config.environment_variables && Object.keys(config.environment_variables).length > 0) {
        mcpConfig.env = config.environment_variables;
      }
    } else if (config.transport_config?.type === 'http') {
      mcpConfig.url = config.transport_config.url;
      mcpConfig.transport = 'http';
      if (config.transport_config.headers && Object.keys(config.transport_config.headers).length > 0) {
        mcpConfig.headers = config.transport_config.headers;
      }
    } else if (config.transport_config?.type === 'webSocket') {
      mcpConfig.url = config.transport_config.url;
      mcpConfig.transport = 'websocket';
      if (config.transport_config.headers && Object.keys(config.transport_config.headers).length > 0) {
        mcpConfig.headers = config.transport_config.headers;
      }
    } else if (config.transport_config?.type === 'tcp') {
      mcpConfig.host = config.transport_config.host;
      mcpConfig.port = config.transport_config.port;
      mcpConfig.transport = 'tcp';
    } else if (config.transport_config?.type === 'unix') {
      mcpConfig.path = config.transport_config.path;
      mcpConfig.transport = 'unix';
    }

    return mcpConfig;
  }

  // Export all servers to MCP JSON format
  async function handleExportAllServers() {
    if (servers.length === 0) {
      uiStore.showError('No servers to export');
      return;
    }

    try {
      const mcpServersConfig: Record<string, any> = {};

      // Build config for all servers
      for (const server of servers) {
        mcpServersConfig[server.config.name] = extractMcpConfig(server);
      }

      const fullConfig = {
        mcpServers: mcpServersConfig
      };

      const jsonString = JSON.stringify(fullConfig, null, 2);
      await navigator.clipboard.writeText(jsonString);

      // Show animation feedback
      exportAllCopied = true;
      setTimeout(resetExportAllCopyState, 2000);

      uiStore.showSuccess(`Exported ${servers.length} server${servers.length !== 1 ? 's' : ''} to clipboard`);
      logger.debug('📋 Exported all servers:', jsonString);
    } catch (error) {
      logger.error('Failed to export servers:', error);
      uiStore.showError('Failed to export servers');
    }
  }

  // Export servers for a specific profile to MCP JSON format
  async function handleExportProfileServers(profileId: string, profileName: string) {
    try {
      // Get servers in this profile
      const profileServerIds = localProfileServerMap.get(profileId) || new Set();
      if (profileServerIds.size === 0) {
        uiStore.showError(`No servers in profile "${profileName}"`);
        return;
      }

      const profileServers = servers.filter(s => profileServerIds.has(s.id));
      const mcpServersConfig: Record<string, any> = {};

      // Build config for profile servers
      for (const server of profileServers) {
        mcpServersConfig[server.config.name] = extractMcpConfig(server);
      }

      const fullConfig = {
        mcpServers: mcpServersConfig
      };

      const jsonString = JSON.stringify(fullConfig, null, 2);
      await navigator.clipboard.writeText(jsonString);

      // Show animation feedback for this profile
      setProfileCopied(profileId);

      uiStore.showSuccess(`Exported ${profileServers.length} server${profileServers.length !== 1 ? 's' : ''} from "${profileName}" to clipboard`);
      logger.debug('📋 Exported profile servers:', jsonString);
    } catch (error) {
      logger.error('Failed to export profile servers:', error);
      uiStore.showError('Failed to export profile servers');
    }
  }

  // ✅ FIXED: Delete with timeout, loading state, and proper cleanup
  // Track servers being confirmed for deletion to prevent race conditions
  let confirmingDelete = $state<Set<string>>(new Set());
  let deleteCallCounter = 0;
  let globalConfirmInProgress = false; // NUCLEAR OPTION: Prevent ALL deletes while confirm is showing

  // Delete confirmation dialog state
  let deleteConfirmDialog = $state<{serverId: string; serverName: string} | null>(null);

  function requestDeleteServer(serverId: string, serverName: string) {
    // Prevent concurrent deletes on same server
    if (deletingServers.has(serverId)) {
      logger.warn('⚠️ Delete already in progress for:', serverName);
      return;
    }

    // Prevent showing multiple confirm dialogs for the same server
    if (confirmingDelete.has(serverId)) {
      logger.warn('⚠️ Confirmation dialog already open for:', serverName);
      return;
    }

    // Show confirmation dialog
    deleteConfirmDialog = { serverId, serverName };
  }

  async function executeDeleteServer(serverId: string, serverName: string) {
    // Close dialog
    deleteConfirmDialog = null;

    const callId = ++deleteCallCounter;
    console.log(`🗑️ DELETE [${callId}]: executeDeleteServer called for:`, serverName, 'ID:', serverId);
    logger.debug('🗑️ executeDeleteServer called for:', serverName);

    // Add to deleting set
    deletingServers = new Set(deletingServers).add(serverId);
    console.log(`🗑️ DELETE [${callId}]: deletingServers updated:`, Array.from(deletingServers));

    try {
      logger.debug('🗑️ Deleting server:', serverId, serverName);

      // ✅ NEW: Wrap entire operation in timeout
      await withTimeout(
        async () => {
          // Find the server to check its status
          const server = servers.find(s => s.id === serverId);
          if (!server) {
            throw new Error('Server not found');
          }

          // If server is connected, disconnect it first
          const status = getServerStatus(server);
          if (status === 'connected' || status === 'connecting') {
            logger.debug('🔌 Disconnecting server before deletion...');
            try {
              await serverStore.disconnectServer(serverId);
            } catch (disconnectError) {
              logger.warn('⚠️ Failed to disconnect server, continuing with deletion:', disconnectError);
            }
          }

          logger.debug('🗑️ Calling deleteServerConfig...');
          await serverStore.deleteServerConfig(serverId);
          logger.debug('✅ Server deleted successfully');
        },
        TIMEOUTS.DEFAULT_OPERATION,
        'Delete operation timed out'
      );

      // ✅ NEW: Reload profiles and relationships to update server counts
      logger.debug('🔄 Reloading profiles and relationships after deletion...');
      await Promise.all([
        profileStore.loadProfiles(),
        profileStore.loadActiveProfile(),
        loadAllProfileServerRelationships(),
      ]);

      uiStore.showSuccess(`Server "${serverName}" deleted successfully`);
    } catch (error) {
      logger.error('❌ Failed to delete server:', error);
      logger.error('Error details:', JSON.stringify(error, null, 2));
      uiStore.showError(`Failed to delete server: ${error}`);
    } finally {
      // ✅ NEW: Always cleanup, even on error
      deletingServers = new Set(Array.from(deletingServers).filter(id => id !== serverId));
    }
  }

  // Track ALL profile-server relationships (loaded from backend)
  let localProfileServerMap = $state<Map<string, Set<string>>>(new Map());

  // Load all profile-server relationships from backend
  async function loadAllProfileServerRelationships() {
    try {
      logger.debug('📊 Loading ALL profile-server relationships...');
      const relationships: Record<string, string[]> = await invoke('get_all_profile_server_relationships');

      // Convert to Map<profileId, Set<serverId>>
      const newMap = new Map<string, Set<string>>();
      for (const [profileId, serverIds] of Object.entries(relationships)) {
        newMap.set(profileId, new Set(serverIds));
      }

      localProfileServerMap = newMap;

      logger.debug('✅ Loaded relationships for', newMap.size, 'profiles:', {
        profiles: Array.from(newMap.entries()).map(([pid, sids]) => ({
          profileId: pid,
          serverCount: sids.size,
          serverIds: Array.from(sids)
        }))
      });
    } catch (error) {
      logger.error('❌ Failed to load profile-server relationships:', error);
    }
  }

  // ✅ FIXED: Single effect to sync active profile (prevents infinite loop)
  // Track previous profile ID to detect actual profile changes
  let previousProfileId = $state<string | null>(null);

  $effect(() => {
    // Only run if we have an active profile with servers
    if (!activeProfile?.servers || !activeProfile.profile) {
      previousProfileId = null;
      return;
    }

    const profileId = activeProfile.profile.id;
    const serverIds = new Set(activeProfile.servers.map(ps => ps.server_id));

    // Check if data actually changed (prevents unnecessary updates)
    const existing = localProfileServerMap.get(profileId);
    const profileChanged = previousProfileId !== profileId;
    const serversChanged = !existing ||
      existing.size !== serverIds.size ||
      !Array.from(serverIds).every(id => existing.has(id));

    // Only update if something actually changed
    if (profileChanged || serversChanged) {
      logger.debug('🔄 Syncing active profile to local map:', {
        profileId,
        profileName: activeProfile.profile.name,
        serverCount: serverIds.size,
        serverIds: Array.from(serverIds),
        reason: profileChanged ? 'Profile changed' : 'Servers changed'
      });

      // Create new Map to trigger reactivity
      const newMap = new Map(localProfileServerMap);
      newMap.set(profileId, serverIds);
      localProfileServerMap = newMap;
      previousProfileId = profileId;
    }
  });

  // Check if a server is in a specific profile
  function isServerInProfile(serverId: string, profileId: string): boolean {
    // First check local map (includes recent changes)
    if (localProfileServerMap.has(profileId)) {
      return localProfileServerMap.get(profileId)!.has(serverId);
    }

    // Fall back to serverProfileMap (active profile only)
    const profileId2 = serverProfileMap().get(serverId);
    if (profileId2 === profileId) return true;

    // Also check if this profile is active and has the server
    if (activeProfile?.profile?.id === profileId) {
      return activeProfile.servers?.some(ps => ps.server_id === serverId) || false;
    }

    return false;
  }

  // Update local map after toggle
  function updateLocalProfileServerMap(serverId: string, profileId: string, inProfile: boolean) {
    if (!localProfileServerMap.has(profileId)) {
      localProfileServerMap.set(profileId, new Set());
    }
    const serverSet = localProfileServerMap.get(profileId)!;
    if (inProfile) {
      serverSet.add(serverId);
    } else {
      serverSet.delete(serverId);
    }
    localProfileServerMap = localProfileServerMap; // Trigger reactivity
  }

  async function handleAddServerToProfile(serverId: string, profileId: string) {
    try {
      await invoke('add_server_to_profile', {
        request: {
          profile_id: profileId,
          server_id: serverId,
          startup_order: 0,
          startup_delay_ms: 0,
          auto_connect: true,
          auto_restart: false,
          required: false,
          environment_overrides: null,
        }
      });

      // Reload data to reflect changes
      await Promise.all([
        profileStore.loadProfiles(),
        profileStore.loadActiveProfile(),
        serverStore.loadServers(),
        loadAllProfileServerRelationships(), // Reload ALL relationships
      ]);
    } catch (error) {
      logger.error('Failed to add server to profile:', error);
      throw error;
    }
  }

  async function handleRemoveServerFromProfile(serverId: string, profileId: string) {
    try {
      await invoke('remove_server_from_profile', {
        profileId,
        serverId,
      });

      // Reload data to reflect changes
      await Promise.all([
        profileStore.loadProfiles(),
        profileStore.loadActiveProfile(),
        serverStore.loadServers(),
        loadAllProfileServerRelationships(), // Reload ALL relationships
      ]);
    } catch (error) {
      logger.error('Failed to remove server from profile:', error);
      throw error;
    }
  }

  async function handleToggleServerProfile(serverId: string, profileId: string, currentlyInProfile: boolean) {
    const profile = profiles.find(p => p.id === profileId);
    const server = servers.find(s => s.id === serverId);

    if (!profile || !server) {
      logger.error('❌ Profile or server not found:', { profileId, serverId, profile, server });
      return;
    }

    logger.debug('🔄 Toggling server profile:', {
      serverId,
      serverName: server.config.name,
      profileId,
      profileName: profile.name,
      currentlyInProfile,
      action: currentlyInProfile ? 'REMOVE' : 'ADD'
    });

    togglingProfile = profileId;

    // Optimistically update local map
    updateLocalProfileServerMap(serverId, profileId, !currentlyInProfile);

    try {
      if (currentlyInProfile) {
        logger.debug('🗑️ Removing server from profile...');
        await handleRemoveServerFromProfile(serverId, profileId);
        logger.debug('✅ Removed successfully');
        uiStore.showSuccess(`Removed "${server.config.name}" from "${profile.name}"`);
      } else {
        logger.debug('➕ Adding server to profile...');
        await handleAddServerToProfile(serverId, profileId);
        logger.debug('✅ Added successfully');
        uiStore.showSuccess(`Added "${server.config.name}" to "${profile.name}"`);
      }

      // Verify local map update
      logger.debug('📊 Local map state:', {
        profileId,
        servers: Array.from(localProfileServerMap.get(profileId) || [])
      });
    } catch (error) {
      logger.error('❌ Failed to toggle:', error);
      // Revert optimistic update on error
      updateLocalProfileServerMap(serverId, profileId, currentlyInProfile);
      uiStore.showError(`Failed to ${currentlyInProfile ? 'remove from' : 'add to'} profile: ${error}`);
    } finally {
      togglingProfile = null;
    }
  }

  // Filtered profiles for search
  const filteredProfilesForDropdown = $derived(() => {
    if (!profileSearchQuery.trim()) return profiles;
    const query = profileSearchQuery.toLowerCase();
    return profiles.filter(p => p.name.toLowerCase().includes(query));
  });

  function handleSelectProfile(profileId: string) {
    selectedProfileId = selectedProfileId === profileId ? null : profileId;
  }

  // Convert server config to standard MCP JSON format
  function serverToMcpJson(server: ServerInfo): string {
    const config = server.config;
    const mcpConfig: any = {};

    if (config.transport_config?.type === 'stdio') {
      mcpConfig.command = config.transport_config.command;
      if (config.transport_config.args && config.transport_config.args.length > 0) {
        mcpConfig.args = config.transport_config.args;
      }
      if (config.environment_variables && Object.keys(config.environment_variables).length > 0) {
        mcpConfig.env = config.environment_variables;
      }
    } else if (config.transport_config?.type === 'http') {
      mcpConfig.url = config.transport_config.url;
      mcpConfig.transport = 'http';
      if (config.transport_config.headers && Object.keys(config.transport_config.headers).length > 0) {
        mcpConfig.headers = config.transport_config.headers;
      }
    } else if (config.transport_config?.type === 'webSocket') {  // ✅ FIXED: camelCase
      mcpConfig.url = config.transport_config.url;
      mcpConfig.transport = 'websocket';
      if (config.transport_config.headers && Object.keys(config.transport_config.headers).length > 0) {
        mcpConfig.headers = config.transport_config.headers;
      }
    } else if (config.transport_config?.type === 'tcp') {
      mcpConfig.host = config.transport_config.host;
      mcpConfig.port = config.transport_config.port;
      mcpConfig.transport = 'tcp';
    } else if (config.transport_config?.type === 'unix') {
      mcpConfig.path = config.transport_config.path;
      mcpConfig.transport = 'unix';
    }

    // Wrap in mcpServers format
    const fullConfig = {
      mcpServers: {
        [config.name]: mcpConfig
      }
    };

    return JSON.stringify(fullConfig, null, 2);
  }

  async function handleCopyServerJson(server: ServerInfo) {
    try {
      const jsonConfig = serverToMcpJson(server);
      await navigator.clipboard.writeText(jsonConfig);

      // Show animation feedback for this server
      setServerCopied(server.id);

      uiStore.showSuccess(`Copied "${server.config.name}" configuration to clipboard`);
      logger.debug('📋 Copied MCP config:', jsonConfig);
    } catch (error) {
      logger.error('Failed to copy to clipboard:', error);
      uiStore.showError('Failed to copy to clipboard');
    }
  }

  function getProfileIcon(icon?: string): string {
    return icon || '📁';
  }

  function getTransportLabel(server: ServerInfo): string {
    const type = server.config.transport_config?.type || 'unknown';
    return type.toUpperCase();
  }

  function getServerStatusColor(server: ServerInfo): string {
    const status = getServerStatus(server);
    switch (status) {
      case 'connected': return 'text-green-600 dark:text-green-400';
      case 'connecting': return 'text-yellow-600 dark:text-yellow-400';
      case 'error': return 'text-red-600 dark:text-red-400';
      default: return 'text-gray-400 dark:text-gray-500';
    }
  }

  function getServerStatusIcon(server: ServerInfo): string {
    const status = getServerStatus(server);
    switch (status) {
      case 'connected': return '🟢';
      case 'connecting': return '🟡';
      case 'error': return '🔴';
      default: return '⚪';
    }
  }

  // Extract just the executable name from a stdio path
  function getExecutableName(path: string): string {
    const parts = path.split('/');
    return parts[parts.length - 1] || path;
  }

  // Get display path for server (truncated for stdio, full for URLs)
  function getServerDisplayPath(server: ServerInfo): string {
    if (server.config.transport_config?.type === 'stdio') {
      // For stdio, show just the executable name
      return getExecutableName(server.config.transport_config.command);
    } else if (server.config.transport_config?.type === 'http') {
      return server.config.transport_config.url;
    } else if (server.config.transport_config?.type === 'webSocket') {  // ✅ FIXED: camelCase
      return server.config.transport_config.url;
    } else {
      return getTransportLabel(server);
    }
  }
</script>

<div class="h-full flex flex-col bg-gray-50 dark:bg-gray-900">
  <!-- Header -->
  <div class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Server Management</h1>
        <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
          Manage MCP servers and organize them into profiles
        </p>
      </div>
      <div class="flex gap-2">
        <Button variant="primary" leftIcon={Plus} onclick={() => uiStore.openModal('addServer')}>
          Add Server
        </Button>
        <Button
          variant="secondary"
          leftIcon={Package}
          onclick={() => (showRegistryBrowser = true)}
          title="Browse Docker MCP Registry (270+ pre-configured servers)"
        >
          Browse Docker Registry
        </Button>
        <Button
          variant="secondary"
          leftIcon={Upload}
          onclick={openInstallModal}
          disabled={servers.length === 0}
          title="Export servers to clipboard or install to client applications"
        >
          Export
        </Button>
      </div>
    </div>
  </div>

  <!-- Two-Column Layout -->
  <div class="flex-1 overflow-hidden grid grid-cols-[30%_70%]">
    <!-- LEFT PANEL: Profiles -->
    <div class="border-r border-gray-200 dark:border-gray-700 overflow-y-auto bg-white dark:bg-gray-800">
      <div class="p-4">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-sm font-semibold text-gray-700 dark:text-gray-300 uppercase tracking-wide">
            Profiles ({profiles.length})
          </h2>
          <button
            onclick={handleCreateProfile}
            class="text-xs text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 font-medium"
          >
            + New Profile
          </button>
        </div>

        {#if profiles.length === 0}
          <div class="text-center py-8">
            <ServerIcon size={32} class="mx-auto text-gray-400 mb-2" />
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-3">
              No profiles yet
            </p>
            <button
              onclick={handleCreateProfile}
              class="text-sm text-blue-600 hover:text-blue-700 dark:text-blue-400 font-medium"
            >
              Create your first profile
            </button>
          </div>
        {:else}
          <div class="space-y-2">
            {#each profiles as profile}
              {@const isActive = activeProfile?.profile?.id === profile.id}
              {@const isSelected = selectedProfileId === profile.id}
              <div
                role="button"
                tabindex="0"
                onclick={() => handleSelectProfile(profile.id)}
                onkeydown={(e) => e.key === 'Enter' && handleSelectProfile(profile.id)}
                class="w-full text-left p-3 rounded-lg border transition-all cursor-pointer {isSelected ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20' : isActive ? 'border-green-500 bg-green-50 dark:bg-green-900/20' : 'border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-750'}"
              >
                <div class="flex items-start justify-between mb-2">
                  <div class="flex items-center gap-2">
                    <span class="text-lg">{getProfileIcon(profile.icon)}</span>
                    <div>
                      <div class="flex items-center gap-2">
                        {#if isActive}
                          <Zap size={14} class="text-green-600 dark:text-green-400" />
                        {/if}
                        <span class="font-medium text-sm text-gray-900 dark:text-white">
                          {profile.name}
                        </span>
                      </div>
                      <span class="text-xs text-gray-500 dark:text-gray-400">
                        {profile.server_count} server{profile.server_count !== 1 ? 's' : ''}
                      </span>
                    </div>
                  </div>
                </div>

                {#if isActive}
                  <div class="text-xs text-green-700 dark:text-green-300 font-medium mb-2">
                    ACTIVE
                  </div>
                {/if}

                <div class="flex items-center gap-1">
                  <button
                    onclick={(e) => {
                      e.stopPropagation();
                      handleEditProfile(profile.id);
                    }}
                    class="flex-1 text-xs py-1.5 px-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded hover:bg-gray-50 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300"
                  >
                    <Edit size={12} class="inline mr-1" />
                    Edit
                  </button>

                  <button
                    onclick={(e) => {
                      e.stopPropagation();
                      handleExportProfileServers(profile.id, profile.name);
                    }}
                    title="Export profile servers to clipboard"
                    class="text-xs py-1.5 px-2 rounded transition-all {exportProfileCopied.has(profile.id) ? 'bg-green-100 dark:bg-green-900 border border-green-300 dark:border-green-600 text-green-600 dark:text-green-400' : 'bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300'}"
                  >
                    {#if exportProfileCopied.has(profile.id)}
                      <CheckCircle size={12} class="inline" />
                    {:else}
                      <Download size={12} class="inline" />
                    {/if}
                  </button>

                  <button
                    onclick={(e) => {
                      e.stopPropagation();
                      handleDeleteProfile(profile.id, profile.name, isActive);
                    }}
                    class="text-xs py-1.5 px-2 bg-white dark:bg-gray-700 border border-red-300 dark:border-red-600 text-red-600 dark:text-red-400 rounded hover:bg-red-50 dark:hover:bg-red-900/20"
                    disabled={loading}
                  >
                    <Trash2 size={12} class="inline" />
                  </button>
                  {#if isActive}
                    <button
                      onclick={(e) => {
                        e.stopPropagation();
                        handleDeactivateProfile();
                      }}
                      class="flex-1 text-xs py-1.5 px-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded hover:bg-gray-50 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300"
                      disabled={loading}
                    >
                      <Square size={12} class="inline mr-1" />
                      Deactivate
                    </button>
                  {:else}
                    <button
                      onclick={(e) => {
                        e.stopPropagation();
                        handleActivateProfile(profile.id);
                      }}
                      class="flex-1 text-xs py-1.5 px-2 bg-blue-600 dark:bg-blue-500 text-white rounded hover:bg-blue-700 dark:hover:bg-blue-600"
                      disabled={loading}
                    >
                      <Play size={12} class="inline mr-1" />
                      Activate
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <!-- RIGHT PANEL: All Servers -->
    <div class="overflow-y-auto bg-gray-50 dark:bg-gray-900">
      <div class="p-6">
        <div class="flex items-center justify-between mb-6">
          <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white">
              All Servers ({servers.length})
            </h2>
            {#if unassignedServers().length > 0}
              <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
                💡 {unassignedServers().length} server{unassignedServers().length !== 1 ? 's' : ''} not in any profile
              </p>
            {/if}
          </div>

          <!-- Toolbar Actions -->
          <div class="flex items-center gap-2">
            <!-- Export All Button -->
            <button
              onclick={handleExportAllServers}
              disabled={servers.length === 0}
              class="flex items-center px-3 py-1.5 rounded transition-all {servers.length === 0 ? 'text-gray-400 dark:text-gray-600 cursor-not-allowed' : exportAllCopied ? 'bg-green-50 dark:bg-green-900 text-green-600 dark:text-green-400' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-800'}"
              title="Export all servers to clipboard"
            >
              {#if exportAllCopied}
                <CheckCircle size={16} />
              {:else}
                <Download size={16} />
              {/if}
            </button>

            <!-- View Toggle -->
            <div class="flex items-center gap-1 bg-gray-100 dark:bg-gray-800 rounded-lg p-1">
              <button
                onclick={() => viewMode = 'card'}
                class="px-3 py-1.5 rounded transition-colors {viewMode === 'card' ? 'bg-white dark:bg-gray-700 shadow-sm text-gray-900 dark:text-white' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'}"
                title="Card View"
              >
                <LayoutGrid size={16} />
              </button>
              <button
                onclick={() => viewMode = 'list'}
                class="px-3 py-1.5 rounded transition-colors {viewMode === 'list' ? 'bg-white dark:bg-gray-700 shadow-sm text-gray-900 dark:text-white' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'}"
                title="List View (Compact)"
              >
                <List size={16} />
              </button>
            </div>
          </div>
        </div>

        {#if servers.length === 0}
          <div class="text-center py-12 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
            <ServerIcon size={48} class="mx-auto text-gray-400 mb-4" />
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">
              No Servers Yet
            </h3>
            <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
              Add your first MCP server to get started
            </p>
            <Button variant="primary" leftIcon={Plus} onclick={() => uiStore.openModal('addServer')}>
              Add Server
            </Button>
          </div>
        {:else if viewMode === 'card'}
          <div class="space-y-3">
            {#each servers as server}
              {@const serverProfiles = getServerProfiles(server.id)}
              {@const isHighlighted = isServerHighlighted(server.id)}
              {@const status = getServerStatus(server)}

              <div
                class="bg-white dark:bg-gray-800 rounded-lg border p-4 transition-all {isHighlighted ? 'border-blue-500 ring-2 ring-blue-200 dark:ring-blue-800' : selectedProfileId ? 'opacity-60 border-gray-200 dark:border-gray-700' : 'border-gray-200 dark:border-gray-700'}"
              >
                <!-- Server Header -->
                <div class="flex items-start justify-between mb-3">
                  <div class="flex items-start gap-3 flex-1">
                    <span class="text-2xl">{getServerStatusIcon(server)}</span>
                    <div class="flex-1 min-w-0">
                      <h3 class="font-semibold text-gray-900 dark:text-white truncate">
                        {server.config.name}
                      </h3>
                      <p class="text-sm text-gray-600 dark:text-gray-400 font-mono truncate" title={server.config.transport_config?.type === 'stdio' ? server.config.transport_config.command : undefined}>
                        {getServerDisplayPath(server)}
                      </p>
                      <div class="flex flex-wrap items-center gap-2 mt-1">
                        <span class="text-xs px-2 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded">
                          {getTransportLabel(server)}
                        </span>
                        {#if serverProfiles.length > 1}
                          <span class="text-xs px-2 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded">
                            In: {serverProfiles.length} profiles
                          </span>
                        {:else if serverProfiles.length === 1}
                          <span class="text-xs px-2 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded">
                            In: {serverProfiles[0].name}
                          </span>
                        {:else}
                          <span class="text-xs px-2 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400 rounded">
                            Not in any profile
                          </span>
                        {/if}
                        <!-- Capability Tags -->
                        {#if server.capabilities}
                          {#if server.capabilities.tools}
                            <span class="text-xs px-2 py-0.5 bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 rounded">
                              Tools
                            </span>
                          {/if}
                          {#if server.capabilities.resources}
                            <span class="text-xs px-2 py-0.5 bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded">
                              Resources
                            </span>
                          {/if}
                          {#if server.capabilities.prompts}
                            <span class="text-xs px-2 py-0.5 bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 rounded">
                              Prompts
                            </span>
                          {/if}
                          {#if server.capabilities.sampling}
                            <span class="text-xs px-2 py-0.5 bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300 rounded">
                              Sampling
                            </span>
                          {/if}
                          {#if server.capabilities.elicitation}
                            <span class="text-xs px-2 py-0.5 bg-pink-100 dark:bg-pink-900/30 text-pink-700 dark:text-pink-300 rounded">
                              Elicitation
                            </span>
                          {/if}
                        {/if}
                      </div>
                    </div>
                  </div>
                </div>

                <!-- Server Actions -->
                <div class="flex items-center gap-2">
                  {#if status === 'connected'}
                    <button
                      onclick={() => handleDisconnectServer(server.id)}
                      class="flex-1 text-sm py-2 px-3 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300 border border-red-200 dark:border-red-800 rounded hover:bg-red-100 dark:hover:bg-red-900/30"
                    >
                      <Square size={14} class="inline mr-1" />
                      Disconnect
                    </button>
                  {:else}
                    <button
                      onclick={() => handleConnectServer(server.id)}
                      class="flex-1 text-sm py-2 px-3 bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300 border border-green-200 dark:border-green-800 rounded hover:bg-green-100 dark:hover:bg-green-900/30"
                    >
                      <Play size={14} class="inline mr-1" />
                      Connect
                    </button>
                  {/if}
                  <button
                    onclick={() => handleConfigureServer(server.id)}
                    class="text-sm py-2 px-3 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded hover:bg-gray-50 dark:hover:bg-gray-600"
                  >
                    <Settings size={14} class="inline mr-1" />
                    Configure
                  </button>
                  <button
                    onclick={() => handleCopyServerJson(server)}
                    class="text-sm py-2 px-3 rounded transition-all {serverCopied.has(server.id) ? 'bg-green-100 dark:bg-green-900 border border-green-300 dark:border-green-600 text-green-700 dark:text-green-300' : 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600'}"
                    title="Copy MCP JSON configuration"
                  >
                    {#if serverCopied.has(server.id)}
                      <CheckCircle size={14} class="inline mr-1" />
                    {:else}
                      <Copy size={14} class="inline mr-1" />
                    {/if}
                    Copy JSON
                  </button>
                  <button
                    onclick={(e) => {
                      e.stopPropagation();
                      requestDeleteServer(server.id, server.config.name);
                    }}
                    disabled={deletingServers.has(server.id)}
                    class="text-sm py-2 px-3 bg-white dark:bg-gray-700 text-red-600 dark:text-red-400 border border-gray-300 dark:border-gray-600 rounded hover:bg-red-50 dark:hover:bg-red-900/20 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {#if deletingServers.has(server.id)}
                      <div class="flex items-center">
                        <div class="animate-spin h-3 w-3 border-2 border-red-600 border-t-transparent rounded-full mr-1"></div>
                        Deleting...
                      </div>
                    {:else}
                      <Trash2 size={14} class="inline mr-1" />
                      Delete
                    {/if}
                  </button>
                  <!-- Manage Profiles Dropdown (Always visible) -->
                  <div class="relative">
                    <button
                      onclick={(e) => {
                        e.stopPropagation();
                        showProfileDropdown = showProfileDropdown === server.id ? null : server.id;
                        if (showProfileDropdown === server.id) {
                          profileSearchQuery = '';
                        }
                      }}
                      class="text-sm py-2 px-3 bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 border border-blue-200 dark:border-blue-800 rounded hover:bg-blue-100 dark:hover:bg-blue-900/30"
                    >
                      <Settings size={14} class="inline mr-1" />
                      Profiles ▾
                    </button>

                    {#if showProfileDropdown === server.id}
                      <div class="absolute z-10 mt-1 right-0 w-72 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg">
                        {#if profiles.length === 0}
                          <div class="px-4 py-3 text-sm text-gray-500 dark:text-gray-400 text-center">
                            No profiles available
                          </div>
                        {:else}
                          <!-- Header with Search -->
                          <div class="p-3 border-b border-gray-200 dark:border-gray-700">
                            <div class="flex items-center justify-between mb-2">
                              <span class="text-xs font-semibold text-gray-700 dark:text-gray-300 uppercase tracking-wide">
                                Manage Profiles
                              </span>
                              <button
                                onclick={(e) => {
                                  e.stopPropagation();
                                  showProfileDropdown = null;
                                }}
                                class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                              >
                                <X size={16} />
                              </button>
                            </div>
                            {#if profiles.length > 5}
                              <input
                                type="text"
                                bind:value={profileSearchQuery}
                                placeholder="Search profiles..."
                                class="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                                onclick={(e) => e.stopPropagation()}
                              />
                            {/if}
                          </div>

                          <!-- Profile Checkboxes -->
                          <div class="max-h-64 overflow-y-auto">
                            {#if filteredProfilesForDropdown().length === 0}
                              <div class="px-4 py-3 text-sm text-gray-500 dark:text-gray-400 text-center">
                                No profiles match your search
                              </div>
                            {:else}
                              {#each filteredProfilesForDropdown() as prof}
                                {@const inProfile = isServerInProfile(server.id, prof.id)}
                                {@const isToggling = togglingProfile === prof.id}
                                <label
                                  class="flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 dark:hover:bg-gray-700 cursor-pointer transition-colors {isToggling ? 'opacity-50' : ''}"
                                  onclick={(e) => e.stopPropagation()}
                                >
                                  <input
                                    type="checkbox"
                                    checked={inProfile}
                                    disabled={isToggling}
                                    onchange={() => handleToggleServerProfile(server.id, prof.id, inProfile)}
                                    class="w-4 h-4 text-blue-600 border-gray-300 dark:border-gray-600 rounded focus:ring-blue-500 focus:ring-2"
                                  />
                                  <div class="flex items-center gap-2 flex-1 min-w-0">
                                    <span class="text-lg flex-shrink-0">{getProfileIcon(prof.icon)}</span>
                                    <div class="flex-1 min-w-0">
                                      <div class="text-sm font-medium text-gray-900 dark:text-white truncate">
                                        {prof.name}
                                      </div>
                                      <div class="text-xs text-gray-500 dark:text-gray-400">
                                        {prof.server_count} server{prof.server_count !== 1 ? 's' : ''}
                                      </div>
                                    </div>
                                  </div>
                                </label>
                              {/each}
                            {/if}
                          </div>

                          <!-- Footer -->
                          <div class="p-3 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-750">
                            <p class="text-xs text-gray-600 dark:text-gray-400">
                              💡 Check profiles to add, uncheck to remove
                            </p>
                          </div>
                        {/if}
                      </div>
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <!-- List View (Compact) -->
          <div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 divide-y divide-gray-200 dark:divide-gray-700">
            {#each servers as server}
              {@const serverProfiles = getServerProfiles(server.id)}
              {@const isHighlighted = isServerHighlighted(server.id)}
              {@const status = getServerStatus(server)}

              <div
                class="flex items-center gap-3 p-3 hover:bg-gray-50 dark:hover:bg-gray-750 transition-colors {isHighlighted ? 'bg-blue-50 dark:bg-blue-900/20' : ''}"
              >
                <!-- Status Icon -->
                <span class="text-lg">{getServerStatusIcon(server)}</span>

                <!-- Server Name & Details -->
                <div class="flex-1 min-w-0">
                  <div class="flex flex-wrap items-center gap-1.5">
                    <h3 class="font-medium text-sm text-gray-900 dark:text-white truncate">
                      {server.config.name}
                    </h3>
                    <span class="text-xs px-1.5 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded">
                      {getTransportLabel(server)}
                    </span>
                    {#if serverProfiles.length > 1}
                      <span class="text-xs px-1.5 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded">
                        {serverProfiles.length} profiles
                      </span>
                    {:else if serverProfiles.length === 1}
                      <span class="text-xs px-1.5 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded">
                        {serverProfiles[0].name}
                      </span>
                    {/if}
                    <!-- Capability Tags -->
                    {#if server.capabilities}
                      {#if server.capabilities.tools}
                        <span class="text-xs px-1.5 py-0.5 bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 rounded">
                          T
                        </span>
                      {/if}
                      {#if server.capabilities.resources}
                        <span class="text-xs px-1.5 py-0.5 bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded">
                          R
                        </span>
                      {/if}
                      {#if server.capabilities.prompts}
                        <span class="text-xs px-1.5 py-0.5 bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 rounded">
                          P
                        </span>
                      {/if}
                      {#if server.capabilities.sampling}
                        <span class="text-xs px-1.5 py-0.5 bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300 rounded">
                          S
                        </span>
                      {/if}
                      {#if server.capabilities.elicitation}
                        <span class="text-xs px-1.5 py-0.5 bg-pink-100 dark:bg-pink-900/30 text-pink-700 dark:text-pink-300 rounded">
                          E
                        </span>
                      {/if}
                    {/if}
                  </div>
                </div>

                <!-- Quick Actions -->
                <div class="flex items-center gap-1">
                  {#if status === 'connected'}
                    <button
                      onclick={() => handleDisconnectServer(server.id)}
                      class="p-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded"
                      title="Disconnect"
                    >
                      <Square size={14} />
                    </button>
                  {:else}
                    <button
                      onclick={() => handleConnectServer(server.id)}
                      class="p-1.5 text-green-600 hover:bg-green-50 dark:hover:bg-green-900/20 rounded"
                      title="Connect"
                    >
                      <Play size={14} />
                    </button>
                  {/if}

                  <!-- Profiles Dropdown -->
                  <div class="relative">
                    <button
                      onclick={(e) => {
                        e.stopPropagation();
                        showProfileDropdown = showProfileDropdown === server.id ? null : server.id;
                        if (showProfileDropdown === server.id) {
                          profileSearchQuery = '';
                        }
                      }}
                      class="p-1.5 text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded"
                      title="Manage Profiles"
                    >
                      <Settings size={14} />
                    </button>

                    {#if showProfileDropdown === server.id}
                      <div class="absolute z-10 mt-1 right-0 w-72 bg-white dark:bg-gray-800 border rounded-lg shadow-lg">
                        <!-- Header with Search -->
                        <div class="p-3 border-b">
                          <div class="flex items-center justify-between mb-2">
                            <span class="text-xs font-semibold uppercase">Manage Profiles</span>
                            <button onclick={(e) => { e.stopPropagation(); showProfileDropdown = null; }}>
                              <X size={16} />
                            </button>
                          </div>
                          {#if profiles.length > 5}
                            <input
                              type="text"
                              bind:value={profileSearchQuery}
                              placeholder="Search profiles..."
                              class="w-full px-2 py-1.5 text-sm border rounded"
                              onclick={(e) => e.stopPropagation()}
                            />
                          {/if}
                        </div>

                        <!-- Profile Checkboxes -->
                        <div class="max-h-64 overflow-y-auto">
                          {#each filteredProfilesForDropdown() as prof}
                            {@const inProfile = isServerInProfile(server.id, prof.id)}
                            {@const isToggling = togglingProfile === prof.id}
                            <label class="flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 cursor-pointer {isToggling ? 'opacity-50' : ''}">
                              <input
                                type="checkbox"
                                checked={inProfile}
                                disabled={isToggling}
                                onchange={() => handleToggleServerProfile(server.id, prof.id, inProfile)}
                                class="w-4 h-4 text-blue-600 rounded"
                              />
                              <div class="flex items-center gap-2 flex-1">
                                <span class="text-lg">{getProfileIcon(prof.icon)}</span>
                                <div class="flex-1">
                                  <div class="text-sm font-medium">{prof.name}</div>
                                  <div class="text-xs text-gray-500">{prof.server_count} servers</div>
                                </div>
                              </div>
                            </label>
                          {/each}
                        </div>

                        <!-- Footer -->
                        <div class="p-3 border-t bg-gray-50">
                          <p class="text-xs text-gray-600">💡 Check profiles to add, uncheck to remove</p>
                        </div>
                      </div>
                    {/if}
                  </div>

                  <button
                    onclick={() => handleCopyServerJson(server)}
                    class="p-1.5 rounded transition-all {serverCopied.has(server.id) ? 'bg-green-100 dark:bg-green-900 text-green-600 dark:text-green-400' : 'text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700'}"
                    title="Copy JSON"
                  >
                    {#if serverCopied.has(server.id)}
                      <CheckCircle size={14} />
                    {:else}
                      <Copy size={14} />
                    {/if}
                  </button>

                  <button
                    onclick={() => handleConfigureServer(server.id)}
                    class="p-1.5 text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
                    title="Configure"
                  >
                    <Edit size={14} />
                  </button>

                  <button
                    onclick={(e) => {
                      e.stopPropagation();
                      requestDeleteServer(server.id, server.config.name);
                    }}
                    disabled={deletingServers.has(server.id)}
                    class="p-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded disabled:opacity-50 disabled:cursor-not-allowed"
                    title={deletingServers.has(server.id) ? 'Deleting...' : 'Delete'}
                  >
                    {#if deletingServers.has(server.id)}
                      <div class="animate-spin h-3 w-3 border-2 border-red-600 border-t-transparent rounded-full"></div>
                    {:else}
                      <Trash2 size={14} />
                    {/if}
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<!-- Profile Editor Modal -->
{#if showProfileEditor}
  <ProfileEditor
    profileId={editingProfile}
    onClose={async () => {
      uiStore.closeProfileEditor();
      // Reload all profile-server relationships after editing
      await loadAllProfileServerRelationships();
    }}
  />
{/if}

<!-- Install Client Modal -->
<InstallClientModal
  isOpen={showInstallModal}
  onClose={closeInstallModal}
  servers={servers}
  profiles={profiles}
  localProfileServerMap={localProfileServerMap}
/>

<!-- Registry Browser Modal -->
{#if showRegistryBrowser}
  <div class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4">
    <div class="bg-white dark:bg-gray-900 rounded-lg shadow-2xl w-full max-w-7xl h-[90vh] flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
        <div>
          <h2 class="text-2xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
            <Package size={24} />
            Docker MCP Registry
          </h2>
          <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
            Browse 270+ pre-configured MCP servers from the official Docker registry
          </p>
        </div>
        <button
          onclick={() => (showRegistryBrowser = false)}
          class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors"
          title="Close"
        >
          <X size={24} />
        </button>
      </div>

      <!-- Registry Browser Content -->
      <div class="flex-1 overflow-hidden">
        <RegistryBrowser onServerAdded={() => (showRegistryBrowser = false)} />
      </div>
    </div>
  </div>
{/if}

<!-- Delete Confirmation Dialog -->
{#if deleteConfirmDialog}
  <ConfirmDialog
    title="Delete Server"
    message={`Are you sure you want to delete server "${deleteConfirmDialog.serverName}"? This action cannot be undone.`}
    confirmText="Delete"
    cancelText="Cancel"
    variant="danger"
    onConfirm={() => executeDeleteServer(deleteConfirmDialog.serverId, deleteConfirmDialog.serverName)}
    onCancel={() => { deleteConfirmDialog = null; }}
  />
{/if}
