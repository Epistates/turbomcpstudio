/**
 * Sampling Store - Manages server-initiated sampling requests with HITL approval
 *
 * This store listens to `sampling_requested` events from the backend and manages
 * pending sampling requests that need user approval before being forwarded to LLMs.
 */

import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// ============================================================================
// TYPES
// ============================================================================

export interface PendingSamplingRequest {
	requestId: string;
	serverId: string;
	serverName: string;
	request: SamplingRequestDetails;
	estimatedCost?: number;
	estimatedTokens?: number;
	createdAt: string;
}

export interface SamplingRequestDetails {
	messages: SamplingMessage[];
	modelPreferences?: ModelPreferences;
	systemPrompt?: string;
	includeContext?: 'none' | 'thisServer' | 'allServers';
	temperature?: number;
	maxTokens?: number;
	stopSequences?: string[];
}

export interface SamplingMessage {
	role: 'user' | 'assistant';
	content: MessageContent;
	metadata?: Record<string, any>;
}

export interface MessageContent {
	type: 'text' | 'image' | 'audio';
	text?: string;
	data?: string; // base64
	mimeType?: string;
}

export interface ModelPreferences {
	hints?: ModelHint[];
	costPriority?: number; // 0.0-1.0
	speedPriority?: number; // 0.0-1.0
	intelligencePriority?: number; // 0.0-1.0
}

export interface ModelHint {
	name?: string; // Substring matching: "gpt-4" matches "gpt-4-turbo"
}

export interface CompletedSamplingRequest extends PendingSamplingRequest {
	status: 'approved' | 'rejected';
	completedAt: string;
	result?: any;
	error?: string;
}

interface SamplingStoreState {
	pending: PendingSamplingRequest[];
	history: CompletedSamplingRequest[];
	loading: boolean;
	error: string | null;
}

// ============================================================================
// STORE IMPLEMENTATION
// ============================================================================

const initialState: SamplingStoreState = {
	pending: [],
	history: [],
	loading: false,
	error: null
};

function createSamplingStore() {
	const { subscribe, set, update } = writable<SamplingStoreState>(initialState);

	// Initialize event listener immediately
	let eventUnlisten: (() => void) | null = null;

	const initializeEventListener = async () => {
		try {
			eventUnlisten = await listen<PendingSamplingRequest>('sampling_requested', (event) => {
				const request = event.payload;

				console.log('🎯 Sampling request received:', request);

				update((state) => ({
					...state,
					pending: [...state.pending, request],
					error: null
				}));

				// Auto-show modal via dynamic import (avoid circular dependency)
				setTimeout(() => {
					import('./uiStore').then(({ uiStore }) => {
						uiStore.showSamplingApproval(request);
					});
				}, 0);
			});

			console.log('✅ Sampling event listener initialized');
		} catch (error) {
			console.error('❌ Failed to initialize sampling event listener:', error);
		}
	};

	// Initialize on store creation
	initializeEventListener();

	return {
		subscribe,

		/**
		 * Approve a sampling request and forward to LLM
		 *
		 * @param requestId - Unique request identifier
		 * @param modifiedRequest - Optional modified request (if user edited)
		 */
		async approve(requestId: string, modifiedRequest?: SamplingRequestDetails): Promise<void> {
			update((s) => ({ ...s, loading: true, error: null }));

			try {
				// Find the original request
				const state = await new Promise<SamplingStoreState>((resolve) => {
					const unsub = subscribe(resolve);
					unsub();
				});

				const request = state.pending.find((r) => r.requestId === requestId);
				if (!request) {
					throw new Error(`Request not found: ${requestId}`);
				}

				// Call backend to approve and forward to LLM
				await invoke('approve_sampling_request', {
					requestId,
					approvedRequest: modifiedRequest || request.request
				});

				// Move from pending to history
				const completed: CompletedSamplingRequest = {
					...request,
					status: 'approved',
					completedAt: new Date().toISOString()
				};

				update((state) => ({
					...state,
					pending: state.pending.filter((r) => r.requestId !== requestId),
					history: [completed, ...state.history],
					loading: false
				}));

				console.log('✅ Sampling request approved:', requestId);
			} catch (error) {
				update((s) => ({
					...s,
					loading: false,
					error: error instanceof Error ? error.message : String(error)
				}));

				console.error('❌ Failed to approve sampling request:', error);
				throw error;
			}
		},

		/**
		 * Reject a sampling request
		 *
		 * @param requestId - Unique request identifier
		 * @param reason - Reason for rejection
		 */
		async reject(requestId: string, reason: string): Promise<void> {
			update((s) => ({ ...s, loading: true, error: null }));

			try {
				// Find the original request
				const state = await new Promise<SamplingStoreState>((resolve) => {
					const unsub = subscribe(resolve);
					unsub();
				});

				const request = state.pending.find((r) => r.requestId === requestId);
				if (!request) {
					throw new Error(`Request not found: ${requestId}`);
				}

				// Call backend to reject
				await invoke('reject_sampling_request', {
					requestId,
					reason
				});

				// Move from pending to history
				const completed: CompletedSamplingRequest = {
					...request,
					status: 'rejected',
					completedAt: new Date().toISOString(),
					error: reason
				};

				update((state) => ({
					...state,
					pending: state.pending.filter((r) => r.requestId !== requestId),
					history: [completed, ...state.history],
					loading: false
				}));

				console.log('❌ Sampling request rejected:', requestId, reason);
			} catch (error) {
				update((s) => ({
					...s,
					loading: false,
					error: error instanceof Error ? error.message : String(error)
				}));

				console.error('❌ Failed to reject sampling request:', error);
				throw error;
			}
		},

		/**
		 * Clear error state
		 */
		clearError(): void {
			update((s) => ({ ...s, error: null }));
		},

		/**
		 * Clear history
		 */
		clearHistory(): void {
			update((s) => ({ ...s, history: [] }));
		},

		/**
		 * Reset store to initial state
		 */
		reset(): void {
			set(initialState);
		},

		/**
		 * Cleanup (call on component unmount if needed)
		 */
		destroy(): void {
			if (eventUnlisten) {
				eventUnlisten();
				eventUnlisten = null;
			}
		}
	};
}

// ============================================================================
// STORE INSTANCE & DERIVED STORES
// ============================================================================

export const samplingStore = createSamplingStore();

// Derived stores for convenient access
export const pendingSamplingRequests = derived(samplingStore, ($store) => $store.pending);

export const samplingHistory = derived(samplingStore, ($store) => $store.history);

export const hasPendingSamplingRequests = derived(
	samplingStore,
	($store) => $store.pending.length > 0
);

export const isProcessingSampling = derived(samplingStore, ($store) => $store.loading);

export const samplingError = derived(samplingStore, ($store) => $store.error);
