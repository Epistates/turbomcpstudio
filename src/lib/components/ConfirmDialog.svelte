<script lang="ts">
  import { X, AlertTriangle } from 'lucide-svelte';
  import Button from './ui/Button.svelte';

  let {
    title = 'Confirm',
    message,
    confirmText = 'Confirm',
    cancelText = 'Cancel',
    variant = 'danger',
    onConfirm,
    onCancel
  }: {
    title?: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    variant?: 'danger' | 'warning' | 'primary';
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();

  function handleConfirm() {
    onConfirm();
  }

  function handleCancel() {
    onCancel();
  }

  function handleEscape(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleCancel();
    }
  }
</script>

<svelte:window onkeydown={handleEscape} />

<div
  class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
  onclick={handleCancel}
  role="dialog"
  aria-modal="true"
>
  <div
    class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-md w-full"
    onclick={(e) => e.stopPropagation()}
    role="document"
  >
    <!-- Header -->
    <div class="flex items-start justify-between p-6 border-b border-gray-200 dark:border-gray-700">
      <div class="flex items-center gap-3">
        {#if variant === 'danger'}
          <div class="w-10 h-10 rounded-full bg-red-100 dark:bg-red-900/20 flex items-center justify-center flex-shrink-0">
            <AlertTriangle class="text-red-600 dark:text-red-400" size={20} />
          </div>
        {:else if variant === 'warning'}
          <div class="w-10 h-10 rounded-full bg-yellow-100 dark:bg-yellow-900/20 flex items-center justify-center flex-shrink-0">
            <AlertTriangle class="text-yellow-600 dark:text-yellow-400" size={20} />
          </div>
        {/if}
        <h2 class="text-xl font-bold text-gray-900 dark:text-white">
          {title}
        </h2>
      </div>
      <button
        onclick={handleCancel}
        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
      >
        <X size={20} />
      </button>
    </div>

    <!-- Content -->
    <div class="p-6">
      <p class="text-gray-700 dark:text-gray-300">
        {message}
      </p>
    </div>

    <!-- Footer -->
    <div class="border-t border-gray-200 dark:border-gray-700 p-6 flex justify-end gap-3">
      <Button variant="secondary" onclick={handleCancel}>
        {cancelText}
      </Button>
      <Button
        variant={(variant === 'danger' ? 'destructive' : 'primary') as any}
        onclick={handleConfirm}
      >
        {confirmText}
      </Button>
    </div>
  </div>
</div>
