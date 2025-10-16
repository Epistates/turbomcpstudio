<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface SystemInfo {
    os: string;
    arch: string;
    version: string;
    family: string;
    locale: string;
  }

  let systemInfo: SystemInfo | null = null;
  let loading = true;
  let error: string | null = null;

  onMount(async () => {
    try {
      systemInfo = await invoke('get_system_info');
      loading = false;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      loading = false;
    }
  });

  // Helper function to format OS names nicely
  function formatOS(os: string): string {
    const osMap: Record<string, string> = {
      macos: 'macOS',
      windows: 'Windows',
      linux: 'Linux',
    };
    return osMap[os] || os;
  }

  // Helper function to format architecture names
  function formatArch(arch: string): string {
    const archMap: Record<string, string> = {
      x86_64: 'x86-64 (Intel/AMD)',
      aarch64: 'ARM64 (Apple Silicon)',
      arm: 'ARM (32-bit)',
      amd64: 'x86-64 (64-bit)',
    };
    return archMap[arch] || arch;
  }
</script>

<div class="system-info">
  <h2>System Information</h2>

  {#if loading}
    <div class="loading">
      <p>Loading system information...</p>
    </div>
  {:else if error}
    <div class="error">
      <p>Error loading system information: {error}</p>
    </div>
  {:else if systemInfo}
    <div class="info-grid">
      <div class="info-item">
        <dt>Operating System</dt>
        <dd>{formatOS(systemInfo.os)}</dd>
      </div>

      <div class="info-item">
        <dt>Architecture</dt>
        <dd>{formatArch(systemInfo.arch)}</dd>
      </div>

      <div class="info-item">
        <dt>OS Version</dt>
        <dd>{systemInfo.version}</dd>
      </div>

      <div class="info-item">
        <dt>OS Family</dt>
        <dd>{systemInfo.family}</dd>
      </div>

      <div class="info-item">
        <dt>Locale</dt>
        <dd>{systemInfo.locale}</dd>
      </div>
    </div>
  {/if}
</div>

<style>
  .system-info {
    padding: 1.5rem;
    background-color: var(--color-bg-secondary, #f5f5f5);
    border-radius: 0.5rem;
    margin: 1rem 0;
  }

  h2 {
    margin: 0 0 1rem 0;
    font-size: 1.25rem;
    color: var(--color-text-primary, #333);
  }

  .loading,
  .error {
    padding: 1rem;
    text-align: center;
  }

  .error {
    background-color: var(--color-error-bg, #fee);
    color: var(--color-error-text, #c33);
    border-radius: 0.25rem;
  }

  .info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1rem;
  }

  .info-item {
    background-color: var(--color-bg-primary, white);
    padding: 1rem;
    border-radius: 0.25rem;
    border-left: 3px solid var(--color-primary, #0066cc);
  }

  dt {
    font-weight: 600;
    color: var(--color-text-secondary, #666);
    font-size: 0.875rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 0.25rem;
  }

  dd {
    margin: 0;
    color: var(--color-text-primary, #333);
    font-size: 1rem;
    font-family: var(--font-mono, 'Monaco', 'Menlo', monospace);
  }

  @media (prefers-color-scheme: dark) {
    .system-info {
      background-color: var(--color-bg-secondary, #2a2a2a);
    }

    h2 {
      color: var(--color-text-primary, #fff);
    }

    .info-item {
      background-color: var(--color-bg-primary, #1a1a1a);
    }

    dt {
      color: var(--color-text-secondary, #aaa);
    }

    dd {
      color: var(--color-text-primary, #fff);
    }
  }
</style>
