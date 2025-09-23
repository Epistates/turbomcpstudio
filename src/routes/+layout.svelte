<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { listen, once } from '@tauri-apps/api/event';
  import { themeStore } from '$lib/stores/themeStore';
  import { serverStore } from '$lib/stores/serverStore';
  import { appStore, appStoreIsReady, appStoreState } from '$lib/stores/appStore';
  import AppLoadingScreen from '$lib/components/AppLoadingScreen.svelte';

  // Reactive state using Svelte 5 runes
  const isAppReady = $derived($appStoreIsReady);
  const currentAppState = $derived($appStoreState);

  // Track cleanup functions
  let mcpUnlisten: (() => void) | null = null;
  let appReadyReceived = false;
  let initializationStartTime = Date.now();

  // Set up event listeners IMMEDIATELY when script loads (before onMount)
  // This prevents race conditions with backend event emission
  const setupEventListeners = async () => {
    console.log('🚀 FRONTEND: Setting up event listeners (immediate)');

    try {
      // Listen for MCP events for real-time updates (persistent)
      mcpUnlisten = await listen('mcp-event', (event) => {
        console.log('📨 FRONTEND: Received MCP event:', event);
        serverStore.handleMcpEvent(event.payload);
      });
      console.log('✅ FRONTEND: MCP event listener set up');

      // Listen for app-early-ready event (MCP manager is ready)
      await listen('app-early-ready', (event) => {
        console.log('🌟 FRONTEND: RECEIVED app-early-ready event!', event);
        appStore.setMcpManagerReady(true);
      });
      console.log('✅ FRONTEND: app-early-ready event listener set up');

      // Use 'once' for app-ready event since it's a single-fire event
      await once('app-ready', async (event) => {
        console.log('🎉 FRONTEND: RECEIVED app-ready event!', event);
        appReadyReceived = true;
        appStore.setDatabaseReady(true);

        try {
          // Retry server initialization after database is ready
          await serverStore.initialize();
          appStore.markStepCompleted('servers');
          appStore.completeInitialization();

          const totalTime = Date.now() - initializationStartTime;
          console.log(`✅ FRONTEND: App initialization completed in ${totalTime}ms`);
        } catch (err) {
          console.error('❌ FRONTEND: Failed to initialize servers after database ready:', err);
          appStore.markStepError('servers', err instanceof Error ? err.message : 'Unknown error');
          appStore.setInitializationError('Failed to load server configurations');
        }
      });
      console.log('✅ FRONTEND: app-ready event listener set up');

      console.log('🔄 FRONTEND: All event listeners are now active and waiting for events...');

      // Timeout fallback if backend doesn't respond within 5 seconds
      setTimeout(() => {
        if (!appReadyReceived) {
          console.warn('⚠️ FRONTEND: No app-ready event received after 5 seconds, forcing completion');
          appStore.setDatabaseReady(true);
          appStore.setMcpManagerReady(true);
          appStore.completeInitialization();
        }
      }, 5000);

    } catch (error) {
      console.error('❌ FRONTEND: Failed to setup event listeners:', error);
      appStore.setInitializationError('Failed to setup event communication');
    }
  };

  // Call setup immediately when script loads (before DOM/onMount)
  setupEventListeners();

  // Initialize app systems on DOM ready
  onMount(async () => {
    console.log('🚀 FRONTEND: Starting application initialization (onMount)');

    // Initialize theme system (fast, synchronous)
    themeStore.init();
    console.log('✅ FRONTEND: Theme store initialized');

    // Mark servers step as loading
    appStore.markStepLoading('servers');
    console.log('🔄 FRONTEND: Marked servers as loading');

    // Attempt initial server load (non-blocking, might fail if database not ready)
    serverStore.initialize().then(() => {
      console.log('Initial server store initialization succeeded');
      // Don't complete here - wait for app-ready event for proper sequencing
    }).catch(err => {
      console.log('Initial server store initialization failed, will retry when backend ready:', err);
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

<!-- Loading Screen -->
<AppLoadingScreen />

<!-- Enterprise App Shell -->
{#if isAppReady}
  <div class="mcp-app">
    <slot />
  </div>
{/if}

<style>
  .mcp-app {
    font-family: var(--mcp-font-sans);
    color: var(--mcp-text-primary);
    background: var(--mcp-surface-primary);
    min-height: 100vh;
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