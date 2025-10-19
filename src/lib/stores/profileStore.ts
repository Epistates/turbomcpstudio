import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { uiStore } from './uiStore';
import { createLogger } from '$lib/utils/logger';

// ============================================================================
// TYPES
// ============================================================================

export interface ServerProfile {
  id: string;
  name: string;
  description?: string;
  icon?: string;
  color?: string;
  auto_activate: boolean;
  created_at: string;
  updated_at: string;
}

export interface ServerProfileWithCount extends ServerProfile {
  server_count: number;
}

export interface ProfileServerConfig {
  startup_order: number;
  startup_delay_ms: number;
  auto_connect: boolean;
  auto_restart: boolean;
  required: boolean;
  environment_overrides?: Record<string, string>;
}

export interface ProfileServer {
  profile_id: string;
  server_id: string;
  server_name: string;
  server_description?: string;
  transport_type: string;
  startup_order: number;
  startup_delay_ms: number;
  auto_connect: boolean;
  auto_restart: boolean;
  required: boolean;
  environment_overrides?: Record<string, string>;
  created_at: string;
}

export interface ProfileActivation {
  id: string;
  profile_id: string;
  profile_name: string;
  activated_at: string;
  deactivated_at?: string;
  success_count: number;
  failure_count: number;
  errors?: string[];
}

export interface ActiveProfileState {
  profile?: ServerProfile;
  servers: ProfileServer[];
  activation?: ProfileActivation;
  is_activating: boolean;
}

export interface CreateProfileRequest {
  name: string;
  description?: string;
  icon?: string;
  color?: string;
  auto_activate?: boolean;
}

export interface AddServerToProfileRequest {
  profile_id: string;
  server_id: string;
  startup_order: number;
  startup_delay_ms: number;
  auto_connect: boolean;
  auto_restart: boolean;
  required: boolean;
  environment_overrides?: Record<string, string>;
}

// ============================================================================
// STORE STATE
// ============================================================================

interface ProfileStoreState {
  profiles: ServerProfileWithCount[];
  activeProfiles: Map<string, ActiveProfileState>;  // Multi-profile support
  selectedProfileId?: string;
  loading: boolean;
  error?: string;
}

const initialState: ProfileStoreState = {
  profiles: [],
  activeProfiles: new Map(),  // Multiple active profiles
  selectedProfileId: undefined,
  loading: false,
  error: undefined,
};

// ============================================================================
// STORE
// ============================================================================

function createProfileStore() {
  const { subscribe, set, update } = writable<ProfileStoreState>(initialState);
  const logger = createLogger('ProfileStore');

  return {
    subscribe,

    // Load all profiles
    async loadProfiles() {
      update((state) => ({ ...state, loading: true, error: undefined }));

      try {
        const profiles = await invoke<ServerProfileWithCount[]>('list_server_profiles');
        update((state) => ({ ...state, profiles, loading: false }));
      } catch (error) {
        const errorMessage = String(error);
        update((state) => ({ ...state, error: errorMessage, loading: false }));
        uiStore.showError(`Failed to load profiles: ${errorMessage}`);
      }
    },

    // Create a new profile
    async createProfile(request: CreateProfileRequest): Promise<ServerProfile | null> {
      try {
        const profile = await invoke<ServerProfile>('create_server_profile', { request });
        await this.loadProfiles();
        uiStore.showSuccess(`Profile "${profile.name}" created`);
        return profile;
      } catch (error) {
        const errorMessage = String(error);
        uiStore.showError(`Failed to create profile: ${errorMessage}`);
        return null;
      }
    },

    // Update a profile
    async updateProfile(id: string, request: CreateProfileRequest): Promise<boolean> {
      try {
        await invoke<ServerProfile>('update_server_profile', { id, request });
        await this.loadProfiles();
        uiStore.showSuccess('Profile updated');
        return true;
      } catch (error) {
        const errorMessage = String(error);
        uiStore.showError(`Failed to update profile: ${errorMessage}`);
        return false;
      }
    },

    // Delete a profile
    async deleteProfile(id: string): Promise<boolean> {
      try {
        await invoke('delete_server_profile', { id });
        await this.loadProfiles();
        uiStore.showSuccess('Profile deleted');
        return true;
      } catch (error) {
        const errorMessage = String(error);
        uiStore.showError(`Failed to delete profile: ${errorMessage}`);
        return false;
      }
    },

    // Get servers in a profile
    async getProfileServers(profileId: string): Promise<ProfileServer[]> {
      try {
        return await invoke<ProfileServer[]>('get_profile_servers', { profileId });
      } catch (error) {
        const errorMessage = String(error);
        uiStore.showError(`Failed to load profile servers: ${errorMessage}`);
        return [];
      }
    },

    // Add a server to a profile
    async addServerToProfile(request: AddServerToProfileRequest): Promise<boolean> {
      try {
        await invoke('add_server_to_profile', { request });
        uiStore.showSuccess('Server added to profile');
        return true;
      } catch (error) {
        const errorMessage = String(error);
        uiStore.showError(`Failed to add server: ${errorMessage}`);
        return false;
      }
    },

    // Remove a server from a profile
    async removeServerFromProfile(profileId: string, serverId: string): Promise<boolean> {
      try {
        await invoke('remove_server_from_profile', { profileId, serverId });
        uiStore.showSuccess('Server removed from profile');
        return true;
      } catch (error) {
        const errorMessage = String(error);
        uiStore.showError(`Failed to remove server: ${errorMessage}`);
        return false;
      }
    },

    // Activate a profile (additive - supports multiple active profiles)
    async activateProfile(profileId: string): Promise<boolean> {
      update((state) => ({ ...state, loading: true }));

      try {
        // ✅ Add timeout to prevent indefinite hang (60 second timeout for profile activation)
        const activationPromise = invoke<ProfileActivation>('activate_profile', { profileId });
        const timeoutPromise = new Promise<never>((_, reject) =>
          setTimeout(() => reject(new Error('Profile activation timed out after 60 seconds')), 60000)
        );

        const activation = await Promise.race([activationPromise, timeoutPromise]);

        // Reload all active profiles
        await this.loadActiveProfiles();

        // ✅ Show success even if some servers failed (partial success is OK)
        const total = activation.success_count + activation.failure_count;

        // Check if already active
        const wasAlreadyActive = activation.errors?.includes('Profile already active') ?? false;

        if (wasAlreadyActive) {
          uiStore.showInfo('Profile is already active');
          return true;
        }

        const successMsg = `Profile activated: ${activation.success_count}/${total} servers connected`;

        if (activation.success_count > 0) {
          uiStore.showSuccess(successMsg);
        } else if (total > 0) {
          uiStore.showWarning(successMsg);
        }

        // ✅ Show errors in a more user-friendly way
        if (activation.errors && activation.errors.length > 0 && !wasAlreadyActive) {
          const errorCount = activation.errors.length;
          const errorSummary = errorCount === 1
            ? '1 server failed to connect'
            : `${errorCount} servers failed to connect`;

          uiStore.showWarning(errorSummary);
          logger.error('Profile activation errors:', activation.errors);

          // Log individual errors without spamming the UI
          activation.errors.forEach((error) => {
            logger.error(`Server connection failed: ${error}`);
          });
        }

        // ✅ Return true even if some servers failed (partial success)
        return activation.success_count > 0 || wasAlreadyActive;
      } catch (error) {
        const errorMessage = String(error);
        logger.error('Failed to activate profile:', error);
        uiStore.showError(`Failed to activate profile: ${errorMessage}`);
        return false;
      } finally {
        // ✅ Always clear loading state, no matter what happens
        update((state) => ({ ...state, loading: false }));
      }
    },

    // Deactivate a specific profile (smart disconnect - only disconnects servers not in other active profiles)
    async deactivateProfile(profileId: string): Promise<boolean> {
      update((state) => ({ ...state, loading: true }));

      try {
        await invoke('deactivate_profile', { profileId });
        await this.loadActiveProfiles();
        uiStore.showSuccess('Profile deactivated');
        return true;
      } catch (error) {
        const errorMessage = String(error);
        uiStore.showError(`Failed to deactivate profile: ${errorMessage}`);
        return false;
      } finally {
        update((state) => ({ ...state, loading: false }));
      }
    },

    // Load all active profiles (multi-profile support)
    async loadActiveProfiles() {
      try {
        const activeProfilesList = await invoke<ActiveProfileState[]>('get_active_profiles');

        // Convert array to Map for easier lookups
        const activeProfiles = new Map<string, ActiveProfileState>();
        activeProfilesList.forEach(profile => {
          if (profile.profile) {
            activeProfiles.set(profile.profile.id, profile);
          }
        });

        update((state) => ({ ...state, activeProfiles }));
        logger.debug(`Loaded ${activeProfiles.size} active profiles`);
      } catch (error) {
        logger.error('Failed to load active profiles:', error);
        update((state) => ({ ...state, activeProfiles: new Map() }));
      }
    },

    // Check if a profile is currently active
    isProfileActive(profileId: string): boolean {
      let isActive = false;
      this.subscribe((state) => {
        isActive = state.activeProfiles.has(profileId);
      })();
      return isActive;
    },

    // Toggle a profile (activate if inactive, deactivate if active)
    async toggleProfile(profileId: string): Promise<boolean> {
      const isActive = this.isProfileActive(profileId);

      if (isActive) {
        return await this.deactivateProfile(profileId);
      } else {
        return await this.activateProfile(profileId);
      }
    },

    // Select a profile
    selectProfile(profileId: string) {
      update((state) => ({ ...state, selectedProfileId: profileId }));
    },

    // Clear selection
    clearSelection() {
      update((state) => ({ ...state, selectedProfileId: undefined }));
    },

    // Set loading state
    setLoading(loading: boolean) {
      update((state) => ({ ...state, loading }));
    },

    // Clear error
    clearError() {
      update((state) => ({ ...state, error: undefined }));
    },
  };
}

export const profileStore = createProfileStore();