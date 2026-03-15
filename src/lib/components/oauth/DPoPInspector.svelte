<script lang="ts">
	import { CheckCircle, Shield, Key, AlertCircle, Info, Copy } from 'lucide-svelte';

	interface Props {
		token?: {
			dpop_jkt?: string;
			use_dpop: boolean;
		};
		flow?: {
			dpop_proof?: {
				header: {
					typ: string;
					alg: string;
					jwk: Record<string, string>;
				};
				claims: {
					jti: string;
					htm: string;
					htu: string;
					iat: number;
					ath?: string;
				};
			};
		};
	}

	let { token, flow }: Props = $props();

	let copiedField = $state<string | null>(null);

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
</script>

<div class="dpop-inspector">
	<div class="header">
		<div class="title-section">
			<Shield size={24} class="header-icon" />
			<div>
				<h3>DPoP Inspector</h3>
				<p>Demonstration of Proof-of-Possession (RFC 9449)</p>
			</div>
		</div>

		{#if token?.use_dpop}
			<div class="status-badge enabled">
				<CheckCircle size={16} />
				DPoP Enabled
			</div>
		{:else}
			<div class="status-badge disabled">
				<AlertCircle size={16} />
				DPoP Disabled
			</div>
		{/if}
	</div>

	<div class="info-section">
		<Info size={16} />
		<p>
			<strong>What is DPoP?</strong> DPoP binds OAuth tokens to the client that requested them,
			preventing token theft and replay attacks. This is achieved by using asymmetric cryptography
			to prove possession of a private key.
		</p>
	</div>

	{#if !token?.use_dpop}
		<div class="disabled-message">
			<AlertCircle size={48} />
			<h4>DPoP is not enabled for this server</h4>
			<p>
				To enable DPoP, edit your OAuth configuration and check "Enable DPoP" in the security
				options. The server must support DPoP (RFC 9449) for this feature to work.
			</p>
		</div>
	{:else}
		<!-- DPoP Token Binding -->
		<div class="section-card">
			<h4>
				<Key size={18} />
				Token Binding
			</h4>

			{#if token.dpop_jkt}
				<div class="binding-info">
					<div class="info-row">
						<span class="label">JWK Thumbprint (jkt):</span>
						<div class="value-with-copy">
							<code class="thumbprint">{token.dpop_jkt}</code>
							<button
								class="btn-copy"
								onclick={() => copyToClipboard(token.dpop_jkt || '', 'jkt')}
							>
								{#if copiedField === 'jkt'}
									<CheckCircle size={14} />
								{:else}
									<Copy size={14} />
								{/if}
							</button>
						</div>
					</div>

					<p class="info-text">
						This cryptographic thumbprint binds the access token to the client's public key,
						ensuring only the holder of the private key can use the token.
					</p>
				</div>
			{:else}
				<div class="no-data">
					<AlertCircle size={24} />
					<p>No DPoP binding information available</p>
				</div>
			{/if}
		</div>

		<!-- DPoP Proof Details -->
		{#if flow?.dpop_proof}
			<div class="section-card">
				<h4>
					<Shield size={18} />
					DPoP Proof JWT
				</h4>

				<div class="proof-details">
					<!-- Header -->
					<div class="proof-section">
						<h5>Header</h5>
						<div class="claims-list">
							<div class="claim-row">
								<span class="claim-key">typ</span>
								<span class="claim-value">{flow.dpop_proof.header.typ}</span>
								<span class="claim-description">Type (must be "dpop+jwt")</span>
							</div>
							<div class="claim-row">
								<span class="claim-key">alg</span>
								<span class="claim-value">{flow.dpop_proof.header.alg}</span>
								<span class="claim-description">Signing algorithm</span>
							</div>
							<div class="claim-row">
								<span class="claim-key">jwk</span>
								<button
									class="btn-expand"
									onclick={() => copyToClipboard(formatJson(flow.dpop_proof?.header.jwk ?? {}), 'jwk')}
								>
									{#if copiedField === 'jwk'}
										<CheckCircle size={14} />
									{:else}
										<Copy size={14} />
									{/if}
									Copy Public Key
								</button>
								<span class="claim-description">Client public key (JWK format)</span>
							</div>
						</div>
					</div>

					<!-- Claims -->
					<div class="proof-section">
						<h5>Claims</h5>
						<div class="claims-list">
							<div class="claim-row">
								<span class="claim-key">jti</span>
								<code class="claim-value">{flow.dpop_proof.claims.jti}</code>
								<span class="claim-description">Unique proof identifier</span>
							</div>
							<div class="claim-row">
								<span class="claim-key">htm</span>
								<code class="claim-value">{flow.dpop_proof.claims.htm}</code>
								<span class="claim-description">HTTP method (GET, POST, etc.)</span>
							</div>
							<div class="claim-row">
								<span class="claim-key">htu</span>
								<code class="claim-value url">{flow.dpop_proof.claims.htu}</code>
								<span class="claim-description">HTTP URI (target endpoint)</span>
							</div>
							<div class="claim-row">
								<span class="claim-key">iat</span>
								<span class="claim-value">
									{new Date(flow.dpop_proof.claims.iat * 1000).toLocaleString()}
								</span>
								<span class="claim-description">Issued at timestamp</span>
							</div>
							{#if flow.dpop_proof.claims.ath}
								<div class="claim-row highlighted">
									<span class="claim-key">ath</span>
									<code class="claim-value">{flow.dpop_proof.claims.ath}</code>
									<span class="claim-description">Access token hash (SHA-256)</span>
								</div>
							{/if}
						</div>
					</div>
				</div>
			</div>

			<!-- Security Properties -->
			<div class="section-card security">
				<h4>
					<Shield size={18} />
					Security Properties
				</h4>

				<div class="properties-grid">
					<div class="property-card">
						<CheckCircle size={20} class="property-icon success" />
						<div class="property-content">
							<strong>Token Binding</strong>
							<p>Token cryptographically bound to client key</p>
						</div>
					</div>

					<div class="property-card">
						<CheckCircle size={20} class="property-icon success" />
						<div class="property-content">
							<strong>Replay Protection</strong>
							<p>Unique jti prevents proof reuse</p>
						</div>
					</div>

					<div class="property-card">
						<CheckCircle size={20} class="property-icon success" />
						<div class="property-content">
							<strong>Request Binding</strong>
							<p>Proof bound to specific HTTP method and URI</p>
						</div>
					</div>

					{#if flow.dpop_proof.claims.ath}
						<div class="property-card">
							<CheckCircle size={20} class="property-icon success" />
							<div class="property-content">
								<strong>Token Hash</strong>
								<p>Access token integrity verified via hash</p>
							</div>
						</div>
					{/if}
				</div>
			</div>
		{:else}
			<div class="section-card">
				<h4>
					<Shield size={18} />
					DPoP Proof JWT
				</h4>
				<div class="no-data">
					<Info size={24} />
					<p>No DPoP proof available yet. Start an OAuth flow to see DPoP proofs.</p>
				</div>
			</div>
		{/if}

		<!-- Competitive Advantage Callout -->
		<div class="competitive-advantage">
			<div class="advantage-icon">✨</div>
			<div class="advantage-content">
				<strong>TurboMCP Studio Exclusive Feature</strong>
				<p>
					DPoP support is unique to TurboMCP Studio. This advanced security feature provides
					enterprise-grade token protection that goes beyond standard OAuth 2.0. Competitors like
					MCPJam do not support DPoP inspection or proof generation.
				</p>
			</div>
		</div>
	{/if}
</div>

<style>
	.dpop-inspector {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		padding: 1rem;
		background: var(--bg-secondary);
		border-radius: 8px;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: start;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--border-color);
	}

	.title-section {
		display: flex;
		gap: 1rem;
	}

	:global(.header-icon) {
		color: var(--accent-color);
	}

	.title-section h3 {
		margin: 0 0 0.25rem 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.title-section p {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.status-badge {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		border-radius: 6px;
		font-weight: 500;
		font-size: 0.875rem;
	}

	.status-badge.enabled {
		background: var(--bg-success-light);
		color: var(--text-success);
		border: 1px solid var(--border-success);
	}

	.status-badge.disabled {
		background: var(--bg-warning-light);
		color: var(--text-warning);
		border: 1px solid var(--border-warning);
	}

	.info-section {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		background: var(--bg-info-light);
		border-left: 3px solid var(--text-info);
		border-radius: 6px;
	}

	.info-section p {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-secondary);
		line-height: 1.6;
	}

	.disabled-message {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
		padding: 3rem 2rem;
		text-align: center;
		background: var(--bg-tertiary);
		border-radius: 8px;
		color: var(--text-tertiary);
	}

	.disabled-message h4 {
		margin: 0;
		color: var(--text-primary);
	}

	.disabled-message p {
		margin: 0;
		max-width: 500px;
	}

	.section-card {
		padding: 1.5rem;
		background: var(--bg-primary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
	}

	.section-card.security {
		background: linear-gradient(135deg, var(--bg-success-light) 0%, var(--bg-info-light) 100%);
		border-color: var(--border-success);
	}

	.section-card h4 {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin: 0 0 1rem 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.section-card h5 {
		margin: 0 0 0.75rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.binding-info {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.info-row {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.info-row .label {
		font-weight: 500;
		color: var(--text-secondary);
		font-size: 0.875rem;
	}

	.value-with-copy {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.thumbprint {
		flex: 1;
		padding: 0.75rem;
		background: var(--bg-code);
		border: 1px solid var(--border-color);
		border-radius: 6px;
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		word-break: break-all;
	}

	.info-text {
		margin: 0;
		padding: 0.75rem;
		background: var(--bg-tertiary);
		border-left: 2px solid var(--accent-color);
		border-radius: 4px;
		font-size: 0.8125rem;
		color: var(--text-secondary);
		font-style: italic;
	}

	.no-data {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		padding: 2rem;
		color: var(--text-tertiary);
	}

	.proof-details {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.proof-section {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.claims-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.claim-row {
		display: grid;
		grid-template-columns: 80px 1fr 2fr;
		align-items: center;
		gap: 1rem;
		padding: 0.75rem;
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 6px;
	}

	.claim-row.highlighted {
		border-color: var(--accent-color);
		background: var(--bg-accent-light);
	}

	.claim-key {
		font-weight: 600;
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		color: var(--accent-color);
	}

	.claim-value {
		font-family: 'Courier New', monospace;
		font-size: 0.8125rem;
		color: var(--text-primary);
	}

	.claim-value.url {
		word-break: break-all;
	}

	.claim-description {
		font-size: 0.8125rem;
		color: var(--text-tertiary);
	}

	.btn-copy,
	.btn-expand {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		background: var(--bg-tertiary);
		border: 1px solid var(--border-color);
		border-radius: 4px;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 0.8125rem;
		transition: all 0.2s;
	}

	.btn-copy:hover,
	.btn-expand:hover {
		background: var(--bg-hover);
		border-color: var(--accent-color);
		color: var(--accent-color);
	}

	.properties-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 1rem;
	}

	.property-card {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		background: white;
		border: 1px solid var(--border-success);
		border-radius: 6px;
	}

	:global(.property-icon) {
		flex-shrink: 0;
	}

	:global(.property-icon.success) {
		color: var(--text-success);
	}

	.property-content {
		flex: 1;
	}

	.property-content strong {
		display: block;
		margin-bottom: 0.25rem;
		color: var(--text-primary);
		font-size: 0.875rem;
	}

	.property-content p {
		margin: 0;
		font-size: 0.8125rem;
		color: var(--text-secondary);
		line-height: 1.5;
	}

	.competitive-advantage {
		display: flex;
		gap: 1.5rem;
		padding: 1.5rem;
		background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
		border-radius: 8px;
		color: white;
	}

	.advantage-icon {
		font-size: 2rem;
		flex-shrink: 0;
	}

	.advantage-content strong {
		display: block;
		margin-bottom: 0.5rem;
		font-size: 1rem;
	}

	.advantage-content p {
		margin: 0;
		font-size: 0.875rem;
		line-height: 1.6;
		opacity: 0.95;
	}
</style>
