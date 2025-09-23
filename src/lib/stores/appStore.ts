/**
 * Application state store for MCP Studio
 * Manages global app lifecycle, loading states, and readiness
 */

import { writable, derived } from 'svelte/store';

export interface AppState {
  isInitializing: boolean;
  databaseReady: boolean;
  mcpManagerReady: boolean;
  initializationError: string | null;
  startupTime: number | null;
}

export interface LoadingStep {
  id: string;
  label: string;
  status: 'pending' | 'loading' | 'completed' | 'error';
  error?: string;
}

// Core app state
const initialState: AppState = {
  isInitializing: true,
  databaseReady: false,
  mcpManagerReady: false,
  initializationError: null,
  startupTime: null,
};

// Internal store state
const appState = writable(initialState);
const loadingSteps = writable<LoadingStep[]>([
  { id: 'ui', label: 'Loading user interface', status: 'completed' },
  { id: 'mcp', label: 'Initializing MCP manager', status: 'completed' },
  { id: 'database', label: 'Setting up database', status: 'loading' },
  { id: 'servers', label: 'Loading server configurations', status: 'pending' },
]);

// Export individual stores for Svelte 5 runes mode compatibility
export const appStoreState = derived(appState, $appState => $appState);
export const appStoreLoadingSteps = derived(loadingSteps, $steps => $steps);
export const appStoreIsReady = derived(appState, $state =>
  !$state.isInitializing &&
  $state.databaseReady &&
  $state.mcpManagerReady &&
  !$state.initializationError
);
export const appStoreCompletedSteps = derived(loadingSteps, $steps =>
  $steps.filter(step => step.status === 'completed').length
);
export const appStoreTotalSteps = derived(loadingSteps, $steps => $steps.length);

// Public interface
export const appStore = {
  // Read-only derived states
  state: appStoreState,
  loadingSteps: appStoreLoadingSteps,

  // Computed properties
  isReady: appStoreIsReady,
  completedSteps: appStoreCompletedSteps,
  totalSteps: appStoreTotalSteps,

  // Actions
  setInitializing(isInitializing: boolean) {
    appState.update(state => ({ ...state, isInitializing }));
  },

  setDatabaseReady(ready: boolean) {
    appState.update(state => ({ ...state, databaseReady: ready }));

    if (ready) {
      loadingSteps.update(steps =>
        steps.map(step =>
          step.id === 'database'
            ? { ...step, status: 'completed' }
            : step
        )
      );
    }
  },

  setMcpManagerReady(ready: boolean) {
    appState.update(state => ({ ...state, mcpManagerReady: ready }));
  },

  setInitializationError(error: string | null) {
    appState.update(state => ({ ...state, initializationError: error }));

    if (error) {
      loadingSteps.update(steps =>
        steps.map(step =>
          step.status === 'loading'
            ? { ...step, status: 'error', error }
            : step
        )
      );
    }
  },

  setStartupTime(time: number) {
    appState.update(state => ({ ...state, startupTime: time }));
  },

  updateLoadingStep(id: string, updates: Partial<LoadingStep>) {
    loadingSteps.update(steps =>
      steps.map(step =>
        step.id === id
          ? { ...step, ...updates }
          : step
      )
    );
  },

  markStepCompleted(id: string) {
    loadingSteps.update(steps =>
      steps.map(step =>
        step.id === id
          ? { ...step, status: 'completed' }
          : step
      )
    );
  },

  markStepLoading(id: string) {
    loadingSteps.update(steps =>
      steps.map(step =>
        step.id === id
          ? { ...step, status: 'loading' }
          : step
      )
    );
  },

  markStepError(id: string, error: string) {
    loadingSteps.update(steps =>
      steps.map(step =>
        step.id === id
          ? { ...step, status: 'error', error }
          : step
      )
    );
  },

  // Complete initialization sequence
  completeInitialization() {
    const startTime = Date.now();

    // Mark all pending steps as completed
    loadingSteps.update(steps =>
      steps.map(step =>
        step.status === 'pending' || step.status === 'loading'
          ? { ...step, status: 'completed' }
          : step
      )
    );

    appState.update(state => ({
      ...state,
      isInitializing: false,
      databaseReady: true,
      mcpManagerReady: true,
      startupTime: Date.now() - startTime,
    }));
  },

  // Reset to initial state (for development/testing)
  reset() {
    appState.set(initialState);
    loadingSteps.set([
      { id: 'ui', label: 'Loading user interface', status: 'completed' },
      { id: 'mcp', label: 'Initializing MCP manager', status: 'completed' },
      { id: 'database', label: 'Setting up database', status: 'loading' },
      { id: 'servers', label: 'Loading server configurations', status: 'pending' },
    ]);
  },
};