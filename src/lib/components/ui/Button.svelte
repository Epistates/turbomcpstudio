<!--
  MCP Studio Button Component
  Enterprise-grade button with full design system integration
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'success';
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

  const dispatch = createEventDispatcher();

  // Compute classes based on props
  const baseClasses = $derived([
    'btn',
    `btn-${variant}`,
    `btn-${size}`,
    fullWidth && 'w-full',
    loading && 'opacity-75 cursor-wait',
    disabled && 'opacity-60 cursor-not-allowed'
  ].filter(Boolean).join(' '));

  // Handle click events
  function handleClick(event: MouseEvent) {
    if (loading || disabled) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    onclick?.(event);
    dispatch('click', event);
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
      <svg 
        class="animate-spin h-4 w-4" 
        xmlns="http://www.w3.org/2000/svg" 
        fill="none" 
        viewBox="0 0 24 24"
        aria-hidden="true"
      >
        <circle 
          class="opacity-25" 
          cx="12" 
          cy="12" 
          r="10" 
          stroke="currentColor" 
          stroke-width="4"
        ></circle>
        <path 
          class="opacity-75" 
          fill="currentColor" 
          d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
        ></path>
      </svg>
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
      <svg 
        class="animate-spin h-4 w-4" 
        xmlns="http://www.w3.org/2000/svg" 
        fill="none" 
        viewBox="0 0 24 24"
        aria-hidden="true"
      >
        <circle 
          class="opacity-25" 
          cx="12" 
          cy="12" 
          r="10" 
          stroke="currentColor" 
          stroke-width="4"
        ></circle>
        <path 
          class="opacity-75" 
          fill="currentColor" 
          d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
        ></path>
      </svg>
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