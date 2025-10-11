/**
 * Elicitation Store - Manages server-initiated elicitation requests
 *
 * This store listens to `elicitation_requested` events from the backend and manages
 * active elicitation requests that need user input before being sent back to servers.
 */

import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { createLogger } from '$lib/utils/logger';
import type { JsonSchema } from '$lib/types/sampling';

// ============================================================================
// TYPES
// ============================================================================

export interface ActiveElicitationRequest {
	id: string;
	protocolMessageId?: string; // For Protocol Inspector correlation
	serverId: string;
	serverName: string;
	message: string;
	requestedSchema: JsonSchema;
	timeout?: number; // seconds
	createdAt: string;
}

export interface CompletedElicitationRequest extends ActiveElicitationRequest {
	status: 'accepted' | 'declined' | 'cancelled';
	completedAt: string;

	// Full response tracking
	response?: {
		action: 'accept' | 'decline' | 'cancel';
		content?: Record<string, any>;
	};

	// Outcome tracking
	outcome?: {
		status: 'success' | 'error';
		message?: string;
	};
}

export interface ElicitationReplayTemplate {
	id: string;
	name: string;
	schema: JsonSchema;
	response: Record<string, any>;
	createdAt: string;
	lastUsed?: string;
	useCount: number;
}

interface ElicitationStoreState {
	active: ActiveElicitationRequest[];
	history: CompletedElicitationRequest[];
	replayTemplates: ElicitationReplayTemplate[];
	loading: boolean;
	error: string | null;
}

// ============================================================================
// STORE IMPLEMENTATION
// ============================================================================

const initialState: ElicitationStoreState = {
	active: [],
	history: [],
	replayTemplates: [],
	loading: false,
	error: null
};

// Load replay templates from localStorage
function loadReplayTemplates(): ElicitationReplayTemplate[] {
	try {
		const stored = localStorage.getItem('elicitation_replay_templates');
		return stored ? JSON.parse(stored) : [];
	} catch (e) {
		console.error('Failed to load elicitation replay templates:', e);
		return [];
	}
}

function saveReplayTemplates(templates: ElicitationReplayTemplate[]): void {
	try {
		localStorage.setItem('elicitation_replay_templates', JSON.stringify(templates));
	} catch (e) {
		console.error('Failed to save elicitation replay templates:', e);
	}
}

function createElicitationStore() {
	const { subscribe, set, update } = writable<ElicitationStoreState>({
		...initialState,
		replayTemplates: loadReplayTemplates()
	});
	const logger = createLogger('ElicitationStore');

	// Initialize event listener immediately
	let eventUnlisten: (() => void) | null = null;

	const initializeEventListener = async () => {
		try {
			eventUnlisten = await listen<any>('elicitation_requested', (event) => {
				const payload = event.payload;

				const request: ActiveElicitationRequest = {
					id: payload.id,
					protocolMessageId: payload.protocolMessageId,
					serverId: payload.serverId,
					serverName: payload.serverName || 'Unknown Server',
					message: payload.message,
					requestedSchema: payload.requestedSchema,
					timeout: payload.timeout,
					createdAt: new Date().toISOString()
				};

				update((state) => ({
					...state,
					active: [...state.active, request],
					error: null
				}));

				logger.info('Received elicitation request:', request.id);

				// Auto-show modal via dynamic import (avoid circular dependency)
				setTimeout(() => {
					import('./uiStore').then(({ uiStore }) => {
						uiStore.showElicitationDialog(request);
					});
				}, 0);
			});
		} catch (error) {
			logger.error('Failed to initialize elicitation event listener:', error);
		}
	};

	// Initialize on store creation
	initializeEventListener();

	return {
		subscribe,

		/**
		 * Submit elicitation response (accept with data)
		 *
		 * @param requestId - Unique request identifier
		 * @param content - The form data provided by the user
		 */
		async accept(requestId: string, content: Record<string, any>): Promise<void> {
			update((s) => ({ ...s, loading: true, error: null }));

			try {
				// Find the active request
				const state = await new Promise<ElicitationStoreState>((resolve) => {
					const unsub = subscribe(resolve);
					unsub();
				});

				const request = state.active.find((r) => r.id === requestId);
				if (!request) {
					throw new Error(`Request not found: ${requestId}`);
				}

				// Call backend to submit response
				await invoke('send_elicitation_response', {
					serverId: request.serverId,
					requestId,
					responseData: {
						action: 'accept',
						content
					}
				});

				// Move from active to history
				const completed: CompletedElicitationRequest = {
					...request,
					status: 'accepted',
					completedAt: new Date().toISOString(),
					response: {
						action: 'accept',
						content
					},
					outcome: {
						status: 'success',
						message: 'User provided input'
					}
				};

				update((state) => ({
					...state,
					active: state.active.filter((r) => r.id !== requestId),
					history: [completed, ...state.history],
					loading: false
				}));
			} catch (error) {
				update((s) => ({
					...s,
					loading: false,
					error: error instanceof Error ? error.message : String(error)
				}));

				logger.error('Failed to accept elicitation request:', error);
				throw error;
			}
		},

		/**
		 * Decline elicitation request
		 *
		 * @param requestId - Unique request identifier
		 */
		async decline(requestId: string): Promise<void> {
			update((s) => ({ ...s, loading: true, error: null }));

			try {
				const state = await new Promise<ElicitationStoreState>((resolve) => {
					const unsub = subscribe(resolve);
					unsub();
				});

				const request = state.active.find((r) => r.id === requestId);
				if (!request) {
					throw new Error(`Request not found: ${requestId}`);
				}

				// Call backend
				await invoke('send_elicitation_response', {
					serverId: request.serverId,
					requestId,
					responseData: {
						action: 'decline'
					}
				});

				const completed: CompletedElicitationRequest = {
					...request,
					status: 'declined',
					completedAt: new Date().toISOString(),
					response: {
						action: 'decline'
					},
					outcome: {
						status: 'success',
						message: 'User declined request'
					}
				};

				update((state) => ({
					...state,
					active: state.active.filter((r) => r.id !== requestId),
					history: [completed, ...state.history],
					loading: false
				}));
			} catch (error) {
				update((s) => ({
					...s,
					loading: false,
					error: error instanceof Error ? error.message : String(error)
				}));

				logger.error('Failed to decline elicitation request:', error);
				throw error;
			}
		},

		/**
		 * Cancel elicitation request (close without responding)
		 */
		async cancel(requestId: string): Promise<void> {
			update((s) => ({ ...s, loading: true, error: null }));

			try {
				const state = await new Promise<ElicitationStoreState>((resolve) => {
					const unsub = subscribe(resolve);
					unsub();
				});

				const request = state.active.find((r) => r.id === requestId);
				if (!request) {
					throw new Error(`Request not found: ${requestId}`);
				}

				// Call backend
				await invoke('send_elicitation_response', {
					serverId: request.serverId,
					requestId,
					responseData: {
						action: 'cancel'
					}
				});

				const completed: CompletedElicitationRequest = {
					...request,
					status: 'cancelled',
					completedAt: new Date().toISOString(),
					response: {
						action: 'cancel'
					},
					outcome: {
						status: 'success',
						message: 'Request cancelled'
					}
				};

				update((state) => ({
					...state,
					active: state.active.filter((r) => r.id !== requestId),
					history: [completed, ...state.history],
					loading: false
				}));
			} catch (error) {
				update((s) => ({
					...s,
					loading: false,
					error: error instanceof Error ? error.message : String(error)
				}));

				logger.error('Failed to cancel elicitation request:', error);
				throw error;
			}
		},

		/**
		 * Save a response as a replay template
		 */
		saveAsReplayTemplate(
			completedRequest: CompletedElicitationRequest,
			name?: string
		): void {
			if (!completedRequest.response?.content) {
				throw new Error('Cannot save template: no response content');
			}

			const template: ElicitationReplayTemplate = {
				id: `elicit-replay-${Date.now()}`,
				name: name || `Input for ${completedRequest.serverName}`,
				schema: completedRequest.requestedSchema,
				response: completedRequest.response.content,
				createdAt: new Date().toISOString(),
				useCount: 0
			};

			update((state) => {
				const newTemplates = [template, ...state.replayTemplates];
				saveReplayTemplates(newTemplates);
				return { ...state, replayTemplates: newTemplates };
			});
		},

		/**
		 * Use a replay template to respond to an active request
		 */
		async useReplayTemplate(requestId: string, templateId: string): Promise<void> {
			const state = await new Promise<ElicitationStoreState>((resolve) => {
				const unsub = subscribe(resolve);
				unsub();
			});

			const template = state.replayTemplates.find((t) => t.id === templateId);
			if (!template) {
				throw new Error(`Template not found: ${templateId}`);
			}

			// Update usage stats
			update((s) => {
				const updatedTemplates = s.replayTemplates.map((t) =>
					t.id === templateId
						? { ...t, lastUsed: new Date().toISOString(), useCount: t.useCount + 1 }
						: t
				);
				saveReplayTemplates(updatedTemplates);
				return { ...s, replayTemplates: updatedTemplates };
			});

			// Submit the template response
			await this.accept(requestId, template.response);
		},

		/**
		 * Delete a replay template
		 */
		deleteReplayTemplate(templateId: string): void {
			update((state) => {
				const newTemplates = state.replayTemplates.filter((t) => t.id !== templateId);
				saveReplayTemplates(newTemplates);
				return { ...state, replayTemplates: newTemplates };
			});
		},

		/**
		 * Get message preview for UI
		 */
		getMessagePreview(request: ActiveElicitationRequest | CompletedElicitationRequest): string {
			return request.message.substring(0, 150) + (request.message.length > 150 ? '...' : '');
		},

		/**
		 * Get response preview for UI
		 */
		getResponsePreview(completed: CompletedElicitationRequest): string {
			if (completed.response?.content) {
				const contentStr = JSON.stringify(completed.response.content);
				return contentStr.substring(0, 150) + (contentStr.length > 150 ? '...' : '');
			}
			return completed.status === 'accepted'
				? 'Accepted'
				: completed.status === 'declined'
					? 'Declined'
					: 'Cancelled';
		},

		/**
		 * Clear history
		 */
		clearHistory(): void {
			update((s) => ({ ...s, history: [] }));
		},

		/**
		 * Clear error
		 */
		clearError(): void {
			update((s) => ({ ...s, error: null }));
		},

		/**
		 * Reset store
		 */
		reset(): void {
			set({ ...initialState, replayTemplates: loadReplayTemplates() });
		},

		/**
		 * Cleanup
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

export const elicitationStore = createElicitationStore();

export const activeElicitationRequests = derived(elicitationStore, ($store) => $store.active);

export const elicitationHistory = derived(elicitationStore, ($store) => $store.history);

export const hasActiveElicitationRequests = derived(
	elicitationStore,
	($store) => $store.active.length > 0
);

export const isProcessingElicitation = derived(elicitationStore, ($store) => $store.loading);

export const elicitationError = derived(elicitationStore, ($store) => $store.error);

export const elicitationReplayTemplates = derived(
	elicitationStore,
	($store) => $store.replayTemplates
);
