<!--
  Profile Context Bar Component
  Shows active profile state with switcher and quick actions
  Adapts based on whether a profile is active or not
-->
<script lang="ts">
  import { profileStore, type ServerProfile } from '$lib/stores/profileStore';
  import { serverStore } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import {
    Zap,
    ChevronDown,
    Pause,
    Folder,
    Play
  } from 'lucide-svelte';
  import Button from '../ui/Button.svelte';

  // Props
  const { isMobile = false } = $props();

  // Reactive store access
  const profiles = $derived($profileStore.profiles);
  const activeProfile = $derived($profileStore.activeProfile);
  // ✅ FIXED: Convert Map to array for UI compatibility
  const serverState = $derived($serverStore);
  const servers = $derived(
    serverState.servers instanceof Map
      ? Array.from(serverState.servers.values())
      : []
  );

  // Derived state for active profile display
  const activeProfileDisplay = $derived(() => {
    if (!activeProfile?.profile) return null;

    const profileServers = activeProfile.servers || [];
    const connectedCount = profileServers.filter(ps => {
      const server = servers.find(s => s.id === ps.server_id);
      return server?.status === 'connected';
    }).length;

    const activatedAt = activeProfile.activation?.activated_at;
    const timeAgo = activatedAt ? getTimeAgo(new Date(activatedAt)) : '';

    return {
      profile: activeProfile.profile,
      connectedCount,
      totalCount: profileServers.length,
      activatedAt: timeAgo,
      isActivating: activeProfile.is_activating
    };
  });

  // Profile switcher dropdown state
  let isDropdownOpen = $state(false);
  let dropdownRef: HTMLDivElement | undefined = $state();

  // Close dropdown on outside click
  function handleClickOutside(event: MouseEvent) {
    if (dropdownRef && !dropdownRef.contains(event.target as Node)) {
      isDropdownOpen = false;
    }
  }

  $effect(() => {
    if (isDropdownOpen) {
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  // Actions
  function toggleDropdown() {
    isDropdownOpen = !isDropdownOpen;
  }

  async function handleActivateProfile(profileId: string) {
    isDropdownOpen = false;
    await profileStore.activateProfile(profileId);
  }

  async function handleDeactivateProfile() {
    await profileStore.deactivateProfile();
  }

  function handleNewProfile() {
    isDropdownOpen = false;
    uiStore.openProfileEditor(null); // null = create new profile
  }

  function handleManageProfiles() {
    isDropdownOpen = false;
    uiStore.setView('servers');
  }

  // Time ago helper
  function getTimeAgo(date: Date): string {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));

    if (hours > 0) {
      return `${hours}h ${minutes}m ago`;
    } else {
      return `${minutes}m ago`;
    }
  }

  // Connection status text
  const connectionStatusText = $derived(() => {
    if (!activeProfileDisplay()) return '';
    const display = activeProfileDisplay();
    if (!display) return '';

    if (display.isActivating) {
      return 'Activating...';
    }

    return `${display.connectedCount}/${display.totalCount} servers ${display.connectedCount === display.totalCount ? 'connected' : 'connecting'}`;
  });
</script>

{#if activeProfileDisplay()}
  <!-- Active Profile State -->
  {@const display = activeProfileDisplay()}
  {#if display}
    <div class="profile-context-bar active" bind:this={dropdownRef}>
      <div class="profile-info">
        <span class="profile-icon" title="Active Profile">
          <Zap size={16} />
        </span>
        <div class="profile-details">
          <span class="profile-name">{display.profile.name}</span>
          {#if !isMobile}
            <span class="profile-status">
              {connectionStatusText()}
              {#if display.activatedAt}
                • Activated {display.activatedAt}
              {/if}
            </span>
          {/if}
        </div>
      </div>

      <div class="profile-actions">
        <button
          class="profile-action-btn switch"
          onclick={toggleDropdown}
          title="Switch profile"
        >
          <span>Switch</span>
          <ChevronDown size={14} />
        </button>

        <button
          class="profile-action-btn deactivate"
          onclick={handleDeactivateProfile}
          title="Deactivate profile"
        >
          <Pause size={14} />
          {#if !isMobile}<span>Deactivate</span>{/if}
        </button>
      </div>

      <!-- Profile Switcher Dropdown -->
      {#if isDropdownOpen}
        <div class="profile-dropdown">
          <div class="profile-dropdown-header">
            <span>SWITCH PROFILE</span>
          </div>

          <div class="profile-dropdown-content">
            {#each profiles as profile}
              {@const isActive = profile.id === display.profile.id}
              <button
                class="profile-dropdown-item"
                class:active={isActive}
                onclick={() => !isActive && handleActivateProfile(profile.id)}
                disabled={isActive}
              >
                <div class="profile-item-main">
                  <span class="profile-item-icon">
                    {#if isActive}
                      <Zap size={14} />
                    {:else}
                      {profile.icon || '📁'}
                    {/if}
                  </span>
                  <span class="profile-item-name">
                    {profile.name} {isActive ? '(ACTIVE)' : ''}
                  </span>
                </div>
                <div class="profile-item-meta">
                  <span class="profile-item-count">
                    {isActive ? `🟢 ${display.connectedCount}/${display.totalCount}` : `🔴 0/${profile.server_count}`} servers
                  </span>
                </div>
                {#if profile.description}
                  <div class="profile-item-desc">
                    📁 {profile.description}
                  </div>
                {/if}
              </button>
            {/each}
          </div>

          <div class="profile-dropdown-footer">
            <button class="profile-dropdown-action" onclick={handleNewProfile}>
              + New Profile
            </button>
            <button class="profile-dropdown-action" onclick={handleManageProfiles}>
              📂 Manage Profiles...
            </button>
          </div>
        </div>
      {/if}
    </div>
  {/if}
{:else}
  <!-- No Active Profile State -->
  <div class="profile-context-bar inactive" bind:this={dropdownRef}>
    <span class="profile-empty">No active profile</span>

    <div class="profile-actions">
      <button
        class="profile-action-btn browse"
        onclick={toggleDropdown}
        title="Browse profiles"
      >
        <Folder size={14} />
        <span>Browse Profiles</span>
        <ChevronDown size={14} />
      </button>
    </div>

    <!-- Profile Switcher Dropdown -->
    {#if isDropdownOpen}
      <div class="profile-dropdown">
        <div class="profile-dropdown-header">
          <span>ACTIVATE PROFILE</span>
        </div>

        <div class="profile-dropdown-content">
          {#if profiles.length > 0}
            {#each profiles as profile}
              <button
                class="profile-dropdown-item"
                onclick={() => handleActivateProfile(profile.id)}
              >
                <div class="profile-item-main">
                  <span class="profile-item-icon">
                    {profile.icon || '📁'}
                  </span>
                  <span class="profile-item-name">
                    {profile.name}
                  </span>
                </div>
                <div class="profile-item-meta">
                  <span class="profile-item-count">
                    🔴 0/{profile.server_count} servers
                  </span>
                </div>
                {#if profile.description}
                  <div class="profile-item-desc">
                    📁 {profile.description}
                  </div>
                {/if}
              </button>
            {/each}
          {:else}
            <div class="profile-dropdown-empty">
              <p>No profiles created yet</p>
              <button class="profile-dropdown-action" onclick={handleNewProfile}>
                Create your first profile
              </button>
            </div>
          {/if}
        </div>

        <div class="profile-dropdown-footer">
          <button class="profile-dropdown-action" onclick={handleNewProfile}>
            + New Profile
          </button>
          <button class="profile-dropdown-action" onclick={handleManageProfiles}>
            📂 Manage Profiles...
          </button>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  /* Profile Context Bar */
  .profile-context-bar {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    font-size: var(--mcp-text-sm);
    min-width: 280px;
  }

  .profile-context-bar.active {
    border-color: var(--mcp-success);
    background: var(--mcp-success-subtle);
  }

  .profile-context-bar.inactive {
    border-color: var(--mcp-border-secondary);
  }

  /* Profile Info */
  .profile-info {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    flex: 1;
    min-width: 0;
  }

  .profile-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--mcp-success);
    flex-shrink: 0;
  }

  .profile-details {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-0-5);
    min-width: 0;
  }

  .profile-name {
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .profile-status {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .profile-empty {
    color: var(--mcp-text-secondary);
    font-size: var(--mcp-text-sm);
  }

  /* Profile Actions */
  .profile-actions {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    flex-shrink: 0;
  }

  .profile-action-btn {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-1-5);
    padding: var(--mcp-space-1) var(--mcp-space-2);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-primary);
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-medium);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .profile-action-btn:hover {
    background: var(--mcp-surface-hover);
    border-color: var(--mcp-border-hover);
  }

  .profile-action-btn.deactivate {
    color: var(--mcp-warning);
    border-color: var(--mcp-warning);
  }

  .profile-action-btn.deactivate:hover {
    background: var(--mcp-warning-subtle);
  }

  /* Profile Dropdown */
  .profile-dropdown {
    position: absolute;
    top: calc(100% + var(--mcp-space-2));
    right: 0;
    min-width: 320px;
    max-width: 400px;
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    box-shadow: var(--mcp-shadow-xl);
    z-index: 1000;
    max-height: 500px;
    display: flex;
    flex-direction: column;
  }

  .profile-dropdown-header {
    padding: var(--mcp-space-3);
    border-bottom: 1px solid var(--mcp-border-primary);
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-bold);
    color: var(--mcp-text-secondary);
    letter-spacing: 0.05em;
  }

  .profile-dropdown-content {
    overflow-y: auto;
    max-height: 360px;
  }

  .profile-dropdown-item {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-1);
    padding: var(--mcp-space-3);
    border: none;
    border-bottom: 1px solid var(--mcp-border-secondary);
    background: transparent;
    text-align: left;
    cursor: pointer;
    transition: background 0.2s ease;
    width: 100%;
  }

  .profile-dropdown-item:hover:not(:disabled) {
    background: var(--mcp-surface-hover);
  }

  .profile-dropdown-item.active {
    background: var(--mcp-success-subtle);
    cursor: default;
  }

  .profile-item-main {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
  }

  .profile-item-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: var(--mcp-text-base);
  }

  .profile-item-name {
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    font-size: var(--mcp-text-sm);
  }

  .profile-item-meta {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-secondary);
    padding-left: var(--mcp-space-6);
  }

  .profile-item-desc {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    padding-left: var(--mcp-space-6);
  }

  .profile-dropdown-empty {
    padding: var(--mcp-space-6);
    text-align: center;
    color: var(--mcp-text-secondary);
  }

  .profile-dropdown-empty p {
    margin-bottom: var(--mcp-space-3);
  }

  .profile-dropdown-footer {
    padding: var(--mcp-space-2);
    border-top: 1px solid var(--mcp-border-primary);
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-1);
  }

  .profile-dropdown-action {
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-secondary);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    text-align: left;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .profile-dropdown-action:hover {
    background: var(--mcp-surface-hover);
    border-color: var(--mcp-border-primary);
    color: var(--mcp-text-primary);
  }

  /* Mobile Styles */
  @media (max-width: 768px) {
    .profile-context-bar {
      min-width: auto;
      max-width: 200px;
    }

    .profile-action-btn span {
      display: none;
    }

    .profile-dropdown {
      right: auto;
      left: 0;
      width: 100vw;
      max-width: 100vw;
      border-radius: 0;
    }
  }
</style>
