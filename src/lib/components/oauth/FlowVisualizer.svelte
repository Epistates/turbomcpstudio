<script lang="ts">
	import { oauthFlowStore, type OAuthFlow, type OAuthFlowStep } from '$lib/stores/oauthFlowStore';
	import { CheckCircle, XCircle, Clock, AlertCircle, Loader2, X } from 'lucide-svelte';

	interface Props {
		flowId: string;
	}

	let { flowId }: Props = $props();

	// Destructure the flows store
	const { flows: flowsStore } = oauthFlowStore;

	// Subscribe to flows
	let flows = $derived($flowsStore);
	let flow = $derived(flows.get(flowId));

	// State colors and icons
	const stateConfig = {
		Initializing: { color: 'blue', icon: Clock, label: 'Initializing' },
		AwaitingUserAuth: { color: 'yellow', icon: AlertCircle, label: 'Awaiting Authorization' },
		ExchangingCode: { color: 'blue', icon: Loader2, label: 'Exchanging Code' },
		Complete: { color: 'green', icon: CheckCircle, label: 'Complete' },
		Failed: { color: 'red', icon: XCircle, label: 'Failed' },
		Cancelled: { color: 'gray', icon: X, label: 'Cancelled' }
	};

	async function handleCancel() {
		if (!flow) return;
		try {
			await oauthFlowStore.cancelFlow(flowId);
		} catch (error) {
			console.error('Failed to cancel flow:', error);
		}
	}

	function getStepIcon(step: OAuthFlowStep) {
		if (step.error) return XCircle;
		if (step.http_response) return CheckCircle;
		return Clock;
	}

	function getStepColor(step: OAuthFlowStep): string {
		if (step.error) return 'red';
		if (step.http_response) {
			const status = step.http_response.status;
			if (status >= 200 && status < 300) return 'green';
			if (status >= 300 && status < 400) return 'yellow';
			return 'red';
		}
		return 'blue';
	}

	function formatTimestamp(timestamp: string): string {
		const date = new Date(timestamp);
		return date.toLocaleTimeString();
	}

	function formatDuration(startedAt: string, completedAt?: string): string {
		const start = new Date(startedAt);
		const end = completedAt ? new Date(completedAt) : new Date();
		const duration = end.getTime() - start.getTime();

		if (duration < 1000) return `${duration}ms`;
		if (duration < 60000) return `${(duration / 1000).toFixed(1)}s`;
		return `${(duration / 60000).toFixed(1)}m`;
	}
</script>

{#if flow}
	<div class="flow-visualizer">
		<!-- Header -->
		<div class="flow-header">
			<div class="flow-title">
				<h3>OAuth Flow</h3>
				<span class="flow-id">{flowId.slice(0, 8)}</span>
			</div>

			<!-- Current State -->
			<div class="flow-state" data-state={flow.state.toLowerCase()}>
				{#if flow.state in stateConfig}
					{@const config = stateConfig[flow.state as keyof typeof stateConfig]}
					<config.icon class="state-icon {config.color}" size={20} />
					<span class="state-label {config.color}">{config.label}</span>
				{/if}
			</div>

			<!-- Duration -->
			<div class="flow-duration">
				<Clock size={16} />
				<span>{formatDuration(flow.started_at, flow.completed_at)}</span>
			</div>

			<!-- Cancel Button -->
			{#if !['Complete', 'Failed', 'Cancelled'].includes(flow.state)}
				<button class="btn-cancel" onclick={handleCancel}>
					<X size={16} />
					Cancel
				</button>
			{/if}
		</div>

		<!-- Error Display -->
		{#if flow.error}
			<div class="flow-error">
				<XCircle size={20} />
				<div class="error-content">
					<strong>{flow.error.code}</strong>
					<p>{flow.error.description}</p>
				</div>
			</div>
		{/if}

		<!-- State Machine Visualization -->
		<div class="state-machine">
			<div class="states">
				{#each Object.entries(stateConfig) as [stateName, config]}
					<div
						class="state-node"
						class:active={flow.state === stateName}
						class:completed={flow.steps.some((s) => s.step_type.includes(stateName))}
					>
						<div class="node-icon {config.color}">
							<config.icon size={24} />
						</div>
						<span class="node-label">{config.label}</span>
					</div>
					{#if stateName !== 'Cancelled' && stateName !== 'Failed' && stateName !== 'Complete'}
						<div class="state-connector"></div>
					{/if}
				{/each}
			</div>
		</div>

		<!-- Flow Steps Timeline -->
		<div class="steps-timeline">
			<h4>Flow Steps</h4>

			{#if flow.steps.length === 0}
				<div class="no-steps">
					<Loader2 size={32} class="spinning" />
					<p>Waiting for flow to start...</p>
				</div>
			{:else}
				<div class="steps-list">
					{#each flow.steps as step, index}
						{@const stepColor = getStepColor(step)}
						{@const StepIcon = getStepIcon(step)}

						<div class="step-item">
							<div class="step-marker">
								<div class="marker-line"></div>
								<div class="marker-icon {stepColor}">
									<StepIcon size={16} />
								</div>
								{#if index < flow.steps.length - 1}
									<div class="marker-line"></div>
								{/if}
							</div>

							<div class="step-content">
								<div class="step-header">
									<span class="step-type">{step.step_type}</span>
									<span class="step-time">{formatTimestamp(step.timestamp)}</span>
								</div>

								<p class="step-description">{step.description}</p>

								<!-- HTTP Request -->
								{#if step.http_request}
									<details class="http-details">
										<summary>
											<span class="method {step.http_request.method.toLowerCase()}">
												{step.http_request.method}
											</span>
											<span class="url">{step.http_request.url}</span>
										</summary>
										<div class="details-content">
											<div class="headers">
												<strong>Headers:</strong>
												<pre>{JSON.stringify(step.http_request.headers, null, 2)}</pre>
											</div>
											{#if step.http_request.body}
												<div class="body">
													<strong>Body:</strong>
													<pre>{step.http_request.body}</pre>
												</div>
											{/if}
										</div>
									</details>
								{/if}

								<!-- HTTP Response -->
								{#if step.http_response}
									<details class="http-details">
										<summary>
											<span
												class="status status-{Math.floor(step.http_response.status / 100)}xx"
											>
												{step.http_response.status}
											</span>
											<span>Response</span>
										</summary>
										<div class="details-content">
											<div class="headers">
												<strong>Headers:</strong>
												<pre>{JSON.stringify(step.http_response.headers, null, 2)}</pre>
											</div>
											{#if step.http_response.body}
												<div class="body">
													<strong>Body:</strong>
													<pre>{JSON.stringify(JSON.parse(step.http_response.body), null, 2)}</pre>
												</div>
											{/if}
										</div>
									</details>
								{/if}

								<!-- Error -->
								{#if step.error}
									<div class="step-error">
										<XCircle size={16} />
										<span>{step.error}</span>
									</div>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</div>
{:else}
	<div class="no-flow">
		<AlertCircle size={48} />
		<p>Flow not found</p>
	</div>
{/if}

<style>
	.flow-visualizer {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		padding: 1rem;
		background: var(--bg-secondary);
		border-radius: 8px;
	}

	.flow-header {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--border-color);
	}

	.flow-title {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex: 1;
	}

	.flow-title h3 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.flow-id {
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		color: var(--text-tertiary);
		background: var(--bg-tertiary);
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
	}

	.flow-state {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: var(--bg-tertiary);
		border-radius: 6px;
	}

	.state-label {
		font-weight: 500;
	}

	.flow-duration {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--text-secondary);
		font-size: 0.875rem;
	}

	.btn-cancel {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: var(--bg-error);
		color: white;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		font-size: 0.875rem;
		transition: opacity 0.2s;
	}

	.btn-cancel:hover {
		opacity: 0.9;
	}

	.flow-error {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		background: var(--bg-error-light);
		border: 1px solid var(--border-error);
		border-radius: 6px;
		color: var(--text-error);
	}

	.error-content {
		flex: 1;
	}

	.error-content strong {
		display: block;
		margin-bottom: 0.5rem;
	}

	.error-content p {
		margin: 0;
		color: var(--text-secondary);
	}

	/* State Machine */
	.state-machine {
		padding: 1.5rem;
		background: var(--bg-primary);
		border-radius: 8px;
		overflow-x: auto;
	}

	.states {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-width: max-content;
	}

	.state-node {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 1rem;
		border-radius: 8px;
		opacity: 0.4;
		transition: all 0.3s;
	}

	.state-node.active {
		opacity: 1;
		background: var(--bg-accent-light);
		box-shadow: 0 0 0 2px var(--accent-color);
	}

	.state-node.completed {
		opacity: 0.7;
	}

	.node-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 48px;
		height: 48px;
		border-radius: 50%;
		background: var(--bg-tertiary);
	}

	.node-icon.green {
		background: var(--bg-success-light);
		color: var(--text-success);
	}

	.node-icon.blue {
		background: var(--bg-info-light);
		color: var(--text-info);
	}

	.node-icon.yellow {
		background: var(--bg-warning-light);
		color: var(--text-warning);
	}

	.node-icon.red {
		background: var(--bg-error-light);
		color: var(--text-error);
	}

	.node-icon.gray {
		background: var(--bg-tertiary);
		color: var(--text-tertiary);
	}

	.node-label {
		font-size: 0.75rem;
		font-weight: 500;
		text-align: center;
		white-space: nowrap;
	}

	.state-connector {
		flex: 0 0 24px;
		height: 2px;
		background: var(--border-color);
	}

	/* Steps Timeline */
	.steps-timeline h4 {
		margin: 0 0 1rem 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.no-steps {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
		padding: 3rem;
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

	.steps-list {
		display: flex;
		flex-direction: column;
		gap: 0;
	}

	.step-item {
		display: flex;
		gap: 1rem;
	}

	.step-marker {
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 32px;
		flex-shrink: 0;
	}

	.marker-line {
		flex: 1;
		width: 2px;
		background: var(--border-color);
	}

	.marker-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: var(--bg-tertiary);
		flex-shrink: 0;
	}

	.marker-icon.green {
		background: var(--bg-success-light);
		color: var(--text-success);
	}

	.marker-icon.blue {
		background: var(--bg-info-light);
		color: var(--text-info);
	}

	.marker-icon.red {
		background: var(--bg-error-light);
		color: var(--text-error);
	}

	.step-content {
		flex: 1;
		padding-bottom: 1.5rem;
	}

	.step-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.step-type {
		font-weight: 600;
		color: var(--text-primary);
	}

	.step-time {
		font-size: 0.875rem;
		color: var(--text-tertiary);
		font-family: 'Courier New', monospace;
	}

	.step-description {
		margin: 0 0 1rem 0;
		color: var(--text-secondary);
	}

	.http-details {
		margin-top: 0.75rem;
		border: 1px solid var(--border-color);
		border-radius: 6px;
		overflow: hidden;
	}

	.http-details summary {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1rem;
		background: var(--bg-tertiary);
		cursor: pointer;
		user-select: none;
		font-size: 0.875rem;
	}

	.http-details summary:hover {
		background: var(--bg-hover);
	}

	.method {
		font-weight: 600;
		font-family: 'Courier New', monospace;
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
	}

	.method.get {
		background: var(--bg-success-light);
		color: var(--text-success);
	}

	.method.post {
		background: var(--bg-info-light);
		color: var(--text-info);
	}

	.url {
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.status {
		font-weight: 600;
		font-family: 'Courier New', monospace;
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
	}

	.status-2xx {
		background: var(--bg-success-light);
		color: var(--text-success);
	}

	.status-3xx {
		background: var(--bg-warning-light);
		color: var(--text-warning);
	}

	.status-4xx,
	.status-5xx {
		background: var(--bg-error-light);
		color: var(--text-error);
	}

	.details-content {
		padding: 1rem;
		background: var(--bg-primary);
	}

	.details-content .headers,
	.details-content .body {
		margin-bottom: 1rem;
	}

	.details-content .headers:last-child,
	.details-content .body:last-child {
		margin-bottom: 0;
	}

	.details-content strong {
		display: block;
		margin-bottom: 0.5rem;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.details-content pre {
		margin: 0;
		padding: 0.75rem;
		background: var(--bg-code);
		border-radius: 4px;
		font-size: 0.8125rem;
		font-family: 'Courier New', monospace;
		overflow-x: auto;
		line-height: 1.5;
	}

	.step-error {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.75rem;
		padding: 0.75rem;
		background: var(--bg-error-light);
		border: 1px solid var(--border-error);
		border-radius: 6px;
		color: var(--text-error);
		font-size: 0.875rem;
	}

	.no-flow {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		padding: 4rem;
		color: var(--text-tertiary);
	}

	/* Color utilities */
	.green {
		color: var(--text-success);
	}
	.blue {
		color: var(--text-info);
	}
	.yellow {
		color: var(--text-warning);
	}
	.red {
		color: var(--text-error);
	}
	.gray {
		color: var(--text-tertiary);
	}
</style>
