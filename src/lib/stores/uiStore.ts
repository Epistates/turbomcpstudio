import { writable } from 'svelte/store';

export type View = 'dashboard' | 'servers' | 'tools' | 'resources' | 'prompts' | 'sampling' | 'elicitation' | 'protocol' | 'collections' | 'settings';

/**
 * ✅ NEW: Unified modal state structure
 * Each modal has: open (visibility), loading (operation state), requestId (deduplication)
 */
export interface ModalState {
  open: boolean;
  loading: boolean;
  requestId: string | null;
}

// ✅ Export the type for external use
export type { ModalState as ModalStateType };

interface UiStoreState {
  currentView: View;
  sidebarCollapsed: boolean;
  error?: string;
  notification?: {
    type: 'success' | 'error' | 'warning' | 'info';
    message: string;
    timeout?: number;
  };
  loading: boolean;
  // ✅ NEW: Modal state with loading and request tracking
  modals: {
    addServer: ModalState;
    serverConfig: ModalState;
    toolCall: ModalState;
    profileEditor: ModalState;
    samplingApproval: ModalState;
    elicitationDialog: ModalState;
  };
  // Sampling approval modal state
  pendingSamplingRequest?: any;
  // Elicitation dialog state
  pendingElicitationRequest?: any;
  // Profile editor state
  editingProfileId?: string | null;
  // Tool Explorer state persistence
  selectedTool?: {
    name: string;
    serverId: string;
  };
}

/**
 * ✅ NEW: Helper to create initial modal state
 */
const createModalState = (): ModalState => ({
  open: false,
  loading: false,
  requestId: null,
});

const initialState: UiStoreState = {
  currentView: 'dashboard',
  sidebarCollapsed: false,
  error: undefined,
  notification: undefined,
  loading: false,
  // ✅ NEW: Initialize all modals with proper state structure
  modals: {
    addServer: createModalState(),
    serverConfig: createModalState(),
    toolCall: createModalState(),
    profileEditor: createModalState(),
    samplingApproval: createModalState(),
    elicitationDialog: createModalState(),
  },
  pendingSamplingRequest: undefined,
  pendingElicitationRequest: undefined,
  editingProfileId: undefined,
  selectedTool: undefined,
};

function createUiStore() {
  const { subscribe, set, update } = writable<UiStoreState>(initialState);

  return {
    subscribe,

    // Navigate to a different view
    setView(view: View) {
      update(state => {
        return { ...state, currentView: view };
      });
    },

    // Toggle sidebar
    toggleSidebar() {
      update(state => ({ ...state, sidebarCollapsed: !state.sidebarCollapsed }));
    },

    // Set loading state
    setLoading(loading: boolean) {
      update(state => ({ ...state, loading }));
    },

    // Set error message
    setError(error: string | undefined) {
      update(state => ({ ...state, error }));
    },

    // Clear error
    clearError() {
      update(state => ({ ...state, error: undefined }));
    },

    // Show notification
    showNotification(
      type: 'success' | 'error' | 'warning' | 'info',
      message: string,
      timeout = 5000
    ) {
      update(state => ({
        ...state,
        notification: { type, message, timeout },
      }));

      if (timeout > 0) {
        setTimeout(() => {
          update(state => ({ ...state, notification: undefined }));
        }, timeout);
      }
    },

    // Clear notification
    clearNotification() {
      update(state => ({ ...state, notification: undefined }));
    },

    // ✅ NEW: Enhanced modal management with loading state
    openModal(modal: keyof UiStoreState['modals']) {
      update(state => ({
        ...state,
        modals: {
          ...state.modals,
          [modal]: { open: true, loading: false, requestId: null },
        },
      }));
    },

    closeModal(modal: keyof UiStoreState['modals'], force = false) {
      update(state => {
        const modalState = state.modals[modal];

        // ✅ NEW: Prevent closing if loading (unless forced)
        if (modalState.loading && !force) {
          console.warn(`⚠️ Cannot close ${modal} modal while loading. Use force=true to override.`);
          return state;
        }

        return {
          ...state,
          modals: {
            ...state.modals,
            [modal]: { open: false, loading: false, requestId: null },
          },
        };
      });
    },

    // ✅ NEW: Set modal loading state with request tracking
    setModalLoading(
      modal: keyof UiStoreState['modals'],
      loading: boolean,
      requestId?: string
    ) {
      update(state => ({
        ...state,
        modals: {
          ...state.modals,
          [modal]: {
            ...state.modals[modal],
            loading,
            requestId: requestId || null,
          },
        },
      }));
    },

    // ✅ NEW: Check if modal is loading
    isModalLoading(modal: keyof UiStoreState['modals']): boolean {
      let isLoading = false;
      subscribe(state => {
        isLoading = state.modals[modal].loading;
      })(); // Immediately unsubscribe
      return isLoading;
    },

    closeAllModals() {
      update(state => ({
        ...state,
        modals: {
          addServer: createModalState(),
          serverConfig: createModalState(),
          toolCall: createModalState(),
          profileEditor: createModalState(),
          samplingApproval: createModalState(),
          elicitationDialog: createModalState(),
        },
        editingProfileId: undefined,
        pendingSamplingRequest: undefined,
        pendingElicitationRequest: undefined,
      }));
    },

    // ✅ NEW: Emergency force close all modals (ignores loading state)
    forceCloseAllModals() {
      console.warn('🚨 Force closing all modals (emergency escape)');
      update(state => ({
        ...state,
        modals: {
          addServer: createModalState(),
          serverConfig: createModalState(),
          toolCall: createModalState(),
          profileEditor: createModalState(),
          samplingApproval: createModalState(),
          elicitationDialog: createModalState(),
        },
        editingProfileId: undefined,
        pendingSamplingRequest: undefined,
        pendingElicitationRequest: undefined,
        loading: false,
      }));
    },

    // Sampling approval modal management
    showSamplingApproval(request: any) {
      update(state => ({
        ...state,
        currentView: 'sampling', // Auto-navigate to sampling tab
        modals: {
          ...state.modals,
          samplingApproval: { open: true, loading: false, requestId: null },
        },
        pendingSamplingRequest: request,
      }));
    },

    closeSamplingApproval() {
      update(state => ({
        ...state,
        modals: {
          ...state.modals,
          samplingApproval: { open: false, loading: false, requestId: null },
        },
        pendingSamplingRequest: undefined,
      }));
    },

    // Elicitation dialog management
    showElicitationDialog(request: any) {
      update(state => ({
        ...state,
        currentView: 'elicitation', // Auto-navigate to elicitation tab
        modals: {
          ...state.modals,
          elicitationDialog: { open: true, loading: false, requestId: null },
        },
        pendingElicitationRequest: request,
      }));
    },

    closeElicitationDialog() {
      update(state => ({
        ...state,
        modals: {
          ...state.modals,
          elicitationDialog: { open: false, loading: false, requestId: null },
        },
        pendingElicitationRequest: undefined,
      }));
    },

    // Profile editor management
    openProfileEditor(profileId?: string | null) {
      update(state => ({
        ...state,
        modals: {
          ...state.modals,
          profileEditor: { open: true, loading: false, requestId: null },
        },
        editingProfileId: profileId,
        currentView: 'servers', // Navigate to servers view
      }));
    },

    closeProfileEditor() {
      update(state => ({
        ...state,
        modals: {
          ...state.modals,
          profileEditor: { open: false, loading: false, requestId: null },
        },
        editingProfileId: undefined,
      }));
    },

    // Helper methods
    showSuccess(message: string) {
      this.showNotification('success', message);
    },

    showError(message: string) {
      this.showNotification('error', message);
    },

    showWarning(message: string) {
      this.showNotification('warning', message);
    },

    showInfo(message: string) {
      this.showNotification('info', message);
    },

    // Tool Explorer state management
    setSelectedTool(toolName: string, serverId: string) {
      update(state => ({
        ...state,
        selectedTool: { name: toolName, serverId },
      }));
    },

    clearSelectedTool() {
      update(state => ({
        ...state,
        selectedTool: undefined,
      }));
    },

    // Navigate to Protocol Inspector and optionally select a message
    jumpToProtocolInspector(messageId?: string) {
      this.setView('protocol');
      // TODO: If messageId provided, scroll to and select that message in timeline
      // This would require protocol store integration
    },
  };
}

export const uiStore = createUiStore();