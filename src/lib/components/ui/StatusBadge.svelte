<!--
  StatusBadge - Reusable status indicator for sampling requests

  Features:
  - Multiple size variants (sm, md, lg)
  - Automatic color mapping based on status
  - Dark mode support
  - Uppercase text display
-->
<script lang="ts">
  import { getStatusColor } from '$lib/utils/samplingUtils';

  const { status, size = 'md', uppercase = true } = $props<{
    status: string;
    size?: 'sm' | 'md' | 'lg';
    uppercase?: boolean;
  }>();

  const sizeClasses: Record<string, string> = {
    sm: 'text-xs px-2 py-1',
    md: 'text-xs px-3 py-1',
    lg: 'text-sm px-3 py-1.5'
  };

  const displayText = $derived(uppercase ? status.toUpperCase() : status);
  const sizeClass = $derived(sizeClasses[size] || sizeClasses.md);
</script>

<span class="rounded-full border font-medium {sizeClass} {getStatusColor(status)}">
  {displayText}
</span>
