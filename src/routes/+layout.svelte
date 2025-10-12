<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { listen, once } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { themeStore } from '$lib/stores/themeStore';
  import { serverStore } from '$lib/stores/serverStore';
  import { profileStore } from '$lib/stores/profileStore';
  import { appStore, appStoreIsReady, appStoreState, type AppState } from '$lib/stores/appStore';
  import AppLoadingScreen from '$lib/components/AppLoadingScreen.svelte';
  import ElicitationDialog from '$lib/components/ElicitationDialog.svelte';
  import type { JsonSchema } from '$lib/utils/schemaValidation';

  // Svelte 5 snippet props
  const { children } = $props();

  // Elicitation state
  interface ElicitationRequest {
    id: string;
    protocolMessageId?: string;
    serverId: string;
    serverName?: string;
    message: string;
    requestedSchema: JsonSchema;
  }

  let elicitationDialogVisible = $state(false);
  let elicitationRequest = $state<ElicitationRequest | null>(null);

  // Reactive state using Svelte 5 runes - access store values directly
  let isAppReady = $state(false);
  let currentAppState = $state<AppState>({
    isInitializing: true,
    databaseReady: false,
    mcpManagerReady: false,
    initializationError: null,
    startupTime: null
  });

  // Subscribe to store changes and update reactive state
  $effect(() => {
    const unsubscribeReady = appStoreIsReady.subscribe(value => {
      isAppReady = value;
    });
    const unsubscribeState = appStoreState.subscribe(value => {
      currentAppState = value;
    });

    return () => {
      unsubscribeReady();
      unsubscribeState();
    };
  });

  // Track cleanup functions
  let mcpUnlisten: (() => void) | null = null;
  let elicitationUnlisten: (() => void) | null = null;
  let appReadyReceived = false;
  let initializationStartTime = Date.now();

  // Elicitation event handlers
  async function handleElicitationResponse(data: any) {
    if (!elicitationRequest) return;

    try {
      await invoke('send_elicitation_response', {
        serverId: elicitationRequest.serverId,
        requestId: elicitationRequest.id,
        responseData: data
      });
      console.log('✅ Elicitation response sent successfully');
    } catch (err) {
      console.error('❌ Failed to send elicitation response:', err);
    } finally {
      elicitationDialogVisible = false;
      elicitationRequest = null;
    }
  }

  function handleElicitationClose() {
    elicitationDialogVisible = false;
    elicitationRequest = null;
  }

  // Set up event listeners in onMount to ensure they're registered before backend emits events
  // This prevents race conditions where events are emitted before listeners are ready

  // Initialize app systems on DOM ready
  onMount(async () => {
    // Initialize theme system (fast, synchronous)
    themeStore.init();

    // Set up all event listeners FIRST, before any backend operations
    // This ensures listeners are registered before backend emits events
    console.log('🎧 Setting up event listeners...');

    mcpUnlisten = await listen('mcp-event', (event) => {
      serverStore.handleMcpEvent(event.payload);
    });

    elicitationUnlisten = await listen('elicitation_requested', (event: any) => {
      console.log('🔔 Received elicitation_requested event:', event.payload);
      const payload = event.payload;
      elicitationRequest = {
        id: payload.id,
        protocolMessageId: payload.protocolMessageId,
        serverId: payload.serverId || 'unknown',
        serverName: payload.serverName,
        message: payload.message,
        requestedSchema: payload.requestedSchema
      };
      elicitationDialogVisible = true;
    });

    await listen('app-early-ready', (event) => {
      console.log('🟢 Received app-early-ready event');
      appStore.setMcpManagerReady(true);
    });

    await listen('app-ready', async (event) => {
      if (appReadyReceived) {
        console.log('⏭️ app-ready already received, skipping');
        return;
      }
      appReadyReceived = true;
      console.log('✅ Received app-ready event, completing initialization');
      appStore.setDatabaseReady(true);

      try {
        // Retry server initialization after database is ready
        await serverStore.initialize();
        appStore.markStepCompleted('servers');

        // Load profiles and active profile state
        await profileStore.loadProfiles();
        await profileStore.loadActiveProfile();

        appStore.completeInitialization();

        const totalTime = Date.now() - initializationStartTime;
        console.log(`🎉 Full initialization completed in ${totalTime}ms`);
      } catch (err) {
        console.error('❌ FRONTEND: Failed to initialize servers after database ready:', err);
        appStore.markStepError('servers', err instanceof Error ? err.message : 'Unknown error');
        appStore.setInitializationError('Failed to load server configurations');
      }
    });

    console.log('✅ All event listeners registered');

    // Mark servers step as loading
    appStore.markStepLoading('servers');

    // Attempt initial server load (non-blocking, might fail if database not ready)
    serverStore.initialize().then(async () => {
      // Don't complete here - wait for app-ready event for proper sequencing
      // Also attempt to load profiles (may fail if database not ready)
      try {
        await profileStore.loadProfiles();
        await profileStore.loadActiveProfile();
      } catch (err) {
        // Silent fail - will retry after database ready
      }
    }).catch(err => {
      appStore.markStepError('servers', 'Database not ready yet');
    });

    // Start timeout AFTER listeners are registered
    // Use 6 second timeout for first-run database migration (7 tables + 3 indexes)
    // After first run, subsequent startups will be much faster
    setTimeout(() => {
      if (!appReadyReceived) {
        console.warn('⚠️ FRONTEND: No app-ready event received after 6 seconds, forcing completion');
        console.warn('This may indicate slow database initialization (first run creates 7 tables + 3 indexes)');
        console.warn('or event system issues. Check Rust logs for database initialization progress.');
        appStore.setDatabaseReady(true);
        appStore.setMcpManagerReady(true);
        appStore.completeInitialization();
      }
    }, 6000);
  });

  // Cleanup event listeners
  onDestroy(() => {
    if (mcpUnlisten) {
      mcpUnlisten();
      mcpUnlisten = null;
    }
    if (elicitationUnlisten) {
      elicitationUnlisten();
      elicitationUnlisten = null;
    }
  });
</script>

<!-- Loading Screen - Only show when app is not ready -->
{#if !isAppReady}
  <AppLoadingScreen />
{/if}

<!-- Enterprise App Shell -->
{#if isAppReady}
  <div class="mcp-app">
    {@render children()}
  </div>
{/if}

<!-- Elicitation Dialog (Global) -->
<ElicitationDialog
  visible={elicitationDialogVisible}
  request={elicitationRequest}
  onResponse={handleElicitationResponse}
  onClose={handleElicitationClose}
/>

<style>
  .mcp-app {
    font-family: var(--mcp-font-sans);
    color: var(--mcp-text-primary);
    background: var(--mcp-surface-primary);
    height: 100vh;
  }

  /* Global reset improvements */
  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    overflow: hidden;
  }

  :global(#app) {
    height: 100vh;
    overflow: hidden;
  }
</style>