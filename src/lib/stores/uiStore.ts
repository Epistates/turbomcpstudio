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
    samplingApproval: boolean;
  };
  // Sampling approval modal state
  pendingSamplingRequest?: any;
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
    samplingApproval: false,
  },
  pendingSamplingRequest: undefined,
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
          samplingApproval: false,
        },
        editingProfileId: undefined,
        pendingSamplingRequest: undefined,
      }));
    },

    // Sampling approval modal management
    showSamplingApproval(request: any) {
      update(state => ({
        ...state,
        modals: { ...state.modals, samplingApproval: true },
        pendingSamplingRequest: request,
      }));
    },

    closeSamplingApproval() {
      update(state => ({
        ...state,
        modals: { ...state.modals, samplingApproval: false },
        pendingSamplingRequest: undefined,
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

    // Navigate to Protocol Inspector and optionally select a message
    jumpToProtocolInspector(messageId?: string) {
      this.setView('protocol');
      // TODO: If messageId provided, scroll to and select that message in timeline
      // This would require protocol store integration
      if (messageId) {
        console.log('📍 Jumping to protocol message:', messageId);
      }
    },
  };
}

export const uiStore = createUiStore();