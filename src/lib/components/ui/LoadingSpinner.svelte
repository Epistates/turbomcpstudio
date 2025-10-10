<!--
  LoadingSpinner Component
  Reusable animated loading spinner with size variants
-->
<script lang="ts">
  type SpinnerSize = 'xs' | 'sm' | 'md' | 'lg' | 'xl';

  interface Props {
    size?: SpinnerSize;
    class?: string;
  }

  const {
    size = 'md',
    class: className = ''
  }: Props = $props();

  // Size mapping to pixel values
  const sizeClasses: Record<SpinnerSize, string> = {
    xs: 'h-3 w-3',
    sm: 'h-4 w-4',
    md: 'h-5 w-5',
    lg: 'h-6 w-6',
    xl: 'h-8 w-8'
  };

  const computedClass = $derived(`${sizeClasses[size]} ${className}`);
</script>

<svg
  class="animate-spin {computedClass}"
  xmlns="http://www.w3.org/2000/svg"
  fill="none"
  viewBox="0 0 24 24"
  aria-hidden="true"
  role="status"
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
