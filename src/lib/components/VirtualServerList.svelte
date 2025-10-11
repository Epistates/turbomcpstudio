<script lang="ts">
  import { type ServerInfo } from '$lib/stores/serverStore';
  import ServerOverview from './ServerOverview.svelte';

  interface Props {
    servers: ServerInfo[];
    itemHeight?: number;
    containerHeight?: number;
  }

  let { servers, itemHeight = 120, containerHeight = 600 }: Props = $props();

  let scrollTop = $state(0);
  let containerElement: HTMLDivElement;

  // Virtual scrolling calculations
  const visibleStart = $derived(Math.floor(scrollTop / itemHeight));
  const visibleEnd = $derived(Math.min(
    servers.length,
    Math.ceil((scrollTop + containerHeight) / itemHeight) + 1
  ));

  const visibleServers = $derived(servers.slice(visibleStart, visibleEnd));
  const offsetY = $derived(visibleStart * itemHeight);
  const totalHeight = $derived(servers.length * itemHeight);

  function handleScroll(e: Event) {
    scrollTop = (e.target as HTMLDivElement).scrollTop;
  }
</script>

<div
  class="virtual-list-container"
  style="height: {containerHeight}px; overflow-y: auto;"
  onscroll={handleScroll}
  bind:this={containerElement}
>
  <div class="virtual-list-spacer" style="height: {totalHeight}px;">
    <div class="virtual-list-content" style="transform: translateY({offsetY}px);">
      {#each visibleServers as server (server.id)}
        <div style="height: {itemHeight}px;">
          <ServerOverview {server} />
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .virtual-list-container {
    position: relative;
    overflow-y: auto;
  }

  .virtual-list-spacer {
    position: relative;
  }

  .virtual-list-content {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
  }
</style>
