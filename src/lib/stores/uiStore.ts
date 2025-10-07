import { writable } from 'svelte/store';

export type View = 'dashboard' | 'servers' | 'tools' | 'resources' | 'prompts' | 'sampling' | 'elicitation' | 'protocol' | 'collections' | 'settings';

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
  modals: {
    addServer: boolean;
    serverConfig: boolean;
    toolCall: boolean;
    profileEditor: boolean;
  };
  // Profile editor state
  editingProfileId?: string | null;
  // Tool Explorer state persistence
  selectedTool?: {
    name: string;
    serverId: string;
  };
}

const initialState: UiStoreState = {
  currentView: 'dashboard',
  sidebarCollapsed: false,
  error: undefined,
  notification: undefined,
  loading: false,
  modals: {
    addServer: false,
    serverConfig: false,
    toolCall: false,
    profileEditor: false,
  },
  editingProfileId: undefined,
  selectedTool: undefined,
};

function createUiStore() {
  const { subscribe, set, update } = writable<UiStoreState>(initialState);

  return {
    subscribe,

    // Navigate to a different view
    setView(view: View) {
      console.log('🟢 uiStore.setView called with:', view);
      update(state => {
        console.log('🟢 uiStore updating state from:', state.currentView, 'to:', view);
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

    // Modal management
    openModal(modal: keyof UiStoreState['modals']) {
      update(state => ({
        ...state,
        modals: { ...state.modals, [modal]: true },
      }));
    },

    closeModal(modal: keyof UiStoreState['modals']) {
      update(state => ({
        ...state,
        modals: { ...state.modals, [modal]: false },
      }));
    },

    closeAllModals() {
      update(state => ({
        ...state,
        modals: {
          addServer: false,
          serverConfig: false,
          toolCall: false,
          profileEditor: false,
        },
        editingProfileId: undefined,
      }));
    },

    // Profile editor management
    openProfileEditor(profileId?: string | null) {
      update(state => ({
        ...state,
        modals: { ...state.modals, profileEditor: true },
        editingProfileId: profileId,
        currentView: 'servers', // Navigate to servers view
      }));
    },

    closeProfileEditor() {
      update(state => ({
        ...state,
        modals: { ...state.modals, profileEditor: false },
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
  };
}

export const uiStore = createUiStore();