/**
 * Modal Utilities - Comprehensive modal management
 *
 * Features:
 * - Keyboard shortcuts (Escape)
 * - Click-outside-to-close
 * - Focus trap
 * - Scroll lock
 */

/**
 * Creates an Escape key handler for closing modals
 */
export function createModalEscapeHandler(
  onClose: () => void,
  enabled = true
): (event: KeyboardEvent) => void {
  return (event: KeyboardEvent) => {
    if (enabled && event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    }
  };
}

/**
 * Creates a click-outside handler for closing modals
 */
export function createModalOutsideClickHandler(
  modalElement: HTMLElement | null,
  onClose: () => void,
  enabled = true
): (event: MouseEvent) => void {
  return (event: MouseEvent) => {
    if (!enabled || !modalElement) return;

    // Check if click is on the backdrop (not the modal content)
    if (event.target === modalElement) {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    }
  };
}

/**
 * Locks body scroll when modal is open
 */
export function lockBodyScroll(): () => void {
  const originalOverflow = document.body.style.overflow;
  const originalPaddingRight = document.body.style.paddingRight;

  // Get scrollbar width
  const scrollbarWidth = window.innerWidth - document.documentElement.clientWidth;

  // Apply scroll lock
  document.body.style.overflow = 'hidden';
  if (scrollbarWidth > 0) {
    document.body.style.paddingRight = `${scrollbarWidth}px`;
  }

  // Return cleanup function
  return () => {
    document.body.style.overflow = originalOverflow;
    document.body.style.paddingRight = originalPaddingRight;
  };
}

/**
 * Focus trap for modal accessibility
 */
export function createFocusTrap(
  modalElement: HTMLElement | null
): () => void {
  if (!modalElement) return () => {};

  const focusableElements = modalElement.querySelectorAll<HTMLElement>(
    'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
  );

  const firstFocusable = focusableElements[0];
  const lastFocusable = focusableElements[focusableElements.length - 1];

  // Focus first element
  firstFocusable?.focus();

  const handleTabKey = (event: KeyboardEvent) => {
    if (event.key !== 'Tab') return;

    if (event.shiftKey) {
      // Shift + Tab
      if (document.activeElement === firstFocusable) {
        event.preventDefault();
        lastFocusable?.focus();
      }
    } else {
      // Tab
      if (document.activeElement === lastFocusable) {
        event.preventDefault();
        firstFocusable?.focus();
      }
    }
  };

  modalElement.addEventListener('keydown', handleTabKey);

  // Return cleanup function
  return () => {
    modalElement.removeEventListener('keydown', handleTabKey);
  };
}

/**
 * Comprehensive modal manager
 */
export class ModalManager {
  private activeModals = new Set<string>();
  private scrollUnlocks: Map<string, () => void> = new Map();
  private focusTraps: Map<string, () => void> = new Map();

  /**
   * Register a modal as active
   */
  register(
    modalId: string,
    modalElement: HTMLElement | null,
    options: {
      lockScroll?: boolean;
      trapFocus?: boolean;
    } = {}
  ): void {
    if (this.activeModals.has(modalId)) {
      console.warn(`⚠️ Modal "${modalId}" is already registered`);
      return;
    }

    this.activeModals.add(modalId);

    // Lock scroll if requested
    if (options.lockScroll) {
      const unlock = lockBodyScroll();
      this.scrollUnlocks.set(modalId, unlock);
    }

    // Trap focus if requested
    if (options.trapFocus && modalElement) {
      const cleanup = createFocusTrap(modalElement);
      this.focusTraps.set(modalId, cleanup);
    }

    console.log(`✅ Modal "${modalId}" registered (${this.activeModals.size} active)`);
  }

  /**
   * Unregister a modal
   */
  unregister(modalId: string): void {
    if (!this.activeModals.has(modalId)) {
      return;
    }

    this.activeModals.delete(modalId);

    // Unlock scroll
    const unlock = this.scrollUnlocks.get(modalId);
    if (unlock) {
      unlock();
      this.scrollUnlocks.delete(modalId);
    }

    // Release focus trap
    const focusTrap = this.focusTraps.get(modalId);
    if (focusTrap) {
      focusTrap();
      this.focusTraps.delete(modalId);
    }

    console.log(`✅ Modal "${modalId}" unregistered (${this.activeModals.size} active)`);
  }

  /**
   * Check if a modal is active
   */
  isActive(modalId: string): boolean {
    return this.activeModals.has(modalId);
  }

  /**
   * Get count of active modals
   */
  getActiveCount(): number {
    return this.activeModals.size;
  }

  /**
   * Get all active modal IDs
   */
  getActiveModals(): string[] {
    return Array.from(this.activeModals);
  }

  /**
   * Close all modals (emergency cleanup)
   */
  closeAll(): void {
    const modalIds = Array.from(this.activeModals);
    modalIds.forEach(id => this.unregister(id));
  }
}

/**
 * Global modal manager instance
 */
export const globalModalManager = new ModalManager();
