<!--
  MCP Studio Button Component
  Enterprise-grade button with full design system integration
-->
<script lang="ts">
  import LoadingSpinner from './LoadingSpinner.svelte';

  type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'success' | 'outline';
  type ButtonSize = 'sm' | 'md' | 'lg';

  interface Props {
    variant?: ButtonVariant;
    size?: ButtonSize;
    loading?: boolean;
    disabled?: boolean;
    fullWidth?: boolean;
    leftIcon?: any; // Lucide component
    rightIcon?: any; // Lucide component
    href?: string;
    children?: any;
    onclick?: (event: MouseEvent) => void;
    [key: string]: any;
  }

  // Props using Svelte 5 runes
  const {
    variant = 'primary' as ButtonVariant,
    size = 'md' as ButtonSize,
    loading = false,
    disabled = false,
    fullWidth = false,
    leftIcon = undefined,
    rightIcon = undefined,
    href = undefined as string | undefined,
    children,
    onclick,
    ...restProps
  }: Props = $props();

  // Compute classes based on props
  const baseClasses = $derived([
    'btn',
    `btn-${variant}`,
    `btn-${size}`,
    fullWidth && 'w-full',
    loading && 'opacity-75 cursor-wait',
    disabled && 'opacity-60 cursor-not-allowed'
  ].filter(Boolean).join(' '));

  // Handle click events with proper validation
  function handleClick(event: MouseEvent) {
    if (loading || disabled) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    onclick?.(event);
  }

  // Handle key events for accessibility
  function handleKeydown(event: KeyboardEvent) {
    if (loading || disabled) return;
    
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleClick(event as any);
    }
  }
</script>

<!-- Use dynamic component for button vs anchor -->
{#if href}
  <a
    {href}
    class={baseClasses}
    class:pointer-events-none={loading || disabled}
    role="button"
    tabindex={disabled ? -1 : 0}
    aria-disabled={disabled}
    onclick={handleClick}
    onkeydown={handleKeydown}
    {...restProps}
  >
    {#if loading}
      <LoadingSpinner size={size === 'sm' ? 'sm' : size === 'lg' ? 'lg' : 'md'} />
    {:else if leftIcon}
      {@const IconComponent = leftIcon}
      <IconComponent size={size === 'sm' ? 14 : size === 'lg' ? 18 : 16} />
    {/if}

    {@render children?.()}

    {#if !loading && rightIcon}
      {@const IconComponent = rightIcon}
      <IconComponent size={size === 'sm' ? 14 : size === 'lg' ? 18 : 16} />
    {/if}
  </a>
{:else}
  <button
    type={restProps.type || 'button'}
    class={baseClasses}
    disabled={disabled || loading}
    aria-disabled={disabled || loading}
    onclick={handleClick}
    {...restProps}
  >
    {#if loading}
      <LoadingSpinner size={size === 'sm' ? 'sm' : size === 'lg' ? 'lg' : 'md'} />
    {:else if leftIcon}
      {@const IconComponent = leftIcon}
      <IconComponent size={size === 'sm' ? 14 : size === 'lg' ? 18 : 16} />
    {/if}
    
    {@render children?.()}
    
    {#if !loading && rightIcon}
      {@const IconComponent = rightIcon}
      <IconComponent size={size === 'sm' ? 14 : size === 'lg' ? 18 : 16} />
    {/if}
  </button>
{/if}