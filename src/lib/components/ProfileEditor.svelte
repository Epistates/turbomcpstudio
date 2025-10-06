<script lang="ts">
  import { onMount } from 'svelte';
  import { profileStore, type CreateProfileRequest, type ProfileServer, type ServerProfile } from '$lib/stores/profileStore';
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import Button from './ui/Button.svelte';
  import {
    X,
    Save,
    Plus,
    Trash2,
    GripVertical,
    Settings as SettingsIcon,
    Check,
  } from 'lucide-svelte';

  // Props
  const {
    profileId = null,
    onClose = () => {}
  }: {
    profileId?: string | null;
    onClose?: () => void;
  } = $props();

  // Mode
  const isEditMode = $derived(!!profileId);

  // Form state
  let name = $state('');
  let description = $state('');
  let icon = $state('📁');
  let color = $state('#3b82f6');
  let autoActivate = $state(false);

  // Server management
  let profileServers = $state<ProfileServer[]>([]);
  let availableServers = $state<ServerInfo[]>([]);
  let showAddServerDropdown = $state(false);
  let expandedServerId = $state<string | null>(null);

  // Loading state
  let loading = $state(false);
  let saving = $state(false);

  // Common icons for quick selection
  const iconOptions = ['📁', '🤖', '🧪', '🎯', '🎨', '📊', '🔍', '⚡', '🚀', '🔧', '💼', '🏢'];

  // Color presets
  const colorOptions = [
    { name: 'Blue', value: '#3b82f6' },
    { name: 'Green', value: '#10b981' },
    { name: 'Purple', value: '#8b5cf6' },
    { name: 'Red', value: '#ef4444' },
    { name: 'Orange', value: '#f59e0b' },
    { name: 'Pink', value: '#ec4899' },
    { name: 'Indigo', value: '#6366f1' },
    { name: 'Teal', value: '#14b8a6' },
  ];

  onMount(async () => {
    loading = true;

    // Load available servers
    const serverState = $serverStore;
    availableServers = serverState.servers;

    // If editing, load profile data
    if (isEditMode && profileId) {
      try {
        // Load profile details
        const profiles = $profileStore.profiles;
        const profile = profiles.find(p => p.id === profileId);

        if (profile) {
          name = profile.name;
          description = profile.description || '';
          icon = profile.icon || '📁';
          color = profile.color || '#3b82f6';
          autoActivate = profile.auto_activate;
        }

        // Load profile servers
        profileServers = await profileStore.getProfileServers(profileId);
      } catch (error) {
        console.error('Failed to load profile:', error);
      }
    }

    loading = false;
  });

  async function handleSave() {
    if (!name.trim()) {
      alert('Profile name is required');
      return;
    }

    saving = true;

    try {
      const request: CreateProfileRequest = {
        name: name.trim(),
        description: description.trim() || undefined,
        icon: icon || undefined,
        color: color || undefined,
        auto_activate: autoActivate,
      };

      let savedProfileId: string;

      if (isEditMode && profileId) {
        // Update existing profile
        const success = await profileStore.updateProfile(profileId, request);
        if (!success) {
          saving = false;
          return;
        }
        savedProfileId = profileId;
      } else {
        // Create new profile
        const profile = await profileStore.createProfile(request);
        if (!profile) {
          saving = false;
          return;
        }
        savedProfileId = profile.id;
      }

      // Save server assignments
      // For simplicity, we'll delete all and re-add (could optimize with diff)
      if (isEditMode && profileId) {
        // Remove all existing servers
        const existingServers = await profileStore.getProfileServers(profileId);
        for (const server of existingServers) {
          await profileStore.removeServerFromProfile(profileId, server.server_id);
        }
      }

      // Add all configured servers
      for (const server of profileServers) {
        await profileStore.addServerToProfile({
          profile_id: savedProfileId,
          server_id: server.server_id,
          startup_order: server.startup_order,
          startup_delay_ms: server.startup_delay_ms,
          auto_connect: server.auto_connect,
          auto_restart: server.auto_restart,
          required: server.required,
          environment_overrides: server.environment_overrides,
        });
      }

      // Reload profiles
      await profileStore.loadProfiles();

      onClose();
    } catch (error) {
      console.error('Failed to save profile:', error);
    } finally {
      saving = false;
    }
  }

  function handleAddServer(serverId: string) {
    const server = availableServers.find(s => s.id === serverId);
    if (!server) return;

    // Check if already added
    if (profileServers.some(ps => ps.server_id === serverId)) {
      alert('Server already added to this profile');
      return;
    }

    // Add server with default configuration
    const newServer: ProfileServer = {
      profile_id: profileId || '',
      server_id: serverId,
      server_name: server.config.name,
      server_description: server.config.description,
      transport_type: server.config.transport_config.type,
      startup_order: profileServers.length,
      startup_delay_ms: 0,
      auto_connect: true,
      auto_restart: false,
      required: false,
      environment_overrides: undefined,
      created_at: new Date().toISOString(),
    };

    profileServers = [...profileServers, newServer];
    showAddServerDropdown = false;
  }

  function handleRemoveServer(serverId: string) {
    profileServers = profileServers.filter(s => s.server_id !== serverId);
    // Reorder remaining servers
    profileServers = profileServers.map((s, index) => ({
      ...s,
      startup_order: index,
    }));
  }

  function handleMoveUp(index: number) {
    if (index === 0) return;
    const newServers = [...profileServers];
    [newServers[index - 1], newServers[index]] = [newServers[index], newServers[index - 1]];
    // Update startup_order
    newServers[index - 1].startup_order = index - 1;
    newServers[index].startup_order = index;
    profileServers = newServers;
  }

  function handleMoveDown(index: number) {
    if (index === profileServers.length - 1) return;
    const newServers = [...profileServers];
    [newServers[index], newServers[index + 1]] = [newServers[index + 1], newServers[index]];
    // Update startup_order
    newServers[index].startup_order = index;
    newServers[index + 1].startup_order = index + 1;
    profileServers = newServers;
  }

  function updateServerConfig(serverId: string, updates: Partial<ProfileServer>) {
    profileServers = profileServers.map(s =>
      s.server_id === serverId ? { ...s, ...updates } : s
    );
  }

  function toggleServerExpanded(serverId: string) {
    expandedServerId = expandedServerId === serverId ? null : serverId;
  }
</script>

<!-- Modal Overlay -->
<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4" onclick={onClose}>
  <!-- Modal Content -->
  <div
    class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] flex flex-col"
    onclick={(e) => e.stopPropagation()}
  >
    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
      <h2 class="text-xl font-bold text-gray-900 dark:text-white">
        {isEditMode ? 'Edit Profile' : 'Create Profile'}
      </h2>
      <button
        onclick={onClose}
        class="p-2 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
      >
        <X size={20} />
      </button>
    </div>

    {#if loading}
      <div class="flex-1 flex items-center justify-center p-12">
        <div class="text-center">
          <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <p class="text-sm text-gray-600 dark:text-gray-400">Loading profile...</p>
        </div>
      </div>
    {:else}
      <!-- Body -->
      <div class="flex-1 overflow-y-auto p-6 space-y-6">
        <!-- Profile Information -->
        <div>
          <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-3">Profile Information</h3>

          <div class="space-y-4">
            <!-- Name -->
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Name *
              </label>
              <input
                type="text"
                bind:value={name}
                placeholder="e.g., AI Development, Production, Testing"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <!-- Description -->
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Description
              </label>
              <textarea
                bind:value={description}
                placeholder="Optional description for this profile..."
                rows="2"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
              ></textarea>
            </div>

            <!-- Icon & Color -->
            <div class="grid grid-cols-2 gap-4">
              <!-- Icon Picker -->
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Icon
                </label>
                <div class="flex flex-wrap gap-2">
                  {#each iconOptions as iconOption}
                    <button
                      type="button"
                      onclick={() => icon = iconOption}
                      class={`w-10 h-10 flex items-center justify-center text-2xl border-2 rounded-lg transition-colors ${
                        icon === iconOption
                          ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                          : 'border-gray-300 dark:border-gray-600'
                      }`}
                    >
                      {iconOption}
                    </button>
                  {/each}
                </div>
              </div>

              <!-- Color Picker -->
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Color
                </label>
                <div class="flex flex-wrap gap-2">
                  {#each colorOptions as colorOption}
                    <button
                      type="button"
                      onclick={() => color = colorOption.value}
                      class="w-10 h-10 rounded-lg border-2 transition-all"
                      style="background-color: {colorOption.value}"
                      class:border-gray-900={color === colorOption.value}
                      class:dark:border-white={color === colorOption.value}
                      class:scale-110={color === colorOption.value}
                      class:border-gray-300={color !== colorOption.value}
                      class:dark:border-gray-600={color !== colorOption.value}
                      title={colorOption.name}
                    ></button>
                  {/each}
                </div>
              </div>
            </div>

            <!-- Auto-activate -->
            <div class="flex items-center gap-2">
              <input
                type="checkbox"
                id="auto-activate"
                bind:checked={autoActivate}
                class="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
              />
              <label for="auto-activate" class="text-sm text-gray-700 dark:text-gray-300">
                Auto-activate on startup
              </label>
            </div>
          </div>
        </div>

        <!-- Servers -->
        <div>
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
              Servers ({profileServers.length})
            </h3>
            <div class="relative">
              <Button
                variant="secondary"
                size="sm"
                leftIcon={Plus}
                onclick={() => showAddServerDropdown = !showAddServerDropdown}
              >
                Add Server
              </Button>

              {#if showAddServerDropdown}
                <div class="absolute right-0 mt-2 w-64 bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg shadow-lg z-10 max-h-64 overflow-y-auto">
                  {#if availableServers.length === 0}
                    <div class="p-4 text-sm text-gray-500 dark:text-gray-400 text-center">
                      No servers available
                    </div>
                  {:else}
                    {#each availableServers as server}
                      <button
                        type="button"
                        onclick={() => handleAddServer(server.id)}
                        class="w-full px-4 py-2 text-left text-sm hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors"
                        disabled={profileServers.some(ps => ps.server_id === server.id)}
                      >
                        <div class="font-medium text-gray-900 dark:text-white">
                          {server.config.name}
                        </div>
                        {#if server.config.description}
                          <div class="text-xs text-gray-500 dark:text-gray-400 truncate">
                            {server.config.description}
                          </div>
                        {/if}
                      </button>
                    {/each}
                  {/if}
                </div>
              {/if}
            </div>
          </div>

          {#if profileServers.length === 0}
            <div class="text-center py-8 border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-lg">
              <p class="text-sm text-gray-500 dark:text-gray-400">
                No servers added yet. Click "Add Server" to get started.
              </p>
            </div>
          {:else}
            <div class="space-y-2">
              {#each profileServers as server, index}
                <div class="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                  <!-- Server Header -->
                  <div class="flex items-center gap-2 p-3 bg-gray-50 dark:bg-gray-700/50">
                    <button
                      type="button"
                      class="cursor-grab p-1 text-gray-400 hover:text-gray-600"
                      title="Drag to reorder"
                    >
                      <GripVertical size={16} />
                    </button>

                    <div class="flex-1">
                      <div class="font-medium text-sm text-gray-900 dark:text-white">
                        {index + 1}. {server.server_name}
                      </div>
                      <div class="text-xs text-gray-500 dark:text-gray-400">
                        {server.transport_type}
                        {#if server.startup_delay_ms > 0}
                          • Delay: {server.startup_delay_ms}ms
                        {/if}
                      </div>
                    </div>

                    <!-- Server Badges -->
                    <div class="flex items-center gap-1">
                      {#if server.required}
                        <span class="px-2 py-0.5 text-xs font-medium bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-200 rounded">
                          Required
                        </span>
                      {/if}
                      {#if server.auto_restart}
                        <span class="px-2 py-0.5 text-xs font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-200 rounded">
                          Auto-restart
                        </span>
                      {/if}
                    </div>

                    <!-- Actions -->
                    <button
                      type="button"
                      onclick={() => toggleServerExpanded(server.server_id)}
                      class="p-1 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                      title="Configure"
                    >
                      <SettingsIcon size={16} />
                    </button>

                    <button
                      type="button"
                      onclick={() => handleMoveUp(index)}
                      disabled={index === 0}
                      class="p-1 text-gray-500 hover:text-gray-700 disabled:opacity-30"
                      title="Move up"
                    >
                      ▲
                    </button>

                    <button
                      type="button"
                      onclick={() => handleMoveDown(index)}
                      disabled={index === profileServers.length - 1}
                      class="p-1 text-gray-500 hover:text-gray-700 disabled:opacity-30"
                      title="Move down"
                    >
                      ▼
                    </button>

                    <button
                      type="button"
                      onclick={() => handleRemoveServer(server.server_id)}
                      class="p-1 text-red-500 hover:text-red-700"
                      title="Remove"
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>

                  <!-- Server Configuration (Expandable) -->
                  {#if expandedServerId === server.server_id}
                    <div class="p-4 bg-white dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700 space-y-3">
                      <!-- Startup Delay -->
                      <div>
                        <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                          Startup Delay (ms)
                        </label>
                        <input
                          type="number"
                          value={server.startup_delay_ms}
                          oninput={(e) => updateServerConfig(server.server_id, { startup_delay_ms: parseInt(e.currentTarget.value) || 0 })}
                          min="0"
                          step="100"
                          class="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                        />
                      </div>

                      <!-- Checkboxes -->
                      <div class="space-y-2">
                        <label class="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={server.auto_connect}
                            onchange={(e) => updateServerConfig(server.server_id, { auto_connect: e.currentTarget.checked })}
                            class="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                          />
                          <span class="text-sm text-gray-700 dark:text-gray-300">Auto-connect</span>
                        </label>

                        <label class="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={server.auto_restart}
                            onchange={(e) => updateServerConfig(server.server_id, { auto_restart: e.currentTarget.checked })}
                            class="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                          />
                          <span class="text-sm text-gray-700 dark:text-gray-300">Auto-restart on failure</span>
                        </label>

                        <label class="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={server.required}
                            onchange={(e) => updateServerConfig(server.server_id, { required: e.currentTarget.checked })}
                            class="w-4 h-4 text-red-600 border-gray-300 rounded focus:ring-red-500"
                          />
                          <span class="text-sm text-gray-700 dark:text-gray-300">Required (profile fails if this server fails)</span>
                        </label>
                      </div>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>

            <div class="mt-3 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
              <p class="text-xs text-blue-800 dark:text-blue-200">
                💡 Servers with the same startup order will connect concurrently. Different startup orders connect sequentially.
              </p>
            </div>
          {/if}
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
        <Button variant="secondary" onclick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button
          variant="primary"
          leftIcon={Save}
          onclick={handleSave}
          disabled={saving || !name.trim()}
          loading={saving}
        >
          {saving ? 'Saving...' : (isEditMode ? 'Save Changes' : 'Create Profile')}
        </Button>
      </div>
    {/if}
  </div>
</div>