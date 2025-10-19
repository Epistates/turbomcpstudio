<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { listen, once } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { themeStore } from '$lib/stores/themeStore';
  import { serverStore } from '$lib/stores/serverStore';
  import { profileStore } from '$lib/stores/profileStore';
  import { appStore, appStoreIsReady, appStoreState, type AppState } from '$lib/stores/appStore';
  import { uiStore } from '$lib/stores/uiStore';
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
      try {
        serverStore.handleMcpEvent(event.payload);
      } catch (err) {
        console.error('❌ Failed to process MCP event:', err);
        console.error('Event payload:', event.payload);
        // Continue listening despite error - don't break event stream
      }
    });

    elicitationUnlisten = await listen('elicitation_requested', (event: any) => {
      try {
        console.log('🔔 Received elicitation_requested event:', event.payload);
        const payload = event.payload;

        // Validate payload structure
        if (!payload || typeof payload !== 'object') {
          throw new Error('Invalid elicitation payload: not an object');
        }
        if (!payload.id || typeof payload.id !== 'string') {
          throw new Error('Invalid elicitation payload: missing or invalid id');
        }

        elicitationRequest = {
          id: payload.id,
          protocolMessageId: payload.protocolMessageId,
          serverId: payload.serverId || 'unknown',
          serverName: payload.serverName,
          message: payload.message,
          requestedSchema: payload.requestedSchema
        };
        elicitationDialogVisible = true;
      } catch (err) {
        console.error('❌ Failed to process elicitation event:', err);
        console.error('Event payload:', event.payload);
        // Don't show dialog with invalid data - continue listening
      }
    });

    await listen('app-early-ready', (event) => {
      try {
        console.log('🟢 Received app-early-ready event');
        appStore.setMcpManagerReady(true);
      } catch (err) {
        console.error('❌ Failed to process app-early-ready:', err);
        // Critical event - still try to continue startup
      }
    });

    await listen('app-ready', async (event) => {
      try {
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
          await profileStore.loadActiveProfiles();

          appStore.completeInitialization();

          const totalTime = Date.now() - initializationStartTime;
          console.log(`🎉 Full initialization completed in ${totalTime}ms`);
        } catch (err) {
          console.error('❌ FRONTEND: Failed to initialize servers after database ready:', err);
          appStore.markStepError('servers', err instanceof Error ? err.message : 'Unknown error');
          appStore.setInitializationError('Failed to load server configurations');
        }
      } catch (err) {
        console.error('❌ Failed to process app-ready event:', err);
        appStore.markStepError('app-ready', err instanceof Error ? err.message : 'Unknown error');
        // Force completion despite error to unblock UI
        appStore.setDatabaseReady(true);
        appStore.setMcpManagerReady(true);
        appStore.completeInitialization();
      }
    });

    // Issue #16 fix: Listen for initialization errors from backend
    await listen('initialization-error', (event: any) => {
      try {
        const error = event.payload;
        console.error('🚨 Initialization error:', error);

        if (error.critical) {
          // Critical error - show blocking error dialog
          const message = `Critical Error: ${error.message}\n\n${error.userAction || 'Please restart the application.'}`;
          uiStore.showError(message);
          appStore.setInitializationError(error.message);
        } else {
          // Non-critical error - show warning notification
          const message = error.fallbackUsed
            ? `Warning: ${error.message} (using ${error.fallbackUsed})`
            : `Warning: ${error.message}`;
          console.warn('⚠️', message);

          // Show toast notification (if uiStore supports it, otherwise log)
          if (typeof uiStore.showWarning === 'function') {
            uiStore.showWarning(message);
          }
        }
      } catch (err) {
        console.error('❌ Failed to process initialization-error event:', err);
      }
    });

    console.log('✅ All event listeners registered');

    // Signal backend that frontend is ready (handshake pattern)
    // Backend will respond with app-early-ready after receiving this
    const { emit } = await import('@tauri-apps/api/event');
    await emit('frontend-ready', {});
    console.log('📤 Emitted frontend-ready signal to backend');

    // Mark servers step as loading
    appStore.markStepLoading('servers');

    // Attempt initial server load (non-blocking, might fail if database not ready)
    serverStore.initialize().then(async () => {
      // Don't complete here - wait for app-ready event for proper sequencing
      // Also attempt to load profiles (may fail if database not ready)
      try {
        await profileStore.loadProfiles();
        await profileStore.loadActiveProfiles();
      } catch (err) {
        // Silent fail - will retry after database ready
      }
    }).catch(err => {
      appStore.markStepError('servers', 'Database not ready yet');
    });

    // Issue #5 fix: Timeout removed - backend panic hook now handles all failure cases
    // Backend always emits either:
    // 1. "app-ready" on successful initialization (even with fallbacks)
    // 2. "initialization-error" + "app-ready" on critical failures
    // 3. "initialization-error" + "app-ready" on panic (caught by panic hook)
    // No timeout needed - event-based synchronization is robust and follows Tauri best practices

    // Issue #18 fix: Window close cleanup is handled on Rust side via .on_window_event()
    // No JavaScript handler needed - Rust side has full control without permission issues
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