<script lang="ts">
	import { oauthFlowStore } from '$lib/stores/oauthFlowStore';
	import { oauthConfigStore } from '$lib/stores/oauthConfigStore';
	import { tokenStore } from '$lib/stores/tokenStore';
	import FlowVisualizer from './FlowVisualizer.svelte';
	import TokenInspector from './TokenInspector.svelte';
	import ProviderConfigWizard from './ProviderConfigWizard.svelte';
	import DPoPInspector from './DPoPInspector.svelte';
	import {
		Plus,
		Play,
		Trash2,
		Settings,
		Eye,
		Shield,
		Zap,
		AlertCircle,
		CheckCircle
	} from 'lucide-svelte';

	// Active tab state
	let activeTab: 'flows' | 'tokens' | 'dpop' | 'config' = $state('flows');

	// Wizard state
	let showWizard = $state(false);
	let wizardServerId = $state<number>(0);
	let wizardServerName = $state<string>('');

	// Destructure the derived stores
	const { flows: flowsStore, selectedFlowId: selectedFlowIdStore, activeFlowCount: activeFlowCountStore } = oauthFlowStore;
	const { configs: configsStore, serversWithTokens: serversWithTokensStore } = oauthConfigStore;

	// Subscribe to the stores
	let flows = $derived($flowsStore);
	let selectedFlowId = $derived($selectedFlowIdStore);
	let activeFlowCount = $derived($activeFlowCountStore);
	let serverConfigs = $derived($configsStore);
	let serversWithTokens = $derived($serversWithTokensStore);

	// Flow list as array
	let flowList = $derived(Array.from(flows.values()).reverse());

	function openWizard(serverId: number, serverName: string) {
		wizardServerId = serverId;
		wizardServerName = serverName;
		showWizard = true;
	}

	function closeWizard() {
		showWizard = false;
	}

	async function startFlow(serverId: number, serverName: string) {
		// Check if config exists, otherwise open wizard
		const config = serverConfigs.get(serverId);
		if (!config) {
			openWizard(serverId, serverName);
			return;
		}

		// Start flow with existing config
		try {
			await oauthFlowStore.startAuthorizationFlow(serverId, {
				protocol_version: config.protocol_version,
				auth_server_url: config.auth_server_url,
				token_endpoint: config.token_endpoint,
				client_id: config.client_id,
				client_secret: config.client_secret,
				redirect_uri: config.redirect_uri,
				scopes: config.scopes,
				resource_uri: config.resource_uri,
				use_pkce: config.use_pkce,
				use_dpop: config.use_dpop,
				metadata: config.metadata
			});
		} catch (error) {
			console.error('Failed to start OAuth flow:', error);
		}
	}

	function selectFlow(flowId: string) {
		oauthFlowStore.selectFlow(flowId);
		activeTab = 'flows';
	}

	function clearCompleted() {
		oauthFlowStore.clearCompletedFlows();
	}

	function getFlowStatusColor(
		state: string
	): 'green' | 'blue' | 'yellow' | 'red' | 'gray' {
		switch (state) {
			case 'Complete':
				return 'green';
			case 'AwaitingUserAuth':
			case 'ExchangingCode':
				return 'blue';
			case 'Initializing':
				return 'yellow';
			case 'Failed':
				return 'red';
			case 'Cancelled':
				return 'gray';
			default:
				return 'gray';
		}
	}
</script>

<div class="oauth-debugger">
	<!-- Header -->
	<div class="debugger-header">
		<div class="header-title">
			<Zap size={28} class="title-icon" />
			<div>
				<h2>OAuth Visual Debugger</h2>
				<p>Monitor and debug OAuth 2.1 flows in real-time</p>
			</div>
		</div>

		<div class="header-actions">
			<button class="btn-action" onclick={() => openWizard(1, 'Test Server')}>
				<Plus size={18} />
				New Flow
			</button>

			{#if flowList.length > 0}
				<button class="btn-action secondary" onclick={clearCompleted}>
					<Trash2 size={18} />
					Clear Completed
				</button>
			{/if}
		</div>
	</div>

	<!-- Stats Bar -->
	<div class="stats-bar">
		<div class="stat-card">
			<div class="stat-icon blue">
				<Play size={20} />
			</div>
			<div class="stat-content">
				<span class="stat-value">{activeFlowCount}</span>
				<span class="stat-label">Active Flows</span>
			</div>
		</div>

		<div class="stat-card">
			<div class="stat-icon green">
				<CheckCircle size={20} />
			</div>
			<div class="stat-content">
				<span class="stat-value">{serversWithTokens.length}</span>
				<span class="stat-label">Authorized Servers</span>
			</div>
		</div>

		<div class="stat-card">
			<div class="stat-icon purple">
				<Shield size={20} />
			</div>
			<div class="stat-content">
				<span class="stat-value">{flowList.filter((f) => f.config.use_dpop).length}</span>
				<span class="stat-label">DPoP Enabled</span>
			</div>
		</div>
	</div>

	<!-- Tabs -->
	<div class="tabs">
		<button class="tab" class:active={activeTab === 'flows'} onclick={() => (activeTab = 'flows')}>
			<Play size={16} />
			OAuth Flows
			{#if activeFlowCount > 0}
				<span class="tab-badge">{activeFlowCount}</span>
			{/if}
		</button>

		<button
			class="tab"
			class:active={activeTab === 'tokens'}
			onclick={() => (activeTab = 'tokens')}
		>
			<Eye size={16} />
			Token Inspector
			{#if serversWithTokens.length > 0}
				<span class="tab-badge">{serversWithTokens.length}</span>
			{/if}
		</button>

		<button class="tab" class:active={activeTab === 'dpop'} onclick={() => (activeTab = 'dpop')}>
			<Shield size={16} />
			DPoP Inspector
		</button>

		<button
			class="tab"
			class:active={activeTab === 'config'}
			onclick={() => (activeTab = 'config')}
		>
			<Settings size={16} />
			Configuration
		</button>
	</div>

	<!-- Content -->
	<div class="content">
		{#if activeTab === 'flows'}
			<div class="flows-view">
				{#if flowList.length === 0}
					<div class="empty-state">
						<Zap size={64} class="empty-icon" />
						<h3>No OAuth Flows</h3>
						<p>Start a new OAuth authorization flow to begin debugging</p>
						<button class="btn-primary" onclick={() => openWizard(1, 'Test Server')}>
							<Plus size={18} />
							Start OAuth Flow
						</button>
					</div>
				{:else}
					<div class="flows-layout">
						<!-- Flow List -->
						<div class="flow-list">
							<h3>Active Flows</h3>
							{#each flowList as flow}
								{@const statusColor = getFlowStatusColor(flow.state)}
								<button
									class="flow-item"
									class:selected={selectedFlowId === flow.flow_id}
									onclick={() => selectFlow(flow.flow_id)}
								>
									<div class="flow-status {statusColor}"></div>
									<div class="flow-info">
										<strong>Server {flow.server_id}</strong>
										<span class="flow-state">{flow.state}</span>
									</div>
									<span class="flow-time">
										{new Date(flow.started_at).toLocaleTimeString()}
									</span>
								</button>
							{/each}
						</div>

						<!-- Flow Details -->
						<div class="flow-details">
							{#if selectedFlowId}
								<FlowVisualizer flowId={selectedFlowId} />
							{:else}
								<div class="no-selection">
									<AlertCircle size={48} />
									<p>Select a flow to view details</p>
								</div>
							{/if}
						</div>
					</div>
				{/if}
			</div>
		{:else if activeTab === 'tokens'}
			<div class="tokens-view">
				{#if serversWithTokens.length === 0}
					<div class="empty-state">
						<Eye size={64} class="empty-icon" />
						<h3>No Tokens Available</h3>
						<p>Complete an OAuth flow to inspect access tokens</p>
					</div>
				{:else}
					<div class="tokens-layout">
						<div class="server-list">
							<h3>Servers with Tokens</h3>
							{#each serversWithTokens as server}
								<button class="server-item" onclick={() => activeTab = 'tokens'}>
									<CheckCircle size={16} class="server-icon" />
									<span>{server.server_name}</span>
								</button>
							{/each}
						</div>

						<div class="token-details">
							{#if serversWithTokens[0]}
								<TokenInspector
									serverId={serversWithTokens[0].server_id}
									serverName={serversWithTokens[0].server_name}
								/>
							{/if}
						</div>
					</div>
				{/if}
			</div>
		{:else if activeTab === 'dpop'}
			<div class="dpop-view">
				{#if serversWithTokens.length === 0}
					<div class="empty-state">
						<Shield size={64} class="empty-icon" />
						<h3>No DPoP Data Available</h3>
						<p>Complete an OAuth flow with DPoP enabled to inspect proofs</p>
					</div>
				{:else}
					<DPoPInspector
						token={serversWithTokens.find((s) => s.use_dpop)}
						flow={undefined}
					/>
				{/if}
			</div>
		{:else if activeTab === 'config'}
			<div class="config-view">
				<div class="config-header">
					<h3>OAuth Server Configurations</h3>
					<button class="btn-primary" onclick={() => openWizard(1, 'New Server')}>
						<Plus size={16} />
						Add Server
					</button>
				</div>

				{#if serverConfigs.size === 0}
					<div class="empty-state">
						<Settings size={64} class="empty-icon" />
						<h3>No Configurations</h3>
						<p>Add an OAuth server configuration to get started</p>
					</div>
				{:else}
					<div class="config-grid">
						{#each Array.from(serverConfigs.values()) as serverConfig}
							<div class="config-card">
								<div class="config-header-info">
									<h4>{serverConfig.server_name}</h4>
									{#if serverConfig.has_valid_token}
										<CheckCircle size={16} class="status-icon valid" />
									{:else}
										<AlertCircle size={16} class="status-icon invalid" />
									{/if}
								</div>

								<div class="config-details">
									<div class="detail-row">
										<span class="label">Provider:</span>
										<span class="value">{serverConfig.auth_server_url}</span>
									</div>
									<div class="detail-row">
										<span class="label">Scopes:</span>
										<span class="value">{serverConfig.scopes.length} configured</span>
									</div>
									<div class="detail-row">
										<span class="label">Security:</span>
										<div class="security-badges">
											{#if serverConfig.use_pkce}
												<span class="badge">PKCE</span>
											{/if}
											{#if serverConfig.use_dpop}
												<span class="badge dpop">DPoP</span>
											{/if}
										</div>
									</div>
								</div>

								<div class="config-actions">
									<button class="btn-sm" onclick={() => startFlow(serverConfig.server_id, serverConfig.server_name)}>
										<Play size={14} />
										Start Flow
									</button>
									<button class="btn-sm secondary">
										<Settings size={14} />
										Edit
									</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>

<!-- Wizard Modal -->
{#if showWizard}
	<ProviderConfigWizard
		serverId={wizardServerId}
		serverName={wizardServerName}
		onComplete={closeWizard}
		onCancel={closeWizard}
	/>
{/if}

<style>
	.oauth-debugger {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--bg-primary);
	}

	/* Header */
	.debugger-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem 2rem;
		border-bottom: 1px solid var(--border-color);
		background: var(--bg-secondary);
	}

	.header-title {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.title-icon {
		color: var(--accent-color);
	}

	.header-title h2 {
		margin: 0 0 0.25rem 0;
		font-size: 1.5rem;
		font-weight: 600;
	}

	.header-title p {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.header-actions {
		display: flex;
		gap: 0.75rem;
	}

	.btn-action {
		display: flex;
		align-items: center;
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

	.btn-action:hover {
		opacity: 0.9;
	}

	.btn-action.secondary {
		background: var(--bg-tertiary);
		color: var(--text-primary);
	}

	/* Stats Bar */
	.stats-bar {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
		padding: 1rem 2rem;
		background: var(--bg-tertiary);
		border-bottom: 1px solid var(--border-color);
	}

	.stat-card {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 1rem;
		background: var(--bg-primary);
		border-radius: 8px;
	}

	.stat-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 48px;
		height: 48px;
		border-radius: 50%;
	}

	.stat-icon.blue {
		background: var(--bg-info-light);
		color: var(--text-info);
	}
	.stat-icon.green {
		background: var(--bg-success-light);
		color: var(--text-success);
	}
	.stat-icon.purple {
		background: var(--accent-color);
		color: white;
	}

	.stat-content {
		display: flex;
		flex-direction: column;
	}

	.stat-value {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--text-primary);
	}

	.stat-label {
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	/* Tabs */
	.tabs {
		display: flex;
		gap: 0.5rem;
		padding: 1rem 2rem 0;
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border-color);
	}

	.tab {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1.5rem;
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--text-secondary);
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		position: relative;
	}

	.tab:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}

	.tab.active {
		color: var(--accent-color);
		border-bottom-color: var(--accent-color);
	}

	.tab-badge {
		padding: 0.125rem 0.5rem;
		background: var(--accent-color);
		color: white;
		border-radius: 10px;
		font-size: 0.75rem;
		font-weight: 600;
	}

	/* Content */
	.content {
		flex: 1;
		overflow: auto;
		padding: 2rem;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		padding: 4rem 2rem;
		text-align: center;
		color: var(--text-tertiary);
	}

	.empty-icon {
		opacity: 0.3;
	}

	.empty-state h3 {
		margin: 0;
		color: var(--text-primary);
		font-size: 1.25rem;
	}

	.empty-state p {
		margin: 0;
		max-width: 400px;
	}

	.btn-primary {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 1rem;
		padding: 0.75rem 1.5rem;
		background: var(--accent-color);
		color: white;
		border: none;
		border-radius: 6px;
		font-weight: 500;
		cursor: pointer;
	}

	/* Flows View */
	.flows-layout {
		display: grid;
		grid-template-columns: 300px 1fr;
		gap: 1.5rem;
		height: 100%;
	}

	.flow-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1rem;
		background: var(--bg-secondary);
		border-radius: 8px;
		height: fit-content;
	}

	.flow-list h3 {
		margin: 0 0 0.5rem 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.flow-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem;
		background: var(--bg-primary);
		border: 1px solid var(--border-color);
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
	}

	.flow-item:hover {
		border-color: var(--accent-color);
		background: var(--bg-hover);
	}

	.flow-item.selected {
		border-color: var(--accent-color);
		background: var(--bg-accent-light);
	}

	.flow-status {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.flow-status.green {
		background: var(--text-success);
	}
	.flow-status.blue {
		background: var(--text-info);
	}
	.flow-status.yellow {
		background: var(--text-warning);
	}
	.flow-status.red {
		background: var(--text-error);
	}
	.flow-status.gray {
		background: var(--text-tertiary);
	}

	.flow-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.flow-info strong {
		font-size: 0.875rem;
	}

	.flow-state {
		font-size: 0.8125rem;
		color: var(--text-tertiary);
	}

	.flow-time {
		font-size: 0.75rem;
		font-family: 'Courier New', monospace;
		color: var(--text-tertiary);
	}

	.flow-details {
		min-height: 400px;
	}

	.no-selection {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		height: 100%;
		color: var(--text-tertiary);
	}

	/* Tokens View */
	.tokens-layout {
		display: grid;
		grid-template-columns: 250px 1fr;
		gap: 1.5rem;
	}

	.server-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1rem;
		background: var(--bg-secondary);
		border-radius: 8px;
		height: fit-content;
	}

	.server-list h3 {
		margin: 0 0 0.5rem 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.server-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem;
		background: var(--bg-primary);
		border: 1px solid var(--border-color);
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
	}

	.server-item:hover {
		border-color: var(--accent-color);
		background: var(--bg-hover);
	}

	.server-icon {
		color: var(--text-success);
	}

	/* Config View */
	.config-view {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.config-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.config-header h3 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.config-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: 1.5rem;
	}

	.config-card {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		padding: 1.5rem;
		background: var(--bg-secondary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
	}

	.config-header-info {
		display: flex;
		justify-content: space-between;
		align-items: start;
	}

	.config-header-info h4 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.status-icon.valid {
		color: var(--text-success);
	}
	.status-icon.invalid {
		color: var(--text-warning);
	}

	.config-details {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.detail-row {
		display: flex;
		justify-content: space-between;
		font-size: 0.875rem;
	}

	.detail-row .label {
		color: var(--text-secondary);
	}

	.detail-row .value {
		color: var(--text-primary);
		font-family: 'Courier New', monospace;
		font-size: 0.8125rem;
	}

	.security-badges {
		display: flex;
		gap: 0.5rem;
	}

	.badge {
		padding: 0.125rem 0.5rem;
		background: var(--bg-tertiary);
		border-radius: 10px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.badge.dpop {
		background: var(--accent-color);
		color: white;
	}

	.config-actions {
		display: flex;
		gap: 0.75rem;
		padding-top: 0.5rem;
		border-top: 1px solid var(--border-color);
	}

	.btn-sm {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: var(--accent-color);
		color: white;
		border: none;
		border-radius: 6px;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.btn-sm.secondary {
		background: var(--bg-tertiary);
		color: var(--text-primary);
	}
</style>
