/**
 * OAuth Configuration Store
 *
 * Manages OAuth configurations for MCP servers with templates
 * and provider-specific presets.
 */

import { invoke } from '@tauri-apps/api/core';
import { writable, derived } from 'svelte/store';

export interface OAuthProviderTemplate {
	id: string;
	name: string;
	description: string;
	authorization_endpoint: string;
	token_endpoint: string;
	revocation_endpoint?: string | null;
	default_scopes: string;
	supports_pkce: boolean;
	supports_dpop: boolean;
	docs_url: string;
	registration_url: string;
}

export interface ServerOAuthConfig {
	server_id: number;
	server_name: string;
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
	has_valid_token: boolean;
	token_expires_at?: string;
	metadata?: Record<string, unknown>;
}

// OAuth provider templates loaded from backend
const providerTemplates = writable<OAuthProviderTemplate[]>([]);

/**
 * Load OAuth provider templates from backend
 */
export async function loadProviderTemplates(): Promise<void> {
	try {
		const templates = await invoke<OAuthProviderTemplate[]>('get_oauth_provider_templates');
		providerTemplates.set(templates);
	} catch (error) {
		console.error('Failed to load provider templates:', error);
		// Set empty array on error
		providerTemplates.set([]);
	}
}

// Load templates on module initialization
loadProviderTemplates();

// Server OAuth configurations
const serverConfigs = writable<Map<number, ServerOAuthConfig>>(new Map());

// Currently selected server for configuration
const selectedServerId = writable<number | null>(null);

// Configuration wizard state
export interface WizardState {
	step: 'provider' | 'discovery' | 'credentials' | 'scopes' | 'review';
	serverId: number;
	serverName: string;
	selectedTemplate?: OAuthProviderTemplate;
	discoveredMetadata?: Record<string, unknown>;
	config: Partial<ServerOAuthConfig>;
}

const wizardState = writable<WizardState | null>(null);

/**
 * Load OAuth configuration for a server
 */
export async function loadServerConfig(serverId: number): Promise<ServerOAuthConfig | null> {
	try {
		// Check if server has OAuth token
		const hasToken = await invoke<boolean>('has_valid_oauth_token', { serverId });

		// If has token, load full config
		// For now, return placeholder (will be implemented with backend commands)
		// TODO: Implement backend command to get OAuth config for server

		return null;
	} catch (error) {
		console.error('Failed to load OAuth config:', error);
		return null;
	}
}

/**
 * Check if server has valid OAuth token
 */
export async function hasValidToken(serverId: number): Promise<boolean> {
	try {
		return await invoke<boolean>('has_valid_oauth_token', { serverId });
	} catch (error) {
		console.error('Failed to check token validity:', error);
		return false;
	}
}

/**
 * Revoke OAuth token for a server
 */
export async function revokeToken(serverId: number): Promise<void> {
	try {
		await invoke('revoke_oauth_token', { serverId });

		// Update local state
		serverConfigs.update((configs) => {
			const config = configs.get(serverId);
			if (config) {
				config.has_valid_token = false;
				config.token_expires_at = undefined;
				configs.set(serverId, config);
			}
			return configs;
		});
	} catch (error) {
		console.error('Failed to revoke token:', error);
		throw error;
	}
}

/**
 * Refresh OAuth token for a server
 */
export async function refreshToken(serverId: number): Promise<void> {
	try {
		await invoke('refresh_oauth_token', { serverId });

		// Reload config to get updated expiry
		const config = await loadServerConfig(serverId);
		if (config) {
			serverConfigs.update((configs) => {
				configs.set(serverId, config);
				return configs;
			});
		}
	} catch (error) {
		console.error('Failed to refresh token:', error);
		throw error;
	}
}

/**
 * Start OAuth configuration wizard for a server
 */
export function startConfigWizard(serverId: number, serverName: string): void {
	wizardState.set({
		step: 'provider',
		serverId,
		serverName,
		config: {
			server_id: serverId,
			server_name: serverName,
			protocol_version: '2025-06-18',
			redirect_uri: 'http://localhost:8080/callback',
			scopes: [],
			resource_uri: '',
			use_pkce: true,
			use_dpop: false,
			has_valid_token: false
		}
	});
}

/**
 * Select OAuth provider template in wizard
 */
export function selectProviderTemplate(template: OAuthProviderTemplate): void {
	wizardState.update((state) => {
		if (!state) return null;

		// Parse scopes from space-separated string
		const scopes = template.default_scopes ? template.default_scopes.split(' ').filter(s => s.trim()) : [];

		return {
			...state,
			step: template.id === 'generic' ? 'credentials' : 'discovery',
			selectedTemplate: template,
			config: {
				...state.config,
				auth_server_url: template.authorization_endpoint,
				token_endpoint: template.token_endpoint,
				scopes,
				use_pkce: template.supports_pkce,
				use_dpop: template.supports_dpop
			}
		};
	});
}

/**
 * Enable manual configuration mode (skip discovery)
 */
export function enableManualConfiguration(): void {
	wizardState.update((state) => {
		if (!state) return null;
		return {
			...state,
			step: 'credentials'
		};
	});
}

/**
 * Validate manual OAuth configuration
 */
export async function validateManualConfiguration(
	authorizationEndpoint: string,
	tokenEndpoint: string
): Promise<{ valid: boolean; error?: string }> {
	try {
		await invoke('validate_oauth_manual_config', {
			authorizationEndpoint,
			tokenEndpoint
		});
		return { valid: true };
	} catch (error) {
		return {
			valid: false,
			error: error instanceof Error ? error.message : 'Validation failed'
		};
	}
}

/**
 * Update wizard step
 */
export function setWizardStep(step: WizardState['step']): void {
	wizardState.update((state) => {
		if (!state) return null;
		return { ...state, step };
	});
}

/**
 * Update wizard configuration
 */
export function updateWizardConfig(updates: Partial<ServerOAuthConfig>): void {
	wizardState.update((state) => {
		if (!state) return null;
		return {
			...state,
			config: { ...state.config, ...updates }
		};
	});
}

/**
 * Cancel configuration wizard
 */
export function cancelWizard(): void {
	wizardState.set(null);
}

/**
 * Complete configuration wizard and save
 */
export async function completeWizard(): Promise<void> {
	const state = wizardState;
	// TODO: Save configuration to backend
	wizardState.set(null);
}

/**
 * Export OAuth configuration in Claude Desktop format
 */
export async function exportConfig(serverId: number): Promise<string> {
	try {
		return await invoke<string>('export_oauth_config', { serverId });
	} catch (error) {
		console.error('Failed to export OAuth config:', error);
		throw error;
	}
}

/**
 * Import OAuth configuration from Claude Desktop format
 */
export async function importConfig(configJson: string): Promise<number> {
	try {
		const serverId = await invoke<number>('import_oauth_config', { configJson });

		// Reload configs
		const config = await loadServerConfig(serverId);
		if (config) {
			serverConfigs.update((configs) => {
				configs.set(serverId, config);
				return configs;
			});
		}

		return serverId;
	} catch (error) {
		console.error('Failed to import OAuth config:', error);
		throw error;
	}
}

// Derived store for currently selected server config
export const selectedServerConfig = derived(
	[serverConfigs, selectedServerId],
	([$configs, $selectedId]) => {
		if (!$selectedId) return null;
		return $configs.get($selectedId) || null;
	}
);

// Derived store for servers with OAuth tokens
export const serversWithTokens = derived(serverConfigs, ($configs) => {
	const withTokens: ServerOAuthConfig[] = [];
	$configs.forEach((config) => {
		if (config.has_valid_token) {
			withTokens.push(config);
		}
	});
	return withTokens;
});

// Export store
export const oauthConfigStore = {
	configs: { subscribe: serverConfigs.subscribe },
	selectedServerId: { subscribe: selectedServerId.subscribe },
	selectedServerConfig,
	serversWithTokens,
	wizardState: { subscribe: wizardState.subscribe },
	providerTemplates: { subscribe: providerTemplates.subscribe },
	loadServerConfig,
	loadProviderTemplates,
	hasValidToken,
	revokeToken,
	refreshToken,
	startConfigWizard,
	selectProviderTemplate,
	enableManualConfiguration,
	validateManualConfiguration,
	setWizardStep,
	updateWizardConfig,
	cancelWizard,
	completeWizard,
	exportConfig,
	importConfig
};
