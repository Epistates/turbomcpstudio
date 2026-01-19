/**
 * Token Store
 *
 * Manages OAuth token inspection, JWT decoding, and MCP validation.
 */

import { invoke } from '@tauri-apps/api/core';
import { writable, derived } from 'svelte/store';

export interface JWTHeader {
	alg: string;
	typ: string;
	kid?: string;
	jku?: string;
}

export interface JWTClaims {
	iss?: string; // Issuer
	sub?: string; // Subject
	aud?: string | string[]; // Audience
	exp?: number; // Expiration
	nbf?: number; // Not Before
	iat?: number; // Issued At
	jti?: string; // JWT ID
	scope?: string; // Scopes
	client_id?: string;
	// MCP-specific claims
	mcp_resource?: string;
	mcp_permissions?: string[];
	[key: string]: unknown;
}

export interface DecodedJWT {
	header: JWTHeader;
	claims: JWTClaims;
	signature: string;
	raw: {
		header: string;
		payload: string;
		signature: string;
	};
}

export interface TokenValidation {
	is_valid: boolean;
	is_expired: boolean;
	expires_in_seconds?: number;
	warnings: string[];
	errors: string[];
	mcp_compliance: {
		has_resource_claim: boolean;
		has_valid_scopes: boolean;
		has_required_claims: boolean;
	};
}

export interface ServerToken {
	server_id: number;
	server_name: string;
	access_token: string;
	token_type: string;
	decoded?: DecodedJWT;
	validation?: TokenValidation;
	expires_at?: string;
	scopes?: string[];
}

// Currently inspected token
const inspectedToken = writable<ServerToken | null>(null);

// Token validation results
const validationResults = writable<TokenValidation | null>(null);

/**
 * Decode JWT token without validation (for inspection)
 */
export async function decodeToken(token: string): Promise<DecodedJWT> {
	try {
		const decoded = await invoke<{ header: JWTHeader; claims: JWTClaims }>('decode_jwt_token', {
			token
		});

		// Parse raw parts
		const parts = token.split('.');
		if (parts.length !== 3) {
			throw new Error('Invalid JWT format');
		}

		return {
			header: decoded.header,
			claims: decoded.claims,
			signature: parts[2],
			raw: {
				header: parts[0],
				payload: parts[1],
				signature: parts[2]
			}
		};
	} catch (error) {
		console.error('Failed to decode JWT:', error);
		throw error;
	}
}

/**
 * Get OAuth token for a server
 */
export async function getServerToken(serverId: number): Promise<[string, string]> {
	try {
		return await invoke<[string, string]>('get_oauth_token', { serverId });
	} catch (error) {
		console.error('Failed to get server token:', error);
		throw error;
	}
}

/**
 * Inspect token for a server
 */
export async function inspectServerToken(serverId: number, serverName: string): Promise<void> {
	try {
		const [accessToken, tokenType] = await getServerToken(serverId);
		const decoded = await decodeToken(accessToken);
		const validation = validateToken(decoded);

		const token: ServerToken = {
			server_id: serverId,
			server_name: serverName,
			access_token: accessToken,
			token_type: tokenType,
			decoded,
			validation,
			expires_at: decoded.claims.exp
				? new Date(decoded.claims.exp * 1000).toISOString()
				: undefined,
			scopes: decoded.claims.scope?.split(' ')
		};

		inspectedToken.set(token);
		validationResults.set(validation);
	} catch (error) {
		console.error('Failed to inspect token:', error);
		throw error;
	}
}

/**
 * Validate decoded JWT for MCP compliance
 */
function validateToken(decoded: DecodedJWT): TokenValidation {
	const now = Math.floor(Date.now() / 1000);
	const warnings: string[] = [];
	const errors: string[] = [];

	// Check expiration
	const isExpired = decoded.claims.exp ? decoded.claims.exp < now : false;
	const expiresInSeconds = decoded.claims.exp ? decoded.claims.exp - now : undefined;

	if (isExpired) {
		errors.push('Token has expired');
	} else if (expiresInSeconds && expiresInSeconds < 300) {
		warnings.push(`Token expires in ${expiresInSeconds} seconds`);
	}

	// Check not-before
	if (decoded.claims.nbf && decoded.claims.nbf > now) {
		errors.push('Token is not yet valid (nbf claim)');
	}

	// Check required standard claims
	if (!decoded.claims.iss) {
		warnings.push('Missing issuer (iss) claim');
	}
	if (!decoded.claims.sub) {
		warnings.push('Missing subject (sub) claim');
	}
	if (!decoded.claims.aud) {
		warnings.push('Missing audience (aud) claim');
	}

	// MCP-specific validation
	const hasResourceClaim = !!decoded.claims.mcp_resource;
	const hasValidScopes = !!decoded.claims.scope && decoded.claims.scope.length > 0;
	const hasRequiredClaims = !!(decoded.claims.iss && decoded.claims.sub && decoded.claims.exp);

	if (!hasResourceClaim) {
		warnings.push('Missing MCP resource claim (mcp_resource)');
	}

	if (!hasValidScopes) {
		warnings.push('Missing or empty scope claim');
	}

	// Check algorithm
	if (decoded.header.alg === 'none') {
		errors.push('Insecure algorithm: none');
	} else if (!['RS256', 'RS384', 'RS512', 'ES256', 'ES384', 'ES512'].includes(decoded.header.alg)) {
		warnings.push(`Non-standard algorithm: ${decoded.header.alg}`);
	}

	return {
		is_valid: errors.length === 0,
		is_expired: isExpired,
		expires_in_seconds: expiresInSeconds,
		warnings,
		errors,
		mcp_compliance: {
			has_resource_claim: hasResourceClaim,
			has_valid_scopes: hasValidScopes,
			has_required_claims: hasRequiredClaims
		}
	};
}

/**
 * Clear inspected token
 */
export function clearInspection(): void {
	inspectedToken.set(null);
	validationResults.set(null);
}

/**
 * Format timestamp for display
 */
export function formatTimestamp(timestamp: number): string {
	const date = new Date(timestamp * 1000);
	return date.toLocaleString();
}

/**
 * Calculate time until expiration
 */
export function timeUntilExpiration(exp: number): string {
	const now = Math.floor(Date.now() / 1000);
	const seconds = exp - now;

	if (seconds < 0) return 'Expired';
	if (seconds < 60) return `${seconds}s`;
	if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
	if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
	return `${Math.floor(seconds / 86400)}d`;
}

/**
 * Decode base64url (JWT format)
 */
export function decodeBase64Url(input: string): string {
	// Replace URL-safe characters
	const base64 = input.replace(/-/g, '+').replace(/_/g, '/');
	// Pad if needed
	const padded = base64.padEnd(base64.length + ((4 - (base64.length % 4)) % 4), '=');
	// Decode
	try {
		return atob(padded);
	} catch (error) {
		console.error('Failed to decode base64url:', error);
		return input;
	}
}

// Derived store for token expiration status
export const expirationStatus = derived(inspectedToken, ($token) => {
	if (!$token?.decoded?.claims.exp) return null;

	const now = Math.floor(Date.now() / 1000);
	const exp = $token.decoded.claims.exp;
	const remaining = exp - now;

	if (remaining < 0) {
		return { status: 'expired', color: 'red', text: 'Expired' };
	} else if (remaining < 300) {
		return { status: 'expiring', color: 'yellow', text: `Expires in ${timeUntilExpiration(exp)}` };
	} else {
		return { status: 'valid', color: 'green', text: `Expires in ${timeUntilExpiration(exp)}` };
	}
});

// Derived store for MCP compliance status
export const mcpComplianceStatus = derived(validationResults, ($validation) => {
	if (!$validation) return null;

	const { mcp_compliance } = $validation;
	const compliant =
		mcp_compliance.has_resource_claim &&
		mcp_compliance.has_valid_scopes &&
		mcp_compliance.has_required_claims;

	return {
		compliant,
		color: compliant ? 'green' : 'yellow',
		text: compliant ? 'MCP Compliant' : 'Partially Compliant'
	};
});

// Export store
export const tokenStore = {
	inspectedToken: { subscribe: inspectedToken.subscribe },
	validationResults: { subscribe: validationResults.subscribe },
	expirationStatus,
	mcpComplianceStatus,
	decodeToken,
	getServerToken,
	inspectServerToken,
	clearInspection,
	formatTimestamp,
	timeUntilExpiration,
	decodeBase64Url
};
