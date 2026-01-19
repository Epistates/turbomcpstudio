<script lang="ts">
	import {
		oauthConfigStore,
		type OAuthProviderTemplate,
		type WizardState
	} from '$lib/stores/oauthConfigStore';
	import { oauthFlowStore } from '$lib/stores/oauthFlowStore';
	import {
		CheckCircle,
		ChevronRight,
		ChevronLeft,
		X,
		Loader2,
		Search,
		Key,
		Shield,
		AlertCircle
	} from 'lucide-svelte';

	interface Props {
		serverId: number;
		serverName: string;
		onComplete?: () => void;
		onCancel?: () => void;
	}

	let { serverId, serverName, onComplete, onCancel }: Props = $props();

	// Destructure the wizard state store and provider templates store
	const { wizardState: wizardStateStore, providerTemplates: providerTemplatesStore } = oauthConfigStore;

	// Subscribe to wizard state and templates
	let wizardState = $derived($wizardStateStore);
	let templates = $derived($providerTemplatesStore);

	// Discovery state
	let isDiscovering = $state(false);
	let discoveryError = $state<string | null>(null);
	let discoveredMetadata = $state<any>(null);

	// Initialize wizard on mount
	$effect(() => {
		if (!wizardState) {
			oauthConfigStore.startConfigWizard(serverId, serverName);
		}
	});

	async function selectTemplate(template: OAuthProviderTemplate) {
		oauthConfigStore.selectProviderTemplate(template);

		// If generic (manual), skip discovery
		if (template.id === 'generic') {
			return; // Wizard will advance to credentials step
		}

		// For specific providers (GitHub, Google, etc.), use pre-configured endpoints
		// No discovery needed - endpoints are already in the template
		if (template.authorization_endpoint && template.token_endpoint) {
			oauthConfigStore.setWizardStep('credentials');
		}
	}

	function configureManually() {
		oauthConfigStore.enableManualConfiguration();
	}

	async function discoverMetadata(serverUrl: string) {
		isDiscovering = true;
		discoveryError = null;

		try {
			const metadata = await oauthFlowStore.discoverMetadata(serverUrl);
			discoveredMetadata = metadata;

			// Update wizard config with discovered endpoints
			if (metadata.auth_server) {
				oauthConfigStore.updateWizardConfig({
					auth_server_url: metadata.auth_server.issuer,
					token_endpoint: metadata.auth_server.token_endpoint,
					metadata: metadata as any
				});
			}

			// Auto-advance to next step
			oauthConfigStore.setWizardStep('credentials');
		} catch (error) {
			discoveryError = error instanceof Error ? error.message : 'Discovery failed';
			console.error('Metadata discovery failed:', error);
		} finally {
			isDiscovering = false;
		}
	}

	function updateConfig(field: string, value: any) {
		oauthConfigStore.updateWizardConfig({ [field]: value });
	}

	function nextStep() {
		if (!wizardState) return;

		const steps: WizardState['step'][] = [
			'provider',
			'discovery',
			'credentials',
			'scopes',
			'review'
		];
		const currentIndex = steps.indexOf(wizardState.step);
		if (currentIndex < steps.length - 1) {
			oauthConfigStore.setWizardStep(steps[currentIndex + 1]);
		}
	}

	function previousStep() {
		if (!wizardState) return;

		const steps: WizardState['step'][] = [
			'provider',
			'discovery',
			'credentials',
			'scopes',
			'review'
		];
		const currentIndex = steps.indexOf(wizardState.step);
		if (currentIndex > 0) {
			oauthConfigStore.setWizardStep(steps[currentIndex - 1]);
		}
	}

	async function complete() {
		await oauthConfigStore.completeWizard();
		onComplete?.();
	}

	function cancel() {
		oauthConfigStore.cancelWizard();
		onCancel?.();
	}

	function canProceed(): boolean {
		if (!wizardState) return false;

		switch (wizardState.step) {
			case 'provider':
				return !!wizardState.selectedTemplate;
			case 'discovery':
				return !!discoveredMetadata || wizardState.selectedTemplate?.id === 'custom';
			case 'credentials':
				return !!wizardState.config.client_id;
			case 'scopes':
				return (wizardState.config.scopes?.length ?? 0) > 0;
			case 'review':
				return true;
			default:
				return false;
		}
	}
</script>

{#if wizardState}
	<div class="wizard-modal">
		<div class="wizard-overlay" onclick={cancel}></div>

		<div class="wizard-container">
			<!-- Header -->
			<div class="wizard-header">
				<div class="header-title">
					<h2>Configure OAuth Provider</h2>
					<p>{serverName}</p>
				</div>
				<button class="btn-close" onclick={cancel}>
					<X size={20} />
				</button>
			</div>

			<!-- Progress Steps -->
			<div class="wizard-progress">
				{#each ['provider', 'discovery', 'credentials', 'scopes', 'review'] as step, index}
					{@const isActive = wizardState.step === step}
					{@const isComplete =
						['provider', 'discovery', 'credentials', 'scopes', 'review'].indexOf(wizardState.step) >
						index}

					<div class="progress-step" class:active={isActive} class:complete={isComplete}>
						<div class="step-marker">
							{#if isComplete}
								<CheckCircle size={20} />
							{:else}
								<span class="step-number">{index + 1}</span>
							{/if}
						</div>
						<span class="step-label">{step}</span>
					</div>

					{#if index < 4}
						<div class="step-connector"></div>
					{/if}
				{/each}
			</div>

			<!-- Step Content -->
			<div class="wizard-content">
				{#if wizardState.step === 'provider'}
					<!-- Provider Selection -->
					<div class="step-section">
						<h3>Select OAuth Provider</h3>
						<p class="step-description">
							Choose a pre-configured provider or set up a custom OAuth 2.1 server.
						</p>

						<div class="provider-grid">
							{#each templates as template}
								<button
									class="provider-card"
									class:selected={wizardState.selectedTemplate?.id === template.id}
									onclick={() => selectTemplate(template)}
								>
									<div class="provider-logo">
										{#if template.id === 'github'}
											🐙
										{:else if template.id === 'google'}
											🔍
										{:else if template.id === 'microsoft'}
											🪟
										{:else if template.id === 'gitlab'}
											🦊
										{:else if template.id === 'auth0'}
											🔒
										{:else if template.id === 'okta'}
											🔐
										{:else if template.id === 'keycloak'}
											🔑
										{:else if template.id === 'generic'}
											⚙️
										{:else}
											🔐
										{/if}
									</div>
									<div class="provider-info">
										<h4>{template.name}</h4>
										<p>{template.description}</p>

										<div class="provider-features">
											{#if template.supports_pkce}
												<span class="feature-badge">PKCE</span>
											{/if}
											{#if template.supports_dpop}
												<span class="feature-badge dpop">DPoP</span>
											{/if}
										</div>
									</div>
								</button>
							{/each}
						</div>
					</div>
				{:else if wizardState.step === 'discovery'}
					<!-- Auto-Discovery -->
					<div class="step-section">
						<h3>OAuth Metadata Discovery</h3>
						<p class="step-description">
							Automatically discovering OAuth endpoints from the server...
						</p>

						{#if isDiscovering}
							<div class="discovery-loading">
								<Loader2 size={48} class="spinning" />
								<p>Discovering OAuth metadata...</p>
							</div>
						{:else if discoveryError}
							<div class="discovery-error">
								<AlertCircle size={24} />
								<div>
									<strong>Discovery Failed</strong>
									<p>{discoveryError}</p>
									<p class="error-hint">
										This provider doesn't support automatic discovery. You can configure endpoints manually.
									</p>
									<div class="error-actions">
										<button class="btn-retry" onclick={() => discoverMetadata(wizardState.config.auth_server_url || '')}>
											Try Again
										</button>
										<button class="btn-manual" onclick={configureManually}>
											Configure Manually
										</button>
									</div>
								</div>
							</div>
						{:else if discoveredMetadata}
							<div class="discovery-success">
								<CheckCircle size={24} />
								<div>
									<strong>Discovery Successful</strong>
									<p>Found OAuth endpoints using {discoveredMetadata.discovery_method}</p>

									<div class="discovered-endpoints">
										{#if discoveredMetadata.auth_server}
											<div class="endpoint">
												<strong>Authorization:</strong>
												<code>{discoveredMetadata.auth_server.authorization_endpoint}</code>
											</div>
											<div class="endpoint">
												<strong>Token:</strong>
												<code>{discoveredMetadata.auth_server.token_endpoint}</code>
											</div>
										{/if}
									</div>
								</div>
							</div>
						{/if}
					</div>
				{:else if wizardState.step === 'credentials'}
					<!-- Credentials Configuration -->
					<div class="step-section">
						<h3>OAuth Credentials</h3>
						<p class="step-description">
							Enter your OAuth client credentials. You can obtain these from your provider's developer
							portal.
						</p>

						<div class="form-fields">
							<div class="form-field">
								<label for="client-id">
									<Key size={16} />
									Client ID
									<span class="required">*</span>
								</label>
								<input
									id="client-id"
									type="text"
									placeholder="Enter client ID"
									value={wizardState.config.client_id || ''}
									oninput={(e) => updateConfig('client_id', e.currentTarget.value)}
								/>
								<span class="field-hint">Your OAuth client identifier</span>
							</div>

							<div class="form-field">
								<label for="client-secret">
									<Shield size={16} />
									Client Secret
									{#if wizardState.selectedTemplate?.id === 'github' || wizardState.selectedTemplate?.id === 'google' || wizardState.selectedTemplate?.id === 'microsoft'}
										<span class="required">*</span>
									{/if}
								</label>
								<input
									id="client-secret"
									type="password"
									placeholder="Enter client secret (if required)"
									value={wizardState.config.client_secret || ''}
									oninput={(e) => updateConfig('client_secret', e.currentTarget.value)}
								/>
								<span class="field-hint">Your OAuth client secret (stored securely in OS keyring)</span>
							</div>

							<div class="form-field">
								<label for="redirect-uri">
									Redirect URI
								</label>
								<input
									id="redirect-uri"
									type="text"
									value={wizardState.config.redirect_uri}
									readonly
								/>
								<span class="field-hint">Configure this in your provider's settings</span>
							</div>

							<div class="form-field">
								<label for="resource-uri">
									Resource URI (MCP Server)
									<span class="required">*</span>
								</label>
								<input
									id="resource-uri"
									type="text"
									placeholder="https://mcp.example.com"
									value={wizardState.config.resource_uri || ''}
									oninput={(e) => updateConfig('resource_uri', e.currentTarget.value)}
								/>
								<span class="field-hint">The MCP server resource identifier</span>
							</div>
						</div>

						{#if wizardState.selectedTemplate?.docs_url}
							<a
								href={wizardState.selectedTemplate.docs_url}
								target="_blank"
								class="docs-link"
							>
								View {wizardState.selectedTemplate.name} Documentation →
							</a>
						{/if}
					</div>
				{:else if wizardState.step === 'scopes'}
					<!-- Scopes Configuration -->
					<div class="step-section">
						<h3>OAuth Scopes</h3>
						<p class="step-description">
							Select the permissions your application needs from the MCP server.
						</p>

						<div class="scopes-list">
							{#if wizardState.selectedTemplate && wizardState.selectedTemplate.default_scopes}
								{@const defaultScopes = wizardState.selectedTemplate.default_scopes.split(' ').filter(s => s.trim())}
								{#each defaultScopes as scope}
									{@const isSelected = wizardState.config.scopes?.includes(scope)}
									<label class="scope-checkbox">
										<input
											type="checkbox"
											checked={isSelected}
											onchange={(e) => {
												const scopes = wizardState.config.scopes || [];
												if (e.currentTarget.checked) {
													updateConfig('scopes', [...scopes, scope]);
												} else {
													updateConfig(
														'scopes',
														scopes.filter((s) => s !== scope)
													);
												}
											}}
										/>
										<div class="scope-info">
											<strong>{scope}</strong>
											<p>Access to {scope} resources</p>
										</div>
									</label>
								{/each}
							{/if}

							<div class="custom-scope">
								<label for="custom-scope-input">Add Custom Scope</label>
								<input
									id="custom-scope-input"
									type="text"
									placeholder="custom.scope.name"
									onkeypress={(e) => {
										if (e.key === 'Enter') {
											const value = e.currentTarget.value.trim();
											if (value) {
												const scopes = wizardState.config.scopes || [];
												updateConfig('scopes', [...scopes, value]);
												e.currentTarget.value = '';
											}
										}
									}}
								/>
							</div>
						</div>

						<!-- Security Options -->
						<div class="security-options">
							<h4>Security Options</h4>

							<label class="option-checkbox">
								<input
									type="checkbox"
									checked={wizardState.config.use_pkce ?? true}
									onchange={(e) => updateConfig('use_pkce', e.currentTarget.checked)}
								/>
								<div class="option-info">
									<strong>Enable PKCE (Recommended)</strong>
									<p>Proof Key for Code Exchange - prevents authorization code interception</p>
								</div>
							</label>

							<label class="option-checkbox">
								<input
									type="checkbox"
									checked={wizardState.config.use_dpop ?? false}
									onchange={(e) => updateConfig('use_dpop', e.currentTarget.checked)}
									disabled={!wizardState.selectedTemplate?.supports_dpop}
								/>
								<div class="option-info">
									<strong>Enable DPoP (Advanced)</strong>
									<p>Demonstr of Proof-of-Possession - binds tokens to client</p>
									{#if !wizardState.selectedTemplate?.supports_dpop}
										<span class="not-supported">(Not supported by this provider)</span>
									{/if}
								</div>
							</label>
						</div>
					</div>
				{:else if wizardState.step === 'review'}
					<!-- Review & Confirm -->
					<div class="step-section">
						<h3>Review Configuration</h3>
						<p class="step-description">
							Review your OAuth configuration before saving. You can start the authorization flow
							immediately after.
						</p>

						<div class="review-sections">
							<div class="review-section">
								<h4>Provider</h4>
								<div class="review-row">
									<span class="label">Name:</span>
									<span class="value">{wizardState.selectedTemplate?.name}</span>
								</div>
								<div class="review-row">
									<span class="label">Auth Server:</span>
									<span class="value mono">{wizardState.config.auth_server_url}</span>
								</div>
							</div>

							<div class="review-section">
								<h4>Credentials</h4>
								<div class="review-row">
									<span class="label">Client ID:</span>
									<span class="value mono">{wizardState.config.client_id}</span>
								</div>
								{#if wizardState.config.client_secret}
									<div class="review-row">
										<span class="label">Client Secret:</span>
										<span class="value mono">••••••••</span>
									</div>
								{/if}
								<div class="review-row">
									<span class="label">Resource URI:</span>
									<span class="value mono">{wizardState.config.resource_uri}</span>
								</div>
							</div>

							<div class="review-section">
								<h4>Permissions</h4>
								<div class="review-row">
									<span class="label">Scopes:</span>
									<div class="scopes-badges">
										{#each wizardState.config.scopes || [] as scope}
											<span class="scope-badge">{scope}</span>
										{/each}
									</div>
								</div>
							</div>

							<div class="review-section">
								<h4>Security</h4>
								<div class="review-row">
									<span class="label">PKCE:</span>
									<span class="value">
										{wizardState.config.use_pkce ? 'Enabled ✓' : 'Disabled'}
									</span>
								</div>
								<div class="review-row">
									<span class="label">DPoP:</span>
									<span class="value">
										{wizardState.config.use_dpop ? 'Enabled ✓' : 'Disabled'}
									</span>
								</div>
							</div>
						</div>
					</div>
				{/if}
			</div>

			<!-- Footer Actions -->
			<div class="wizard-footer">
				<button class="btn-secondary" onclick={wizardState.step === 'provider' ? cancel : previousStep}>
					{#if wizardState.step === 'provider'}
						Cancel
					{:else}
						<ChevronLeft size={16} />
						Back
					{/if}
				</button>

				{#if wizardState.step === 'review'}
					<button class="btn-primary" onclick={complete} disabled={!canProceed()}>
						<CheckCircle size={16} />
						Complete Setup
					</button>
				{:else}
					<button class="btn-primary" onclick={nextStep} disabled={!canProceed()}>
						Next
						<ChevronRight size={16} />
					</button>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.wizard-modal {
		position: fixed;
		inset: 0;
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.wizard-overlay {
		position: absolute;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		backdrop-filter: blur(4px);
	}

	.wizard-container {
		position: relative;
		width: 90%;
		max-width: 800px;
		max-height: 90vh;
		background: var(--bg-primary);
		border-radius: 12px;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.3);
		display: flex;
		flex-direction: column;
	}

	.wizard-header {
		display: flex;
		justify-content: space-between;
		align-items: start;
		padding: 2rem;
		border-bottom: 1px solid var(--border-color);
	}

	.header-title h2 {
		margin: 0 0 0.5rem 0;
		font-size: 1.5rem;
		font-weight: 600;
	}

	.header-title p {
		margin: 0;
		color: var(--text-secondary);
		font-size: 0.875rem;
	}

	.btn-close {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		padding: 0;
		background: var(--bg-tertiary);
		border: none;
		border-radius: 6px;
		color: var(--text-secondary);
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-close:hover {
		background: var(--bg-error);
		color: white;
	}

	/* Progress */
	.wizard-progress {
		display: flex;
		align-items: center;
		padding: 1.5rem 2rem;
		background: var(--bg-secondary);
		overflow-x: auto;
	}

	.progress-step {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		opacity: 0.4;
		transition: opacity 0.3s;
	}

	.progress-step.active,
	.progress-step.complete {
		opacity: 1;
	}

	.step-marker {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		border-radius: 50%;
		background: var(--bg-tertiary);
		color: var(--text-secondary);
		font-weight: 600;
		transition: all 0.3s;
	}

	.progress-step.active .step-marker {
		background: var(--accent-color);
		color: white;
		box-shadow: 0 0 0 4px var(--bg-accent-light);
	}

	.progress-step.complete .step-marker {
		background: var(--bg-success);
		color: white;
	}

	.step-number {
		font-size: 0.875rem;
	}

	.step-label {
		font-size: 0.75rem;
		text-transform: capitalize;
		white-space: nowrap;
	}

	.step-connector {
		flex: 1;
		min-width: 40px;
		height: 2px;
		background: var(--border-color);
		margin: 0 0.5rem;
	}

	/* Content */
	.wizard-content {
		flex: 1;
		overflow-y: auto;
		padding: 2rem;
	}

	.step-section h3 {
		margin: 0 0 0.5rem 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.step-description {
		margin: 0 0 1.5rem 0;
		color: var(--text-secondary);
	}

	/* Provider Grid */
	.provider-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
		gap: 1rem;
	}

	.provider-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
		padding: 1.5rem;
		background: var(--bg-secondary);
		border: 2px solid var(--border-color);
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
		text-align: center;
	}

	.provider-card:hover {
		border-color: var(--accent-color);
		background: var(--bg-hover);
	}

	.provider-card.selected {
		border-color: var(--accent-color);
		background: var(--bg-accent-light);
		box-shadow: 0 0 0 3px var(--bg-accent-light);
	}

	.provider-logo {
		font-size: 3rem;
	}

	.provider-info h4 {
		margin: 0 0 0.5rem 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.provider-info p {
		margin: 0 0 1rem 0;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.provider-features {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		justify-content: center;
	}

	.feature-badge {
		padding: 0.25rem 0.75rem;
		background: var(--bg-tertiary);
		border-radius: 12px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.feature-badge.dpop {
		background: var(--accent-color);
		color: white;
	}

	.feature-badge.secret {
		background: var(--bg-warning-light);
		color: var(--text-warning);
	}

	/* Discovery */
	.discovery-loading,
	.discovery-error,
	.discovery-success {
		display: flex;
		align-items: center;
		gap: 1.5rem;
		padding: 2rem;
		border-radius: 8px;
	}

	.discovery-loading {
		background: var(--bg-info-light);
		color: var(--text-info);
	}

	.discovery-error {
		background: var(--bg-error-light);
		border: 1px solid var(--border-error);
		color: var(--text-error);
	}

	.discovery-error .error-hint {
		margin-top: 0.5rem;
		font-size: 0.875rem;
		opacity: 0.9;
	}

	.discovery-error .error-actions {
		display: flex;
		gap: 0.75rem;
		margin-top: 1rem;
	}

	.btn-manual {
		padding: 0.5rem 1rem;
		background: var(--bg-primary);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		border-radius: 6px;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-manual:hover {
		background: var(--bg-hover);
		border-color: var(--border-hover);
	}

	.discovery-success {
		background: var(--bg-success-light);
		border: 1px solid var(--border-success);
		color: var(--text-success);
	}

	.spinning {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.discovered-endpoints {
		margin-top: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.endpoint {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.endpoint code {
		padding: 0.5rem;
		background: var(--bg-code);
		border-radius: 4px;
		font-size: 0.875rem;
		font-family: 'Courier New', monospace;
	}

	.btn-retry {
		margin-top: 1rem;
		padding: 0.5rem 1rem;
		background: var(--accent-color);
		color: white;
		border: none;
		border-radius: 6px;
		cursor: pointer;
	}

	/* Form */
	.form-fields {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.form-field {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.form-field label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-weight: 500;
		color: var(--text-primary);
	}

	.required {
		color: var(--text-error);
	}

	.form-field input {
		padding: 0.75rem 1rem;
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 6px;
		color: var(--text-primary);
		font-size: 0.875rem;
	}

	.form-field input:focus {
		outline: none;
		border-color: var(--accent-color);
		box-shadow: 0 0 0 3px var(--bg-accent-light);
	}

	.form-field input:read-only {
		background: var(--bg-tertiary);
		color: var(--text-tertiary);
	}

	.field-hint {
		font-size: 0.8125rem;
		color: var(--text-tertiary);
	}

	.docs-link {
		display: inline-flex;
		align-items: center;
		margin-top: 1rem;
		color: var(--accent-color);
		text-decoration: none;
		font-size: 0.875rem;
		font-weight: 500;
	}

	.docs-link:hover {
		text-decoration: underline;
	}

	/* Scopes */
	.scopes-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.scope-checkbox {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.scope-checkbox:hover {
		background: var(--bg-hover);
		border-color: var(--accent-color);
	}

	.scope-checkbox input[type='checkbox'] {
		width: 20px;
		height: 20px;
		cursor: pointer;
	}

	.scope-info {
		flex: 1;
	}

	.scope-info strong {
		display: block;
		margin-bottom: 0.25rem;
	}

	.scope-info p {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.custom-scope {
		padding: 1rem;
		background: var(--bg-tertiary);
		border-radius: 8px;
	}

	.custom-scope label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 500;
		font-size: 0.875rem;
	}

	.custom-scope input {
		width: 100%;
		padding: 0.75rem;
		background: var(--bg-primary);
		border: 1px solid var(--border-color);
		border-radius: 6px;
		font-family: 'Courier New', monospace;
	}

	/* Security Options */
	.security-options {
		margin-top: 2rem;
		padding-top: 2rem;
		border-top: 1px solid var(--border-color);
	}

	.security-options h4 {
		margin: 0 0 1rem 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.option-checkbox {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		margin-bottom: 0.75rem;
		cursor: pointer;
	}

	.option-checkbox input[type='checkbox']:disabled {
		cursor: not-allowed;
		opacity: 0.5;
	}

	.option-info {
		flex: 1;
	}

	.option-info strong {
		display: block;
		margin-bottom: 0.25rem;
	}

	.option-info p {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.not-supported {
		font-size: 0.8125rem;
		color: var(--text-tertiary);
		font-style: italic;
	}

	/* Review */
	.review-sections {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.review-section {
		padding: 1.5rem;
		background: var(--bg-secondary);
		border-radius: 8px;
	}

	.review-section h4 {
		margin: 0 0 1rem 0;
		font-size: 1rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.review-row {
		display: flex;
		justify-content: space-between;
		padding: 0.75rem 0;
		border-bottom: 1px solid var(--border-color);
	}

	.review-row:last-child {
		border-bottom: none;
	}

	.review-row .label {
		font-weight: 500;
		color: var(--text-secondary);
	}

	.review-row .value {
		color: var(--text-primary);
	}

	.review-row .value.mono {
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
	}

	.scopes-badges {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.scope-badge {
		padding: 0.25rem 0.75rem;
		background: var(--accent-color);
		color: white;
		border-radius: 12px;
		font-size: 0.875rem;
		font-weight: 500;
	}

	/* Footer */
	.wizard-footer {
		display: flex;
		justify-content: space-between;
		padding: 1.5rem 2rem;
		border-top: 1px solid var(--border-color);
		background: var(--bg-secondary);
	}

	.btn-primary,
	.btn-secondary {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1.5rem;
		border: none;
		border-radius: 6px;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-primary {
		background: var(--accent-color);
		color: white;
	}

	.btn-primary:hover:not(:disabled) {
		opacity: 0.9;
	}

	.btn-primary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-secondary {
		background: var(--bg-tertiary);
		color: var(--text-primary);
	}

	.btn-secondary:hover {
		background: var(--bg-hover);
	}
</style>
