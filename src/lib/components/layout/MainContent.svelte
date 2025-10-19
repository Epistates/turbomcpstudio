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
  import Settings from '../Settings.svelte';
  import AddServerModal from '../AddServerModal.svelte';
  import EditServerModal from '../EditServerModal.svelte';
  import ModeIndicator from '../ModeIndicator.svelte';
  import ServerContextBar from '../ServerContextBar.svelte';

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
    currentView === 'protocol'
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
  $effect(() => {
    if (showContextBar) {
      contextStore.autoSelectServer(contextBarMode());
    }
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
      <!-- NOTE: Collections view disabled in Sidebar for v1 but code preserved -->
      <!-- TODO: Enable in v2 with proper multi-server UI design -->
      <CollectionsManager />
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

  /* Placeholder Styles */
  .mcp-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: var(--mcp-space-8);
    background: var(--mcp-surface-primary);
  }

  .mcp-placeholder__content {
    max-width: 600px;
    text-align: center;
  }

  .mcp-placeholder__icon {
    font-size: 4rem;
    margin-bottom: var(--mcp-space-6);
    opacity: 0.8;
  }

  .mcp-placeholder__title {
    font-size: var(--mcp-text-3xl);
    font-weight: var(--mcp-font-bold);
    color: var(--mcp-text-primary);
    margin: 0 0 var(--mcp-space-4) 0;
  }

  .mcp-placeholder__description {
    font-size: var(--mcp-text-lg);
    color: var(--mcp-text-secondary);
    margin: 0 0 var(--mcp-space-8) 0;
    line-height: var(--mcp-leading-relaxed);
  }

  .mcp-placeholder__features {
    background: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    padding: var(--mcp-space-6);
    margin-bottom: var(--mcp-space-8);
    text-align: left;
  }

  .mcp-placeholder__features h3 {
    font-size: var(--mcp-text-lg);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    margin: 0 0 var(--mcp-space-3) 0;
  }

  .mcp-placeholder__features ul {
    margin: 0;
    padding-left: var(--mcp-space-5);
    color: var(--mcp-text-secondary);
  }

  .mcp-placeholder__features li {
    margin-bottom: var(--mcp-space-1-5);
    line-height: var(--mcp-leading-relaxed);
  }

  .mcp-placeholder__actions {
    display: flex;
    justify-content: center;
    gap: var(--mcp-space-3);
  }

  /* Modal System */
  .mcp-modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--mcp-z-modal);
    padding: var(--mcp-space-4);
  }

  [data-theme="dark"] .mcp-modal-overlay {
    background: rgba(0, 0, 0, 0.7);
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

  /* High contrast mode support */
  @media (prefers-contrast: high) {
    .mcp-placeholder__features {
      border: 2px solid var(--mcp-border-primary);
    }
  }

  /* Reduced motion support */
  @media (prefers-reduced-motion: reduce) {
    .mcp-modal-overlay {
      backdrop-filter: none;
    }
  }

  /* Mobile adjustments */
  @media (max-width: 767px) {
    .mcp-placeholder {
      padding: var(--mcp-space-4);
    }
    
    .mcp-placeholder__title {
      font-size: var(--mcp-text-2xl);
    }
    
    .mcp-placeholder__description {
      font-size: var(--mcp-text-base);
    }
    
    .mcp-placeholder__features {
      padding: var(--mcp-space-4);
    }
  }
</style>