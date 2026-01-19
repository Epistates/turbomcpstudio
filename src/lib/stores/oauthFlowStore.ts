/**
 * OAuth Flow Store
 *
 * Manages OAuth authorization flows with real-time state tracking
 * and visual debugging capabilities.
 */

import { invoke } from '@tauri-apps/api/core';
import { writable, derived, get } from 'svelte/store';

export interface OAuthFlowStep {
	step_type: string;
	description: string;
	timestamp: string;
	http_request?: {
		method: string;
		url: string;
		headers: Record<string, string>;
		body?: string;
	};
	http_response?: {
		status: number;
		headers: Record<string, string>;
		body?: string;
	};
	error?: string;
}

export interface OAuthFlow {
	flow_id: string;
	server_id: number;
	config: OAuthConfig;
	state: 'Initializing' | 'AwaitingUserAuth' | 'ExchangingCode' | 'Complete' | 'Failed' | 'Cancelled';
	steps: OAuthFlowStep[];
	state_param?: string;
	pkce_verifier?: string;
	started_at: string;
	completed_at?: string;
	error?: {
		code: string;
		description: string;
	};
}

export interface OAuthConfig {
	protocol_version: string;
	auth_server_url: string;
	token_endpoint?: string;
	client_id?: string;
	client_secret?: string;
	redirect_uri: string;
	scopes: string[];
	resource_uri: string;
	use_pkce: boolean;
	use_dpop: boolean;
	metadata?: Record<string, unknown>;
}

export interface OAuthMetadata {
	auth_server?: {
		issuer: string;
		authorization_endpoint: string;
		token_endpoint: string;
		jwks_uri?: string;
		scopes_supported?: string[];
		response_types_supported?: string[];
		grant_types_supported?: string[];
		code_challenge_methods_supported?: string[];
		token_endpoint_auth_methods_supported?: string[];
		dpop_signing_alg_values_supported?: string[];
	};
	protected_resource?: {
		resource: string;
		authorization_servers: string[];
		scopes_supported?: string[];
		bearer_methods_supported?: string[];
	};
	discovery_method: string;
	discovered_at: string;
}

// Active flows indexed by flow_id
const activeFlows = writable<Map<string, OAuthFlow>>(new Map());

// Currently selected flow for inspection
const selectedFlowId = writable<string | null>(null);

// Polling intervals for active flows
const pollingIntervals = new Map<string, ReturnType<typeof setInterval>>();

/**
 * Start a new OAuth authorization flow
 */
export async function startAuthorizationFlow(
	serverId: number,
	config: OAuthConfig
): Promise<string> {
	try {
		const flowId = await invoke<string>('start_oauth_authorization_flow', {
			serverId,
			config
		});

		// Start polling for flow status updates
		startFlowPolling(flowId);

		// Auto-select this flow for inspection
		selectedFlowId.set(flowId);

		return flowId;
	} catch (error) {
		console.error('Failed to start OAuth flow:', error);
		throw error;
	}
}

/**
 * Get current status of an OAuth flow
 */
export async function getFlowStatus(flowId: string): Promise<OAuthFlow> {
	return await invoke<OAuthFlow>('get_oauth_flow_status', { flowId });
}

/**
 * Cancel an active OAuth flow
 */
export async function cancelFlow(flowId: string): Promise<void> {
	try {
		await invoke('cancel_oauth_flow', { flowId });
		stopFlowPolling(flowId);

		// Update local state
		activeFlows.update((flows) => {
			const flow = flows.get(flowId);
			if (flow) {
				flow.state = 'Cancelled';
				flows.set(flowId, flow);
			}
			return flows;
		});
	} catch (error) {
		console.error('Failed to cancel OAuth flow:', error);
		throw error;
	}
}

/**
 * Discover OAuth metadata for a server
 */
export async function discoverMetadata(
	serverUrl: string,
	protocolVersion: string = '2025-06-18'
): Promise<OAuthMetadata> {
	return await invoke<OAuthMetadata>('discover_oauth_metadata', {
		serverUrl,
		protocolVersion
	});
}

/**
 * Start polling for flow status updates (every 500ms)
 */
function startFlowPolling(flowId: string): void {
	// Clear existing interval if any
	stopFlowPolling(flowId);

	const interval = setInterval(async () => {
		try {
			const flow = await getFlowStatus(flowId);

			// Update local state
			activeFlows.update((flows) => {
				flows.set(flowId, flow);
				return flows;
			});

			// Stop polling if flow is complete/failed/cancelled
			if (['Complete', 'Failed', 'Cancelled'].includes(flow.state)) {
				stopFlowPolling(flowId);
			}
		} catch (error) {
			console.error(`Failed to poll flow ${flowId}:`, error);
			stopFlowPolling(flowId);
		}
	}, 500);

	pollingIntervals.set(flowId, interval);
}

/**
 * Stop polling for a flow
 */
function stopFlowPolling(flowId: string): void {
	const interval = pollingIntervals.get(flowId);
	if (interval) {
		clearInterval(interval);
		pollingIntervals.delete(flowId);
	}
}

/**
 * Clear all completed/failed flows
 */
export function clearCompletedFlows(): void {
	activeFlows.update((flows) => {
		const activeOnly = new Map<string, OAuthFlow>();
		flows.forEach((flow, id) => {
			if (!['Complete', 'Failed', 'Cancelled'].includes(flow.state)) {
				activeOnly.set(id, flow);
			} else {
				// Stop polling for removed flows
				stopFlowPolling(id);
			}
		});
		return activeOnly;
	});

	// Clear selection if selected flow was removed
	const selectedId = get(selectedFlowId);
	if (selectedId && !get(activeFlows).has(selectedId)) {
		selectedFlowId.set(null);
	}
}

/**
 * Select a flow for detailed inspection
 */
export function selectFlow(flowId: string | null): void {
	selectedFlowId.set(flowId);
}

// Derived store for currently selected flow
export const selectedFlow = derived(
	[activeFlows, selectedFlowId],
	([$flows, $selectedId]) => {
		if (!$selectedId) return null;
		return $flows.get($selectedId) || null;
	}
);

// Derived store for active flow count
export const activeFlowCount = derived(activeFlows, ($flows) => {
	let count = 0;
	$flows.forEach((flow) => {
		if (!['Complete', 'Failed', 'Cancelled'].includes(flow.state)) {
			count++;
		}
	});
	return count;
});

// Export stores
export const oauthFlowStore = {
	flows: { subscribe: activeFlows.subscribe },
	selectedFlowId: { subscribe: selectedFlowId.subscribe },
	selectedFlow,
	activeFlowCount,
	startAuthorizationFlow,
	getFlowStatus,
	cancelFlow,
	discoverMetadata,
	clearCompletedFlows,
	selectFlow
};
