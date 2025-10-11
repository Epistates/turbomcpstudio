<!--
  MCP Studio Master Layout
  Enterprise-grade adaptive layout system with resizable panels
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { themeStore } from '$lib/stores/themeStore';
  import Header from './Header.svelte';
  import Sidebar from './Sidebar.svelte';
  import MainContent from './MainContent.svelte';
  import { uiStore } from '$lib/stores/uiStore';

  // Layout state
  let sidebarWidth = $state(280);
  let isResizing = $state(false);
  let isSidebarCollapsed = $state(false);
  let isMobileMenuOpen = $state(false);
  
  // Responsive breakpoints
  let innerWidth = $state(0);
  const isMobile = $derived(innerWidth < 768);
  const isTablet = $derived(innerWidth >= 768 && innerWidth < 1024);
  const isDesktop = $derived(innerWidth >= 1024);

  // Sidebar state management
  $effect(() => {
    if (isMobile) {
      isSidebarCollapsed = true;
    }
  });

  // Panel resizing logic
  function startResize(event: MouseEvent) {
    if (isMobile) return;
    
    isResizing = true;
    document.addEventListener('mousemove', handleResize);
    document.addEventListener('mouseup', stopResize);
    document.body.style.userSelect = 'none';
    document.body.style.cursor = 'col-resize';
  }

  function handleResize(event: MouseEvent) {
    if (!isResizing) return;
    
    const minWidth = 200;
    const maxWidth = Math.min(500, innerWidth * 0.4);
    sidebarWidth = Math.max(minWidth, Math.min(maxWidth, event.clientX));
  }

  function stopResize() {
    isResizing = false;
    document.removeEventListener('mousemove', handleResize);
    document.removeEventListener('mouseup', stopResize);
    document.body.style.userSelect = '';
    document.body.style.cursor = '';
  }

  // Toggle sidebar
  function toggleSidebar() {
    if (isMobile) {
      isMobileMenuOpen = !isMobileMenuOpen;
    } else {
      isSidebarCollapsed = !isSidebarCollapsed;
    }
  }

  // Close mobile menu when clicking outside
  function closeMobileMenu() {
    if (isMobileMenuOpen) {
      isMobileMenuOpen = false;
    }
  }

  // Theme is already initialized in +layout.svelte, no need to init again

  // Handle escape key
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && isMobileMenuOpen) {
      isMobileMenuOpen = false;
    }
  }

  // CSS custom properties for dynamic layout
  const layoutStyles = $derived(`
    --sidebar-width: ${isSidebarCollapsed ? '0px' : `${sidebarWidth}px`};
    --sidebar-collapsed: ${isSidebarCollapsed ? '1' : '0'};
    --header-height: 64px;
  `);
</script>

<svelte:window bind:innerWidth onkeydown={handleKeydown} />

<!-- Main application shell -->
<div 
  class="mcp-layout h-screen overflow-hidden bg-mcp-surface-primary"
  style={layoutStyles}
  data-mobile={isMobile}
  data-sidebar-collapsed={isSidebarCollapsed}
>
  <!-- Header Bar -->
  <Header
    ontoggleSidebar={toggleSidebar}
    {isMobile}
    {isSidebarCollapsed}
    {isMobileMenuOpen}
  />

  <!-- Main Content Area -->
  <div class="mcp-content-wrapper">
    <!-- Sidebar -->
    <aside
      class="mcp-sidebar"
      class:mcp-sidebar--collapsed={isSidebarCollapsed}
      class:mcp-sidebar--mobile-open={isMobileMenuOpen}
      style:width={isSidebarCollapsed ? '0px' : `${sidebarWidth}px`}
    >
      <Sidebar {isMobile} {isSidebarCollapsed} />
    </aside>

    <!-- Resize Handle -->
    {#if !isMobile && !isSidebarCollapsed}
      <button
        class="mcp-resize-handle"
        class:mcp-resize-handle--active={isResizing}
        aria-label="Resize sidebar"
        type="button"
        onmousedown={startResize}
        onkeydown={(e) => {
          if (e.key === 'ArrowLeft') sidebarWidth = Math.max(200, sidebarWidth - 20);
          if (e.key === 'ArrowRight') sidebarWidth = Math.min(500, sidebarWidth + 20);
        }}
      ></button>
    {/if}

    <!-- Main Content -->
    <main class="mcp-main">
      <MainContent />
    </main>
  </div>

  <!-- Mobile Menu Overlay -->
  {#if isMobile && isMobileMenuOpen}
    <button
      class="mcp-mobile-overlay"
      type="button"
      aria-label="Close sidebar"
      onclick={closeMobileMenu}
    ></button>
  {/if}
</div>

<style>
  /* Master Layout Grid */
  .mcp-layout {
    display: grid;
    grid-template-rows: var(--header-height) 1fr;
    grid-template-areas: 
      "header"
      "content";
  }

  /* Content Wrapper - Houses sidebar and main */
  .mcp-content-wrapper {
    grid-area: content;
    display: flex;
    height: 100%;
    overflow: hidden;
  }

  /* Sidebar Styles */
  .mcp-sidebar {
    background: var(--mcp-surface-secondary);
    border-right: 1px solid var(--mcp-border-primary);
    transition: width var(--mcp-transition-base), transform var(--mcp-transition-base);
    flex-shrink: 0;
    height: 100%;
    overflow: hidden;
    z-index: 20;
  }

  .mcp-sidebar--collapsed {
    width: 0 !important;
    border-right: none;
  }

  /* Mobile Sidebar */
  @media (max-width: 767px) {
    .mcp-sidebar {
      position: fixed;
      top: var(--header-height);
      left: 0;
      width: 280px !important;
      height: calc(100vh - var(--header-height));
      transform: translateX(-100%);
      z-index: 50;
    }

    .mcp-sidebar--mobile-open {
      transform: translateX(0);
    }
  }

  /* Resize Handle */
  .mcp-resize-handle {
    width: 4px;
    height: 100%;
    background: transparent;
    cursor: col-resize;
    flex-shrink: 0;
    position: relative;
    transition: background-color var(--mcp-transition-fast);
    border: none;
    padding: 0;
    margin: 0;
    display: block;
  }

  .mcp-resize-handle:hover,
  .mcp-resize-handle:focus {
    background: var(--mcp-primary-500);
  }

  .mcp-resize-handle--active {
    background: var(--mcp-primary-600);
  }

  .mcp-resize-handle:focus {
    outline: none;
  }

  /* Add visual indicator */
  .mcp-resize-handle::after {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    width: 3px;
    height: 24px;
    background: currentColor;
    border-radius: 1px;
    transform: translate(-50%, -50%);
    opacity: 0;
    transition: opacity var(--mcp-transition-fast);
  }

  .mcp-resize-handle:hover::after,
  .mcp-resize-handle:focus::after,
  .mcp-resize-handle--active::after {
    opacity: 0.6;
  }

  /* Main Content Area */
  .mcp-main {
    flex: 1;
    height: 100%;
    overflow: hidden;
    background: var(--mcp-surface-primary);
  }

  /* Mobile Overlay */
  .mcp-mobile-overlay {
    position: fixed;
    top: var(--header-height);
    left: 0;
    width: 100%;
    height: calc(100vh - var(--header-height));
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(2px);
    z-index: 40;
    border: none;
    padding: 0;
    margin: 0;
    cursor: pointer;
  }

  /* Dark theme adjustments */
  [data-theme="dark"] .mcp-mobile-overlay {
    background: rgba(0, 0, 0, 0.7);
  }

  /* High contrast mode support */
  @media (prefers-contrast: high) {
    .mcp-sidebar {
      border-right: 2px solid var(--mcp-border-primary);
    }
    
    .mcp-resize-handle {
      background: var(--mcp-border-primary);
    }
  }

  /* Reduced motion support */
  @media (prefers-reduced-motion: reduce) {
    .mcp-sidebar,
    .mcp-resize-handle {
      transition: none;
    }
  }
</style>