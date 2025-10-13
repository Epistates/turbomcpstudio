/**
 * Sampling Store - Manages server-initiated sampling requests with HITL approval
 *
 * This store listens to `sampling_requested` events from the backend and manages
 * pending sampling requests that need user approval before being forwarded to LLMs.
 */

import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { createLogger } from '$lib/utils/logger';

// ============================================================================
// TYPES
// ============================================================================

export interface PendingSamplingRequest {
	requestId: string;
	protocolMessageId?: string;  // For Protocol Inspector correlation
	serverId: string;
	serverName: string;
	request: SamplingRequestDetails;
	estimatedCost?: number;
	estimatedTokens?: number;
	createdAt: string;
	retryCount?: number;  // Retry detection from backend
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

	// Full response tracking
	response?: {
		mode: 'manual' | 'ai' | 'mock';
		content: CreateMessageResult;
	};

	// Outcome tracking
	outcome?: {
		status: 'success' | 'error';
		message?: string;
	};

	// Legacy fields
	result?: any;
	error?: string;
}

export interface CreateMessageResult {
	role: 'assistant';
	content: {
		type: 'text' | 'image';
		text?: string;
		data?: string;
		mimeType?: string;
	};
	model: string;
	stopReason: 'endTurn' | 'stopSequence' | 'maxTokens';
}

export interface ReplayTemplate {
	id: string;
	name: string;
	response: CreateMessageResult;
	createdAt: string;
	lastUsed?: string;
	useCount: number;
}

interface SamplingStoreState {
	pending: PendingSamplingRequest[];
	history: CompletedSamplingRequest[];
	replayTemplates: ReplayTemplate[];
	loading: boolean;
	error: string | null;
}

// ============================================================================
// STORE IMPLEMENTATION
// ============================================================================

const initialState: SamplingStoreState = {
	pending: [],
	history: [],
	replayTemplates: [],
	loading: false,
	error: null
};

// Load replay templates from localStorage
function loadReplayTemplates(): ReplayTemplate[] {
	try {
		const stored = localStorage.getItem('sampling_replay_templates');
		return stored ? JSON.parse(stored) : [];
	} catch (e) {
		console.error('Failed to load replay templates:', e);
		return [];
	}
}

function saveReplayTemplates(templates: ReplayTemplate[]): void {
	try {
		localStorage.setItem('sampling_replay_templates', JSON.stringify(templates));
	} catch (e) {
		console.error('Failed to save replay templates:', e);
	}
}

function createSamplingStore() {
	const { subscribe, set, update } = writable<SamplingStoreState>({
		...initialState,
		replayTemplates: loadReplayTemplates()
	});
	const logger = createLogger('SamplingStore');

	// Initialize event listener immediately
	let eventUnlisten: (() => void) | null = null;

	const initializeEventListener = async () => {
		try {
			eventUnlisten = await listen<PendingSamplingRequest>('sampling_requested', (event) => {
				const request = event.payload;

				logger.info('📥 [DEBUG] Received sampling_requested event:', {
					requestId: request.requestId,
					serverId: request.serverId,
					serverName: request.serverName,
					timestamp: new Date().toISOString()
				});

				update((state) => ({
					...state,
					pending: [...state.pending, request],
					error: null
				}));

				// Auto-show modal via dynamic import (avoid circular dependency)
				setTimeout(() => {
					import('./uiStore').then(({ uiStore }) => {
						logger.info('🔔 [DEBUG] Showing sampling modal for request:', request.requestId);
						uiStore.showSamplingApproval(request);
					});
				}, 0);
			});
		} catch (error) {
			logger.error('Failed to initialize sampling event listener:', error);
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
			} catch (error) {
				update((s) => ({
					...s,
					loading: false,
					error: error instanceof Error ? error.message : String(error)
				}));

				logger.error('Failed to approve sampling request:', error);
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
			logger.info('❌ [DEBUG] Rejecting sampling request:', { requestId, reason });
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

				logger.info('📤 [DEBUG] Calling backend reject_sampling_request');
				// Call backend to reject
				await invoke('reject_sampling_request', {
					requestId,
					reason
				});
				logger.info('✅ [DEBUG] Backend rejection completed');

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
				logger.info('📋 [DEBUG] Moved request to history');
			} catch (error) {
				const errorMessage = error instanceof Error ? error.message : String(error);

				// Handle "no pending channel" error - this means the request is stale
				if (errorMessage.includes('No pending channel for request')) {
					logger.warn('⚠️ [DEBUG] No pending channel - request is stale, removing from pending list:', requestId);

					// Remove from pending list without adding to history since the backend already processed it
					update((state) => ({
						...state,
						pending: state.pending.filter((r) => r.requestId !== requestId),
						loading: false,
						error: 'Request no longer valid (already processed or timed out)'
					}));

					// Don't throw - just return, error is stored in state
					return;
				}

				update((s) => ({
					...s,
					loading: false,
					error: errorMessage
				}));

				logger.error('Failed to reject sampling request:', error);
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
		 * Submit manual response for sampling request (testing tool feature)
		 *
		 * Bypasses LLM and provides a direct manual response
		 *
		 * @param requestId - Unique request identifier
		 * @param manualResponse - The manual response to send back (CreateMessageResult format)
		 */
		async submitManual(requestId: string, manualResponse: CreateMessageResult): Promise<void> {
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

				// Call backend to submit manual response
				await invoke('submit_manual_sampling_response', {
					requestId,
					manualResponse
				});

				// Move from pending to history with full response tracking
				const completed: CompletedSamplingRequest = {
					...request,
					status: 'approved',
					completedAt: new Date().toISOString(),
					response: {
						mode: 'manual',
						content: manualResponse
					},
					outcome: {
						status: 'success',
						message: 'Manual response provided'
					},
					result: manualResponse // legacy
				};

				update((state) => ({
					...state,
					pending: state.pending.filter((r) => r.requestId !== requestId),
					history: [completed, ...state.history],
					loading: false
				}));
			} catch (error) {
				const errorMessage = error instanceof Error ? error.message : String(error);

				// Handle "no pending channel" error - this means the request is stale
				if (errorMessage.includes('No pending channel for request')) {
					logger.warn('⚠️ [DEBUG] No pending channel for manual response - request is stale, removing from pending list:', requestId);

					// Remove from pending list without adding to history since the backend already processed it
					update((state) => ({
						...state,
						pending: state.pending.filter((r) => r.requestId !== requestId),
						loading: false,
						error: 'Request no longer valid (already processed or timed out)'
					}));

					// Don't throw - just return, error is stored in state
					return;
				}

				update((s) => ({
					...s,
					loading: false,
					error: errorMessage
				}));

				logger.error('Failed to submit manual sampling response:', error);
				throw error;
			}
		},

		/**
		 * Clear history
		 */
		clearHistory(): void {
			update((s) => ({ ...s, history: [] }));
		},

		/**
		 * Save a response as a replay template for quick reuse
		 *
		 * @param completedRequest - The completed request to save
		 * @param name - Optional custom name for the template
		 */
		saveAsReplayTemplate(completedRequest: CompletedSamplingRequest, name?: string): void {
			if (!completedRequest.response) {
				throw new Error('Cannot save template: no response data');
			}

			const template: ReplayTemplate = {
				id: `replay-${Date.now()}`,
				name: name || `Response from ${completedRequest.serverName}`,
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
		 * Use a replay template to respond to a pending request
		 *
		 * @param requestId - The request to respond to
		 * @param templateId - The template to use
		 */
		async useReplayTemplate(requestId: string, templateId: string): Promise<void> {
			const state = await new Promise<SamplingStoreState>((resolve) => {
				const unsub = subscribe(resolve);
				unsub();
			});

			const template = state.replayTemplates.find((t) => t.id === templateId);
			if (!template) {
				throw new Error(`Template not found: ${templateId}`);
			}

			// Update template usage stats
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
			await this.submitManual(requestId, template.response);
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
		 * Get preview text for a sampling request (for UI display)
		 */
		getRequestPreview(request: PendingSamplingRequest | CompletedSamplingRequest): string {
			const firstMessage = request.request.messages[0];
			if (firstMessage && typeof firstMessage.content === 'object' && 'text' in firstMessage.content) {
				const text = firstMessage.content.text || '';
				return text.substring(0, 150) + (text.length > 150 ? '...' : '');
			}
			return 'No content';
		},

		/**
		 * Get response preview text for display
		 */
		getResponsePreview(completed: CompletedSamplingRequest): string {
			if (completed.response?.content) {
				const content = completed.response.content.content;
				if (content.type === 'text' && content.text) {
					return content.text.substring(0, 150) + (content.text.length > 150 ? '...' : '');
				}
			}
			return completed.status === 'approved' ? 'Approved' : 'Rejected';
		},

		/**
		 * Reset store to initial state
		 */
		reset(): void {
			set({ ...initialState, replayTemplates: loadReplayTemplates() });
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

export const replayTemplates = derived(samplingStore, ($store) => $store.replayTemplates);
