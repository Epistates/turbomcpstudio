<script lang="ts">
	import { tokenStore } from '$lib/stores/tokenStore';
	import {
		CheckCircle,
		XCircle,
		AlertTriangle,
		Clock,
		Shield,
		Key,
		Copy,
		Eye,
		EyeOff
	} from 'lucide-svelte';

	interface Props {
		serverId: number;
		serverName: string;
	}

	let { serverId, serverName }: Props = $props();

	// Destructure the token stores
	const {
		inspectedToken: inspectedTokenStore,
		validationResults: validationStore,
		expirationStatus: expirationStatusStore,
		mcpComplianceStatus: mcpComplianceStatusStore
	} = tokenStore;

	// Subscribe to token store
	let inspectedToken = $derived($inspectedTokenStore);
	let validation = $derived($validationStore);
	let expirationStatus = $derived($expirationStatusStore);
	let mcpComplianceStatus = $derived($mcpComplianceStatusStore);

	// UI state
	let showRawToken = $state(false);
	let copiedField: string | null = $state(null);
	let activeTab: 'header' | 'claims' | 'signature' | 'raw' = $state('claims');

	// Load token on mount
	$effect(() => {
		loadToken();
	});

	async function loadToken() {
		try {
			await tokenStore.inspectServerToken(serverId, serverName);
		} catch (error) {
			console.error('Failed to inspect token:', error);
		}
	}

	async function copyToClipboard(text: string, field: string) {
		try {
			await navigator.clipboard.writeText(text);
			copiedField = field;
			setTimeout(() => (copiedField = null), 2000);
		} catch (error) {
			console.error('Failed to copy:', error);
		}
	}

	function formatJson(obj: unknown): string {
		return JSON.stringify(obj, null, 2);
	}

	function getCriticality(key: string): 'high' | 'medium' | 'low' {
		const highPriority = ['sub', 'iss', 'exp', 'aud'];
		const mediumPriority = ['scope', 'client_id', 'mcp_resource'];
		if (highPriority.includes(key)) return 'high';
		if (mediumPriority.includes(key)) return 'medium';
		return 'low';
	}
</script>

<div class="token-inspector">
	{#if !inspectedToken}
		<div class="loading">
			<Clock size={48} class="spinning" />
			<p>Loading token...</p>
		</div>
	{:else}
		<!-- Status Overview -->
		<div class="status-overview">
			<!-- Expiration Status -->
			{#if expirationStatus}
				<div class="status-card" data-status={expirationStatus.status}>
					<div class="status-icon {expirationStatus.color}">
						<Clock size={24} />
					</div>
					<div class="status-content">
						<span class="status-label">Expiration</span>
						<span class="status-value">{expirationStatus.text}</span>
					</div>
				</div>
			{/if}

			<!-- MCP Compliance -->
			{#if mcpComplianceStatus}
				<div class="status-card" data-compliant={mcpComplianceStatus.compliant}>
					<div class="status-icon {mcpComplianceStatus.color}">
						<Shield size={24} />
					</div>
					<div class="status-content">
						<span class="status-label">MCP Compliance</span>
						<span class="status-value">{mcpComplianceStatus.text}</span>
					</div>
				</div>
			{/if}

			<!-- Validation Status -->
			{#if validation}
				<div class="status-card" data-valid={validation.is_valid}>
					<div class="status-icon {validation.is_valid ? 'green' : 'red'}">
						{#if validation.is_valid}
							<CheckCircle size={24} />
						{:else}
							<XCircle size={24} />
						{/if}
					</div>
					<div class="status-content">
						<span class="status-label">Validation</span>
						<span class="status-value"
							>{validation.is_valid ? 'Valid' : `${validation.errors.length} Errors`}</span
						>
					</div>
				</div>
			{/if}
		</div>

		<!-- Warnings & Errors -->
		{#if validation && (validation.warnings.length > 0 || validation.errors.length > 0)}
			<div class="alerts">
				{#each validation.errors as error}
					<div class="alert error">
						<XCircle size={16} />
						<span>{error}</span>
					</div>
				{/each}

				{#each validation.warnings as warning}
					<div class="alert warning">
						<AlertTriangle size={16} />
						<span>{warning}</span>
					</div>
				{/each}
			</div>
		{/if}

		<!-- Token Metadata -->
		<div class="token-metadata">
			<div class="metadata-row">
				<span class="label">Token Type:</span>
				<span class="value">{inspectedToken.token_type}</span>
			</div>

			{#if inspectedToken.scopes && inspectedToken.scopes.length > 0}
				<div class="metadata-row">
					<span class="label">Scopes:</span>
					<div class="scopes">
						{#each inspectedToken.scopes as scope}
							<span class="scope-badge">{scope}</span>
						{/each}
					</div>
				</div>
			{/if}

			{#if inspectedToken.decoded}
				{#if inspectedToken.decoded.claims.iss}
					<div class="metadata-row">
						<span class="label">Issuer:</span>
						<span class="value mono">{inspectedToken.decoded.claims.iss}</span>
					</div>
				{/if}

				{#if inspectedToken.decoded.claims.sub}
					<div class="metadata-row">
						<span class="label">Subject:</span>
						<span class="value mono">{inspectedToken.decoded.claims.sub}</span>
					</div>
				{/if}
			{/if}
		</div>

		<!-- JWT Viewer Tabs -->
		{#if inspectedToken.decoded}
			<div class="jwt-viewer">
				<div class="tabs">
					<button
						class="tab"
						class:active={activeTab === 'header'}
						onclick={() => (activeTab = 'header')}
					>
						Header
					</button>
					<button
						class="tab"
						class:active={activeTab === 'claims'}
						onclick={() => (activeTab = 'claims')}
					>
						Claims
					</button>
					<button
						class="tab"
						class:active={activeTab === 'signature'}
						onclick={() => (activeTab = 'signature')}
					>
						Signature
					</button>
					<button
						class="tab"
						class:active={activeTab === 'raw'}
						onclick={() => (activeTab = 'raw')}
					>
						Raw JWT
					</button>
				</div>

				<div class="tab-content">
					{#if activeTab === 'header'}
						<div class="jwt-section">
							<h4>JWT Header</h4>
							<div class="claims-list">
								{#each Object.entries(inspectedToken.decoded.header) as [key, value]}
									<div class="claim-row">
										<span class="claim-key">{key}</span>
										<span class="claim-value mono">{String(value)}</span>
										<button
											class="btn-copy"
											onclick={() => copyToClipboard(String(value), `header.${key}`)}
										>
											{#if copiedField === `header.${key}`}
												<CheckCircle size={14} />
											{:else}
												<Copy size={14} />
											{/if}
										</button>
									</div>
								{/each}
							</div>
						</div>
					{:else if activeTab === 'claims'}
						<div class="jwt-section">
							<h4>JWT Claims</h4>
							<div class="claims-list">
								{#each Object.entries(inspectedToken.decoded.claims) as [key, value]}
									{@const criticality = getCriticality(key)}
									<div class="claim-row" data-criticality={criticality}>
										<span class="claim-key">
											{key}
											{#if criticality === 'high'}
												<span class="badge critical">Required</span>
											{:else if key.startsWith('mcp_')}
												<span class="badge mcp">MCP</span>
											{/if}
										</span>

										<span class="claim-value mono">
											{#if key === 'exp' || key === 'nbf' || key === 'iat'}
												{tokenStore.formatTimestamp(value as number)}
												<span class="timestamp-relative">
													({tokenStore.timeUntilExpiration(value as number)})
												</span>
											{:else if typeof value === 'object'}
												<pre>{formatJson(value)}</pre>
											{:else}
												{String(value)}
											{/if}
										</span>

										<button
											class="btn-copy"
											onclick={() => copyToClipboard(String(value), `claims.${key}`)}
										>
											{#if copiedField === `claims.${key}`}
												<CheckCircle size={14} />
											{:else}
												<Copy size={14} />
											{/if}
										</button>
									</div>
								{/each}
							</div>
						</div>
					{:else if activeTab === 'signature'}
						<div class="jwt-section">
							<h4>JWT Signature</h4>
							<div class="signature-info">
								<p class="info-text">
									The signature is used to verify that the token was issued by a trusted authorization
									server and has not been tampered with.
								</p>

								<div class="signature-data">
									<div class="signature-header">
										<span>Algorithm: <strong>{inspectedToken.decoded?.header.alg ?? 'Unknown'}</strong></span>
										<button
											class="btn-copy"
											onclick={() => copyToClipboard(inspectedToken.decoded?.signature ?? '', 'signature')}
										>
											{#if copiedField === 'signature'}
												<CheckCircle size={14} />
											{:else}
												<Copy size={14} />
											{/if}
										</button>
									</div>
									<pre class="signature-value">{inspectedToken.decoded?.signature ?? ''}</pre>
								</div>

								{#if inspectedToken.decoded?.header.kid}
									<div class="key-id">
										<Key size={16} />
										<span>Key ID: <code>{inspectedToken.decoded.header.kid}</code></span>
									</div>
								{/if}
							</div>
						</div>
					{:else if activeTab === 'raw'}
						<div class="jwt-section">
							<div class="raw-token-header">
								<h4>Raw JWT Token</h4>
								<button class="btn-toggle" onclick={() => (showRawToken = !showRawToken)}>
									{#if showRawToken}
										<EyeOff size={16} />
										Hide
									{:else}
										<Eye size={16} />
										Show
									{/if}
								</button>
							</div>

							{#if showRawToken}
								<div class="raw-token-content">
									<button
										class="btn-copy-full"
										onclick={() => copyToClipboard(inspectedToken.access_token, 'raw-token')}
									>
										{#if copiedField === 'raw-token'}
											<CheckCircle size={16} />
											Copied!
										{:else}
											<Copy size={16} />
											Copy Token
										{/if}
									</button>

									<div class="jwt-parts">
										<div class="jwt-part">
											<span class="part-label">Header (Base64URL)</span>
											<pre class="part-value header">{inspectedToken.decoded.raw.header}</pre>
										</div>

										<div class="jwt-part">
											<span class="part-label">Payload (Base64URL)</span>
											<pre class="part-value payload">{inspectedToken.decoded.raw.payload}</pre>
										</div>

										<div class="jwt-part">
											<span class="part-label">Signature (Base64URL)</span>
											<pre class="part-value signature">{inspectedToken.decoded.raw.signature}</pre>
										</div>
									</div>
								</div>
							{:else}
								<p class="hidden-message">Click "Show" to reveal the raw JWT token</p>
							{/if}
						</div>
					{/if}
				</div>
			</div>
		{/if}
	{/if}
</div>

<style>
	.token-inspector {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		padding: 1rem;
		background: var(--bg-secondary);
		border-radius: 8px;
	}

	.loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
		padding: 4rem;
		color: var(--text-tertiary);
	}

	.spinning {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	/* Status Overview */
	.status-overview {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
	}

	.status-card {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 1rem;
		background: var(--bg-tertiary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
	}

	.status-card[data-status='expired'],
	.status-card[data-valid='false'] {
		border-color: var(--border-error);
		background: var(--bg-error-light);
	}

	.status-card[data-status='expiring'] {
		border-color: var(--border-warning);
		background: var(--bg-warning-light);
	}

	.status-card[data-status='valid'],
	.status-card[data-valid='true'],
	.status-card[data-compliant='true'] {
		border-color: var(--border-success);
		background: var(--bg-success-light);
	}

	.status-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 48px;
		height: 48px;
		border-radius: 50%;
		background: var(--bg-primary);
	}

	.status-icon.green {
		color: var(--text-success);
	}
	.status-icon.yellow {
		color: var(--text-warning);
	}
	.status-icon.red {
		color: var(--text-error);
	}

	.status-content {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		flex: 1;
	}

	.status-label {
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.status-value {
		font-weight: 600;
		color: var(--text-primary);
	}

	/* Alerts */
	.alerts {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.alert {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1rem;
		border-radius: 6px;
		font-size: 0.875rem;
	}

	.alert.error {
		background: var(--bg-error-light);
		border: 1px solid var(--border-error);
		color: var(--text-error);
	}

	.alert.warning {
		background: var(--bg-warning-light);
		border: 1px solid var(--border-warning);
		color: var(--text-warning);
	}

	/* Token Metadata */
	.token-metadata {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1rem;
		background: var(--bg-tertiary);
		border-radius: 6px;
	}

	.metadata-row {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.metadata-row .label {
		font-weight: 500;
		color: var(--text-secondary);
		min-width: 100px;
	}

	.metadata-row .value {
		flex: 1;
		color: var(--text-primary);
	}

	.metadata-row .value.mono {
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
	}

	.scopes {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.scope-badge {
		padding: 0.25rem 0.75rem;
		background: var(--bg-accent-light);
		color: var(--accent-color);
		border-radius: 12px;
		font-size: 0.875rem;
		font-weight: 500;
	}

	/* JWT Viewer */
	.jwt-viewer {
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: hidden;
	}

	.tabs {
		display: flex;
		background: var(--bg-tertiary);
		border-bottom: 1px solid var(--border-color);
	}

	.tab {
		flex: 1;
		padding: 0.75rem 1rem;
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--text-secondary);
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.tab:hover {
		background: var(--bg-hover);
	}

	.tab.active {
		color: var(--accent-color);
		border-bottom-color: var(--accent-color);
	}

	.tab-content {
		padding: 1.5rem;
		background: var(--bg-primary);
	}

	.jwt-section h4 {
		margin: 0 0 1rem 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.claims-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.claim-row {
		display: grid;
		grid-template-columns: 200px 1fr auto;
		align-items: start;
		gap: 1rem;
		padding: 0.75rem;
		background: var(--bg-secondary);
		border-radius: 6px;
		border: 1px solid var(--border-color);
	}

	.claim-row[data-criticality='high'] {
		border-color: var(--border-info);
		background: var(--bg-info-light);
	}

	.claim-key {
		font-weight: 600;
		color: var(--text-primary);
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.claim-value {
		color: var(--text-secondary);
		word-break: break-all;
	}

	.claim-value.mono {
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
	}

	.claim-value pre {
		margin: 0;
		font-size: 0.8125rem;
		line-height: 1.5;
	}

	.timestamp-relative {
		color: var(--text-tertiary);
		font-size: 0.875rem;
		margin-left: 0.5rem;
	}

	.badge {
		padding: 0.125rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.badge.critical {
		background: var(--bg-error);
		color: white;
	}

	.badge.mcp {
		background: var(--accent-color);
		color: white;
	}

	.btn-copy {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		padding: 0;
		background: var(--bg-tertiary);
		border: 1px solid var(--border-color);
		border-radius: 4px;
		color: var(--text-secondary);
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-copy:hover {
		background: var(--bg-hover);
		border-color: var(--accent-color);
		color: var(--accent-color);
	}

	/* Signature Section */
	.signature-info {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.info-text {
		margin: 0;
		padding: 1rem;
		background: var(--bg-info-light);
		border-left: 3px solid var(--text-info);
		border-radius: 4px;
		color: var(--text-secondary);
		font-size: 0.875rem;
	}

	.signature-data {
		padding: 1rem;
		background: var(--bg-secondary);
		border-radius: 6px;
	}

	.signature-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.signature-value {
		margin: 0;
		padding: 0.75rem;
		background: var(--bg-code);
		border-radius: 4px;
		font-family: 'Courier New', monospace;
		font-size: 0.8125rem;
		word-break: break-all;
		line-height: 1.5;
	}

	.key-id {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background: var(--bg-tertiary);
		border-radius: 6px;
		font-size: 0.875rem;
	}

	.key-id code {
		font-family: 'Courier New', monospace;
		background: var(--bg-code);
		padding: 0.125rem 0.5rem;
		border-radius: 4px;
	}

	/* Raw Token Section */
	.raw-token-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.btn-toggle {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: var(--bg-tertiary);
		border: 1px solid var(--border-color);
		border-radius: 6px;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 0.875rem;
		transition: all 0.2s;
	}

	.btn-toggle:hover {
		background: var(--bg-hover);
		border-color: var(--accent-color);
		color: var(--accent-color);
	}

	.hidden-message {
		padding: 2rem;
		text-align: center;
		color: var(--text-tertiary);
		font-style: italic;
	}

	.raw-token-content {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.btn-copy-full {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.75rem 1.5rem;
		background: var(--accent-color);
		color: white;
		border: none;
		border-radius: 6px;
		font-weight: 500;
		cursor: pointer;
		transition: opacity 0.2s;
	}

	.btn-copy-full:hover {
		opacity: 0.9;
	}

	.jwt-parts {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.jwt-part {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.part-label {
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--text-secondary);
	}

	.part-value {
		margin: 0;
		padding: 1rem;
		background: var(--bg-code);
		border-radius: 6px;
		font-family: 'Courier New', monospace;
		font-size: 0.8125rem;
		word-break: break-all;
		line-height: 1.5;
		border: 1px solid var(--border-color);
	}

	.part-value.header {
		border-left: 3px solid #e74c3c;
	}
	.part-value.payload {
		border-left: 3px solid #3498db;
	}
	.part-value.signature {
		border-left: 3px solid #2ecc71;
	}
</style>
