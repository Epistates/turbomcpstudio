<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { listen, once } from '@tauri-apps/api/event';
  import { themeStore } from '$lib/stores/themeStore';
  import { serverStore } from '$lib/stores/serverStore';
  import { appStore, appStoreIsReady, appStoreState, type AppState } from '$lib/stores/appStore';
  import AppLoadingScreen from '$lib/components/AppLoadingScreen.svelte';

  // Svelte 5 snippet props
  const { children } = $props();

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
  let appReadyReceived = false;
  let initializationStartTime = Date.now();

  // Set up event listeners synchronously to prevent race conditions
  // Use Tauri's recommended synchronous pattern from docs

  // Set up event listeners immediately and synchronously
  listen('mcp-event', (event) => {
    serverStore.handleMcpEvent(event.payload);
  }).then((unlisten) => {
    mcpUnlisten = unlisten;
  });

  listen('app-early-ready', (event) => {
    appStore.setMcpManagerReady(true);
  }).then(() => {
  });

  listen('app-ready', async (event) => {
    if (appReadyReceived) {
      return;
    }
    appReadyReceived = true;
    appStore.setDatabaseReady(true);

    try {
      // Retry server initialization after database is ready
      await serverStore.initialize();
      appStore.markStepCompleted('servers');
      appStore.completeInitialization();

      const totalTime = Date.now() - initializationStartTime;
    } catch (err) {
      console.error('❌ FRONTEND: Failed to initialize servers after database ready:', err);
      appStore.markStepError('servers', err instanceof Error ? err.message : 'Unknown error');
      appStore.setInitializationError('Failed to load server configurations');
    }
  }).then(() => {
  });

  // Shorter timeout with aggressive fallback
  setTimeout(() => {
    if (!appReadyReceived) {
      console.warn('⚠️ FRONTEND: No app-ready event received after 1 second, forcing completion');
      appStore.setDatabaseReady(true);
      appStore.setMcpManagerReady(true);
      appStore.completeInitialization();
    }
  }, 1000);

  // Initialize app systems on DOM ready
  onMount(async () => {

    // Initialize theme system (fast, synchronous)
    themeStore.init();

    // Mark servers step as loading
    appStore.markStepLoading('servers');

    // Attempt initial server load (non-blocking, might fail if database not ready)
    serverStore.initialize().then(() => {
      // Don't complete here - wait for app-ready event for proper sequencing
    }).catch(err => {
      appStore.markStepError('servers', 'Database not ready yet');
    });
  });

  // Cleanup event listeners
  onDestroy(() => {
    if (mcpUnlisten) {
      mcpUnlisten();
      mcpUnlisten = null;
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