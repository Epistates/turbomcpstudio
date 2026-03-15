<!--
  MCP Studio Main Content Area
  Dynamic content routing based on current view
-->
<script lang="ts">
  import { uiStore } from '$lib/stores/uiStore';
  import { contextStore } from '$lib/stores/contextStore';
  import Dashboard from '../Dashboard.svelte';
  import ServerManagement from '../ServerManagement.svelte';
  import ToolExplorer from '../ToolExplorer.svelte';
  import ResourceBrowser from '../ResourceBrowser.svelte';
  import PromptDesigner from '../PromptDesigner.svelte';
  import SamplingView from '../SamplingView.svelte';
  import ElicitationView from '../ElicitationView.svelte';
  import ProtocolInspector from '../ProtocolInspector.svelte';
  import CollectionsManager from '../CollectionsManager.svelte';
  import ChatPlayground from '../ChatPlayground.svelte';
  import UnifiedTesting from '../UnifiedTesting.svelte';
  import Settings from '../Settings.svelte';
  import AddServerModal from '../AddServerModal.svelte';
  import EditServerModal from '../EditServerModal.svelte';
  import ModeIndicator from '../ModeIndicator.svelte';
  import ServerContextBar from '../ServerContextBar.svelte';
  import OAuthDebugger from '../oauth/OAuthDebugger.svelte';

  // Reactive view state using Svelte 5 runes
  // Access store properties directly with $derived to maintain reactivity
  const currentView = $derived($uiStore.currentView);
  const modals = $derived($uiStore.modals);
  const pendingSamplingRequest = $derived($uiStore.pendingSamplingRequest);

  // Check if current view should show mode indicator (testing-related views)
  const showModeIndicator = $derived(
    currentView === 'sampling' || currentView === 'elicitation' || currentView === 'protocol'
  );

  // ✅ NEW: Check if current view should show server context bar (operational views)
  // NOTE: Collections disabled for v1 (requires multi-server UI design)
  const showContextBar = $derived(
    currentView === 'tools' ||
    currentView === 'resources' ||
    currentView === 'prompts' ||
    currentView === 'sampling' ||
    currentView === 'elicitation' ||
    currentView === 'protocol' ||
    currentView === 'testing'
  );

  // ✅ NEW: Get required capability for current view (for filtering servers)
  const requiredCapability = $derived(() => {
    switch (currentView) {
      case 'tools': return 'tools';
      case 'resources': return 'resources';
      case 'prompts': return 'prompts';
      case 'sampling': return 'sampling';
      case 'elicitation': return 'elicitation';
      default: return null;
    }
  });

  // ✅ NEW: Get ServerContextBar mode based on current view
  const contextBarMode = $derived(() => {
    switch (currentView) {
      case 'sampling':
      case 'elicitation':
        return 'filter';  // Optional filter mode for monitoring tabs
      default:
        return 'selector';  // Required selection mode for operational tabs
    }
  });

  // ✅ NEW: Auto-select server when context bar is shown
  // Pass mode to respect filter vs selector behavior
  // Also re-trigger when servers become available (fixes race condition on profile activation)
  // BUG FIX: Addresses issue where "No tool servers connected" appears after profile activation
  // even though servers are connecting. The fix detects when servers transition from
  // disconnected to connected state and triggers server selection at that point.
  const contextState = $derived($contextStore);
  const availableServerCount = $derived(contextState.availableServers.length);

  // Track previous server count to detect when servers become available
  let previousServerCount = $state(0);

  $effect(() => {
    // Track availableServerCount to ensure effect re-runs when servers connect
    const serverCount = availableServerCount;

    if (showContextBar) {
      // Auto-select when:
      // 1. Context bar first becomes visible
      // 2. Servers become available (0 -> N transition, important for profile activation)
      // 3. First server connects when none were connected before
      if (serverCount > 0 && (previousServerCount === 0 || !contextState.selectedServer)) {
        contextStore.autoSelectServer(contextBarMode());
      }
    }

    previousServerCount = serverCount;
  });

  // Content component mapping (unused - kept for reference)
  // function getContentComponent(view: string) {
  //   switch (view) {
  //     case 'dashboard':
  //       return Dashboard;
  //     case 'tools':
  //       return ToolExplorer;
  //     case 'resources':
  //       return ResourceBrowser;
  //     case 'prompts':
  //       return PromptDesigner;
  //     case 'sampling':
  //       return SamplingView;
  //     case 'elicitation':
  //       return ElicitationView;
  //     case 'collections':
  //     case 'settings':
  //       return PlaceholderView;
  //     default:
  //       return Dashboard;
  //   }
  // }
</script>

<div class="mcp-main-content">
  <!-- Mode Indicator for Testing Views -->
  {#if showModeIndicator}
    <ModeIndicator mode="manual" compact={true} />
  {/if}

  <!-- ✅ NEW: Server Context Bar for Operational Views -->
  {#if showContextBar}
    <ServerContextBar requiredCapability={requiredCapability()} mode={contextBarMode()} />
  {/if}

  <!-- Dynamic Content Area -->
  <div class="mcp-content-viewport">
    {#if currentView === 'dashboard'}
      <Dashboard />
    {:else if currentView === 'servers'}
      <ServerManagement />
    {:else if currentView === 'tools'}
      <ToolExplorer />
    {:else if currentView === 'resources'}
      <ResourceBrowser />
    {:else if currentView === 'prompts'}
      <PromptDesigner />
    {:else if currentView === 'sampling'}
      <SamplingView />
    {:else if currentView === 'elicitation'}
      <ElicitationView />
    {:else if currentView === 'protocol'}
      <ProtocolInspector />
    {:else if currentView === 'collections'}
      <CollectionsManager />
    {:else if currentView === 'chat'}
      <ChatPlayground />
    {:else if currentView === 'testing'}
      <UnifiedTesting serverId={contextState.selectedServer?.id} />
    {:else if currentView === 'oauth'}
      <OAuthDebugger />
    {:else if currentView === 'settings'}
      <Settings />
    {/if}
  </div>
</div>

<!-- Modal System -->
<!-- Note: Each modal component renders its own backdrop, no wrapper needed -->
{#if modals.addServer.open}
  <AddServerModal />
{/if}

{#if modals.samplingApproval.open && pendingSamplingRequest}
  {#await import('../SamplingApprovalModal.svelte')}
    <div class="loading">Loading...</div>
  {:then { default: SamplingApprovalModal }}
    <SamplingApprovalModal
      request={pendingSamplingRequest}
      onClose={() => uiStore.closeSamplingApproval()}
    />
  {/await}
{/if}

{#if modals.serverConfig.open}
  <EditServerModal />
{/if}

<style>
  .mcp-main-content {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--mcp-surface-primary);
  }

  .mcp-content-viewport {
    flex: 1;
    overflow: auto;
    height: 100%;
  }

  /* Ensure all child containers propagate height */
  .mcp-content-viewport > :global(*) {
    height: 100%;
  }

  /* Content scrollbar styling */
  .mcp-content-viewport::-webkit-scrollbar {
    width: 8px;
  }

  .mcp-content-viewport::-webkit-scrollbar-track {
    background: var(--mcp-surface-secondary);
  }

  .mcp-content-viewport::-webkit-scrollbar-thumb {
    background: var(--mcp-border-primary);
    border-radius: 4px;
  }

  .mcp-content-viewport::-webkit-scrollbar-thumb:hover {
    background: var(--mcp-border-secondary);
  }

  /* Focus management */
  .mcp-content-viewport:focus {
    outline: none;
  }
</style>