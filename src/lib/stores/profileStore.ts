import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { uiStore } from './uiStore';

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
  activeProfile: ActiveProfileState | null;
  selectedProfileId?: string;
  loading: boolean;
  error?: string;
}

const initialState: ProfileStoreState = {
  profiles: [],
  activeProfile: null,
  selectedProfileId: undefined,
  loading: false,
  error: undefined,
};

// ============================================================================
// STORE
// ============================================================================

function createProfileStore() {
  const { subscribe, set, update } = writable<ProfileStoreState>(initialState);

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

    // Activate a profile
    async activateProfile(profileId: string): Promise<boolean> {
      update((state) => ({ ...state, loading: true }));

      try {
        const activation = await invoke<ProfileActivation>('activate_profile', { profileId });

        // Reload active profile state
        await this.loadActiveProfile();

        const successMsg = `Profile activated: ${activation.success_count}/${activation.success_count + activation.failure_count} servers connected`;
        uiStore.showSuccess(successMsg);

        // Show errors if any
        if (activation.errors && activation.errors.length > 0) {
          activation.errors.forEach((error) => {
            uiStore.showError(`Server Connection Failed: ${error}`);
          });
        }

        return true;
      } catch (error) {
        const errorMessage = String(error);
        uiStore.showError(`Failed to activate profile: ${errorMessage}`);
        return false;
      } finally {
        update((state) => ({ ...state, loading: false }));
      }
    },

    // Deactivate the current profile
    async deactivateProfile(): Promise<boolean> {
      update((state) => ({ ...state, loading: true }));

      try {
        await invoke('deactivate_profile');
        await this.loadActiveProfile();
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

    // Load active profile state
    async loadActiveProfile() {
      try {
        const activeProfile = await invoke<ActiveProfileState | null>('get_active_profile');
        update((state) => ({ ...state, activeProfile }));
      } catch (error) {
        console.error('Failed to load active profile:', error);
        update((state) => ({ ...state, activeProfile: null }));
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