<!--
  Minimal Profile Status Component

  Shows active profile status in the sidebar - for awareness, not interaction.
  Clicking navigates to Server Manager for full profile/server management.

  Design Philosophy:
  - Minimal footprint (small indicator at sidebar bottom)
  - Just shows status (not an interactive selector)
  - One action: "Go to Server Manager"
-->
<script lang="ts">
  import { profileStore } from '$lib/stores/profileStore';
  import { serverStore } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import { Zap, Server, ChevronRight } from 'lucide-svelte';

  // Reactive profile state
  const activeProfile = $derived(Array.from($profileStore.activeProfiles.values())[0]);
  const servers = $derived(
    $serverStore.servers instanceof Map
      ? Array.from($serverStore.servers.values())
      : []
  );

  // Calculate connection status
  const connectionStatus = $derived(() => {
    if (!activeProfile?.profile || !activeProfile.servers) {
      return { connected: 0, total: 0, allConnected: false };
    }

    const profileServerIds = new Set(activeProfile.servers.map(ps => ps.server_id));
    const profileServers = servers.filter(s => profileServerIds.has(s.id));
    const connectedCount = profileServers.filter(s => s.status === 'connected').length;

    return {
      connected: connectedCount,
      total: profileServers.length,
      allConnected: connectedCount > 0 && connectedCount === profileServers.length,
    };
  });

  // Navigate to Server Manager
  function goToServerManager() {
    uiStore.setView('servers');
  }
</script>

{#if activeProfile?.profile}
  <!-- Active Profile State -->
  <button class="profile-status-mini active" onclick={goToServerManager}>
    <div class="profile-indicator">
      <span class="status-icon" class:all-connected={connectionStatus().allConnected}>
        {#if connectionStatus().allConnected}
          <Zap size={14} />
        {:else}
          <Server size={14} />
        {/if}
      </span>
      <div class="profile-details">
        <span class="profile-name">{activeProfile.profile.name}</span>
        <span class="profile-status">
          {connectionStatus().connected}/{connectionStatus().total} connected
        </span>
      </div>
    </div>
    <ChevronRight size={14} class="nav-icon" />
  </button>
{:else}
  <!-- No Active Profile State -->
  <button class="profile-status-mini inactive" onclick={goToServerManager}>
    <div class="profile-indicator">
      <span class="status-icon inactive-icon">
        <Server size={14} />
      </span>
      <div class="profile-details">
        <span class="profile-name">No Active Profile</span>
        <span class="profile-status">Manage servers →</span>
      </div>
    </div>
  </button>
{/if}

<style>
  .profile-status-mini {
    width: 100%;
    padding: var(--mcp-space-3);
    background: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--mcp-space-2);
  }

  .profile-status-mini:hover {
    background: var(--mcp-surface-hover);
    border-color: var(--mcp-border-hover);
  }

  .profile-status-mini.active {
    border-color: var(--mcp-success);
    background: var(--mcp-success-subtle);
  }

  .profile-status-mini.active:hover {
    background: var(--mcp-success-hover);
  }

  .profile-indicator {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    flex: 1;
    min-width: 0;
  }

  .status-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--mcp-text-tertiary);
  }

  .status-icon.all-connected {
    color: var(--mcp-success);
  }

  .status-icon.inactive-icon {
    color: var(--mcp-text-quaternary);
  }

  .profile-details {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-0-5);
    min-width: 0;
    flex: 1;
  }

  .profile-name {
    font-size: var(--mcp-text-sm);
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

  .nav-icon {
    color: var(--mcp-text-tertiary);
    flex-shrink: 0;
  }

  /* Mobile adjustments */
  @media (max-width: 768px) {
    .profile-status-mini {
      padding: var(--mcp-space-2);
    }

    .profile-name {
      font-size: var(--mcp-text-xs);
    }

    .profile-status {
      font-size: 10px;
    }
  }
</style>
