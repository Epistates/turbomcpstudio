<script lang="ts">
  import { serverStore } from '$lib/stores/serverStore';
  import { profileStore } from '$lib/stores/profileStore';
  import { Terminal, Activity, Zap, Server, AlertCircle } from 'lucide-svelte';

  const { ontoggleconsole } = $props<{ ontoggleconsole: () => void }>();

  const serverState = $derived($serverStore);
  const profileState = $derived($profileStore);

  // Convert servers Map to array
  const servers = $derived(
    serverState.servers instanceof Map
      ? Array.from(serverState.servers.values())
      : []
  );

  const activeProfile = $derived(profileState.activeProfile);

  // Server status counts
  const connectedCount = $derived(servers.filter((s) => s.status === 'connected').length);
  const connectingCount = $derived(servers.filter((s) => s.status === 'connecting').length);
  const errorCount = $derived(servers.filter((s) => s.status === 'error').length);
  const totalCount = $derived(servers.length);

  // Status color based on health
  const statusColor = $derived(() => {
    if (errorCount > 0) return 'text-red-600 dark:text-red-400';
    if (connectingCount > 0) return 'text-yellow-600 dark:text-yellow-400';
    if (connectedCount > 0) return 'text-green-600 dark:text-green-400';
    return 'text-gray-400 dark:text-gray-500';
  });
</script>

<div
  class="fixed bottom-0 left-0 right-0 h-8 bg-gray-800 dark:bg-gray-950 border-t border-gray-700 dark:border-gray-800 flex items-center justify-between px-4 text-xs text-gray-300 z-50"
>
  <!-- Left: Server Status -->
  <div class="flex items-center gap-4">
    <div class="flex items-center gap-2" title="Server Status">
      <Server size={14} class={statusColor()} />
      <span class="font-mono">
        {connectedCount}/{totalCount}
      </span>
      {#if connectingCount > 0}
        <span class="text-yellow-400">({connectingCount} connecting)</span>
      {/if}
      {#if errorCount > 0}
        <span class="text-red-400">({errorCount} errors)</span>
      {/if}
    </div>

    {#if activeProfile?.profile}
      <div class="flex items-center gap-2 text-blue-400" title="Active Profile">
        <Zap size={14} />
        <span>{activeProfile.profile.name}</span>
      </div>
    {/if}
  </div>

  <!-- Right: Actions -->
  <div class="flex items-center gap-3">
    {#if serverState.error}
      <div class="flex items-center gap-1 text-red-400" title={serverState.error}>
        <AlertCircle size={14} />
        <span class="max-w-xs truncate">{serverState.error}</span>
      </div>
    {/if}

    <button
      onclick={ontoggleconsole}
      class="flex items-center gap-1.5 px-2 py-1 hover:bg-gray-700 dark:hover:bg-gray-800 rounded transition-colors"
      title="Toggle Developer Console"
    >
      <Terminal size={14} />
      <span>Console</span>
    </button>

    <div class="flex items-center gap-1 text-gray-500" title="System Status">
      <Activity size={14} />
      <span>Ready</span>
    </div>
  </div>
</div>
