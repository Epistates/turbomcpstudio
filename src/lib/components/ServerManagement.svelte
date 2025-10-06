<script lang="ts">
  import { onMount } from 'svelte';
  import { profileStore } from '$lib/stores/profileStore';
  import { uiStore } from '$lib/stores/uiStore';
  import Button from './ui/Button.svelte';
  import ProfileEditor from './ProfileEditor.svelte';
  import {
    Plus,
    Play,
    Square,
    Edit,
    Trash2,
    CheckCircle,
    Server,
    Clock,
    AlertCircle,
  } from 'lucide-svelte';

  // Reactive store subscriptions using Svelte 5 runes
  const activeProfile = $derived($profileStore.activeProfile);
  const profiles = $derived($profileStore.profiles);
  const loading = $derived($profileStore.loading);

  // Modal state
  let showProfileEditor = $state(false);
  let editingProfile = $state<string | null>(null);

  onMount(async () => {
    await profileStore.loadProfiles();
    await profileStore.loadActiveProfile();
  });

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

  async function handleDeleteProfile(profileId: string, profileName: string) {
    if (confirm(`Are you sure you want to delete profile "${profileName}"?`)) {
      await profileStore.deleteProfile(profileId);
    }
  }

  function handleCreateProfile() {
    editingProfile = null;
    showProfileEditor = true;
  }

  function handleEditProfile(profileId: string) {
    editingProfile = profileId;
    showProfileEditor = true;
  }

  function formatTimestamp(timestamp: string): string {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;

    return date.toLocaleDateString();
  }

  function getProfileIcon(icon?: string): string {
    return icon || '📁';
  }
</script>

<div class="h-full flex flex-col bg-gray-50 dark:bg-gray-900">
  <!-- Header -->
  <div class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Server Profiles</h1>
        <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
          Manage groups of MCP servers for different environments
        </p>
      </div>
      <Button variant="primary" leftIcon={Plus} onclick={handleCreateProfile}>
        New Profile
      </Button>
    </div>
  </div>

  <!-- Active Profile Banner -->
  {#if activeProfile?.profile}
    <div class="bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20 border-b border-blue-200 dark:border-blue-800">
      <div class="px-6 py-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <CheckCircle size={20} class="text-green-600" />
            <div>
              <h2 class="text-sm font-semibold text-gray-900 dark:text-white">
                Active Profile: {activeProfile.profile.name}
              </h2>
              <p class="text-xs text-gray-600 dark:text-gray-400">
                {activeProfile.servers.length} server{activeProfile.servers.length !== 1 ? 's' : ''} •
                {#if activeProfile.activation}
                  {activeProfile.activation.success_count}/{activeProfile.activation.success_count + activeProfile.activation.failure_count} connected
                  • Activated {formatTimestamp(activeProfile.activation.activated_at)}
                {:else}
                  Ready
                {/if}
              </p>
            </div>
          </div>
          <Button variant="secondary" leftIcon={Square} onclick={handleDeactivateProfile} disabled={loading}>
            Deactivate
          </Button>
        </div>

        <!-- Show errors if any -->
        {#if activeProfile.activation?.errors && activeProfile.activation.errors.length > 0}
          <div class="mt-3 p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
            <div class="flex items-start gap-2">
              <AlertCircle size={16} class="text-amber-600 mt-0.5 flex-shrink-0" />
              <div class="text-xs text-amber-800 dark:text-amber-200">
                <strong>Connection Issues:</strong>
                <ul class="mt-1 space-y-1">
                  {#each activeProfile.activation.errors as error}
                    <li>• {error}</li>
                  {/each}
                </ul>
              </div>
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Main Content -->
  <div class="flex-1 overflow-y-auto p-6">
    {#if loading && profiles.length === 0}
      <div class="flex items-center justify-center h-64">
        <div class="text-center">
          <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <p class="text-sm text-gray-600 dark:text-gray-400">Loading profiles...</p>
        </div>
      </div>
    {:else if profiles.length === 0}
      <div class="flex items-center justify-center h-64">
        <div class="text-center max-w-md">
          <Server size={48} class="mx-auto text-gray-400 mb-4" />
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">
            No Profiles Yet
          </h3>
          <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
            Create your first profile to group MCP servers together for quick environment switching.
          </p>
          <Button variant="primary" leftIcon={Plus} onclick={handleCreateProfile}>
            Create Profile
          </Button>
        </div>
      </div>
    {:else}
      <!-- Profile Grid -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {#each profiles as profile}
          {@const isActive = activeProfile?.profile?.id === profile.id}
          <div
            class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:shadow-lg transition-shadow"
            class:ring-2={isActive}
            class:ring-blue-500={isActive}
          >
            <!-- Profile Header -->
            <div class="flex items-start justify-between mb-3">
              <div class="flex items-center gap-2">
                <span class="text-2xl">{getProfileIcon(profile.icon)}</span>
                <div>
                  <h3 class="font-semibold text-gray-900 dark:text-white">
                    {profile.name}
                  </h3>
                  {#if isActive}
                    <span class="text-xs text-green-600 dark:text-green-400 flex items-center gap-1">
                      <CheckCircle size={12} />
                      Active
                    </span>
                  {/if}
                </div>
              </div>

              <!-- Actions Menu -->
              <div class="flex items-center gap-1">
                <button
                  onclick={() => handleEditProfile(profile.id)}
                  class="p-2 text-gray-500 hover:text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded transition-colors"
                  title="Edit profile"
                >
                  <Edit size={16} />
                </button>
                <button
                  onclick={() => handleDeleteProfile(profile.id, profile.name)}
                  class="p-2 text-gray-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition-colors"
                  title="Delete profile"
                  disabled={isActive}
                >
                  <Trash2 size={16} />
                </button>
              </div>
            </div>

            <!-- Profile Description -->
            {#if profile.description}
              <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">
                {profile.description}
              </p>
            {/if}

            <!-- Profile Stats -->
            <div class="flex items-center justify-between text-xs text-gray-500 dark:text-gray-400 mb-3">
              <span class="flex items-center gap-1">
                <Server size={12} />
                {profile.server_count} server{profile.server_count !== 1 ? 's' : ''}
              </span>
              <span class="flex items-center gap-1">
                <Clock size={12} />
                {formatTimestamp(profile.updated_at)}
              </span>
            </div>

            <!-- Activate Button -->
            {#if !isActive}
              <Button
                variant="primary"
                size="sm"
                leftIcon={Play}
                onclick={() => handleActivateProfile(profile.id)}
                disabled={loading}
                class="w-full"
              >
                Activate
              </Button>
            {:else}
              <Button
                variant="secondary"
                size="sm"
                leftIcon={Square}
                onclick={handleDeactivateProfile}
                disabled={loading}
                class="w-full"
              >
                Deactivate
              </Button>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<!-- Profile Editor Modal -->
{#if showProfileEditor}
  <ProfileEditor
    profileId={editingProfile}
    onClose={() => {
      showProfileEditor = false;
      editingProfile = null;
    }}
  />
{/if}