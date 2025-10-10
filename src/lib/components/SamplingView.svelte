<!--
  Sampling View - Testing Tool Request Management

  Shows pending and completed sampling requests for manual testing workflow.
  Integrates with HITL approval modals and Protocol Inspector.
-->
<script lang="ts">
	import { samplingStore, type PendingSamplingRequest, type CompletedSamplingRequest } from '$lib/stores/samplingStore';
	import { uiStore } from '$lib/stores/uiStore';
	import { Clock, CheckCircle2, XCircle, AlertCircle, ExternalLink } from 'lucide-svelte';

	// Reactive store access
	const pending = $derived($samplingStore.pending);
	const history = $derived($samplingStore.history);
	const loading = $derived($samplingStore.loading);
	const error = $derived($samplingStore.error);

	// Helper functions
	function formatTimeAgo(timestamp: string): string {
		const now = new Date();
		const then = new Date(timestamp);
		const seconds = Math.floor((now.getTime() - then.getTime()) / 1000);

		if (seconds < 60) return `${seconds}s ago`;
		const minutes = Math.floor(seconds / 60);
		if (minutes < 60) return `${minutes}m ago`;
		const hours = Math.floor(minutes / 60);
		if (hours < 24) return `${hours}h ago`;
		const days = Math.floor(hours / 24);
		return `${days}d ago`;
	}

	function getRequestPreview(request: PendingSamplingRequest): string {
		const firstMessage = request.request.messages[0];
		if (firstMessage && typeof firstMessage.content === 'object' && 'text' in firstMessage.content) {
			const text = firstMessage.content.text || '';
			return text.substring(0, 100) + (text.length > 100 ? '...' : '');
		}
		return 'No content';
	}

	function openRequest(request: PendingSamplingRequest) {
		uiStore.showSamplingApproval(request);
	}

	function viewInProtocol(messageId: string | undefined) {
		if (messageId) {
			uiStore.jumpToProtocolInspector(messageId);
		}
	}

	function getStatusIcon(status: 'approved' | 'rejected') {
		return status === 'approved' ? CheckCircle2 : XCircle;
	}

	function getStatusColor(status: 'approved' | 'rejected') {
		return status === 'approved'
			? 'text-green-600 dark:text-green-400'
			: 'text-red-600 dark:text-red-400';
	}
</script>

<div class="h-full flex flex-col bg-gray-50 dark:bg-gray-900">
	<!-- Header -->
	<header class="flex-shrink-0 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
		<div class="flex items-center justify-between">
			<div>
				<h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">Sampling</h1>
				<p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
					Review and respond to server-initiated LLM requests
				</p>
			</div>
			<div class="flex items-center gap-4">
				<div class="text-sm">
					<span class="text-gray-600 dark:text-gray-400">Mode:</span>
					<span class="ml-2 px-2 py-1 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded font-medium">
						Manual Testing
					</span>
				</div>
			</div>
		</div>
	</header>

	<!-- Content -->
	<div class="flex-1 overflow-auto p-6 space-y-6">
		<!-- Error Message -->
		{#if error}
			<div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 flex items-start gap-3">
				<AlertCircle size={20} class="text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
				<div class="flex-1">
					<h3 class="text-sm font-semibold text-red-900 dark:text-red-100 mb-1">Error</h3>
					<p class="text-sm text-red-800 dark:text-red-200">{error}</p>
				</div>
			</div>
		{/if}

		<!-- Pending Requests Section -->
		<section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
			<div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
				<div class="flex items-center gap-2">
					<Clock size={20} class="text-gray-600 dark:text-gray-400" />
					<h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
						Pending Requests
					</h2>
					{#if pending.length > 0}
						<span class="ml-2 px-2 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded text-sm font-medium">
							{pending.length}
						</span>
					{/if}
				</div>
			</div>

			<div class="divide-y divide-gray-200 dark:divide-gray-700">
				{#if pending.length === 0}
					<div class="px-6 py-12 text-center">
						<Clock size={48} class="mx-auto text-gray-400 dark:text-gray-600 mb-3" />
						<p class="text-gray-600 dark:text-gray-400">No pending sampling requests</p>
						<p class="text-sm text-gray-500 dark:text-gray-500 mt-1">
							Requests will appear here when servers need LLM inference
						</p>
					</div>
				{:else}
					{#each pending as request}
						<div class="px-6 py-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
							<div class="flex items-start justify-between gap-4">
								<div class="flex-1 min-w-0">
									<div class="flex items-center gap-3 mb-2">
										<span class="font-medium text-gray-900 dark:text-gray-100">
											{request.serverName}
										</span>
										<span class="text-sm text-gray-500 dark:text-gray-400">
											{formatTimeAgo(request.createdAt)}
										</span>
									</div>
									<p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-2">
										{getRequestPreview(request)}
									</p>
									{#if request.estimatedCost || request.estimatedTokens}
										<div class="flex items-center gap-4 mt-2 text-xs text-gray-500 dark:text-gray-400">
											{#if request.estimatedTokens}
												<span>~{request.estimatedTokens} tokens</span>
											{/if}
											{#if request.estimatedCost}
												<span>~${request.estimatedCost.toFixed(4)}</span>
											{/if}
										</div>
									{/if}
								</div>
								<div class="flex items-center gap-2">
									{#if request.protocolMessageId}
										<button
											onclick={() => viewInProtocol(request.protocolMessageId)}
											class="px-3 py-1.5 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors flex items-center gap-1"
											title="View in Protocol Inspector"
										>
											<ExternalLink size={14} />
											<span>Protocol</span>
										</button>
									{/if}
									<button
										onclick={() => openRequest(request)}
										class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600 rounded-lg transition-colors"
									>
										Review Request →
									</button>
								</div>
							</div>
						</div>
					{/each}
				{/if}
			</div>
		</section>

		<!-- Completed Requests Section -->
		<section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
			<div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
				<div class="flex items-center gap-2">
					<CheckCircle2 size={20} class="text-gray-600 dark:text-gray-400" />
					<h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
						Completed Requests
					</h2>
					{#if history.length > 0}
						<span class="ml-2 px-2 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded text-sm font-medium">
							{history.length}
						</span>
					{/if}
				</div>
				{#if history.length > 0}
					<button
						onclick={() => samplingStore.clearHistory()}
						class="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
					>
						Clear History
					</button>
				{/if}
			</div>

			<div class="divide-y divide-gray-200 dark:divide-gray-700">
				{#if history.length === 0}
					<div class="px-6 py-12 text-center">
						<CheckCircle2 size={48} class="mx-auto text-gray-400 dark:text-gray-600 mb-3" />
						<p class="text-gray-600 dark:text-gray-400">No completed requests</p>
						<p class="text-sm text-gray-500 dark:text-gray-500 mt-1">
							Approved and rejected requests will appear here
						</p>
					</div>
				{:else}
					{#each history.slice(0, 10) as request}
						<div class="px-6 py-4">
							<div class="flex items-start justify-between gap-4">
								<div class="flex-1 min-w-0">
									<div class="flex items-center gap-3 mb-2">
										<svelte:component
											this={getStatusIcon(request.status)}
											size={16}
											class={getStatusColor(request.status)}
										/>
										<span class="text-sm font-medium capitalize {getStatusColor(request.status)}">
											{request.status}
										</span>
										<span class="text-sm text-gray-600 dark:text-gray-400">
											{request.serverName}
										</span>
										<span class="text-sm text-gray-500 dark:text-gray-400">
											{formatTimeAgo(request.completedAt)}
										</span>
									</div>
									{#if request.error}
										<p class="text-sm text-red-600 dark:text-red-400">{request.error}</p>
									{/if}
								</div>
								{#if request.protocolMessageId}
									<button
										onclick={() => viewInProtocol(request.protocolMessageId)}
										class="px-3 py-1.5 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors flex items-center gap-1"
										title="View in Protocol Inspector"
									>
										<ExternalLink size={14} />
										<span>Protocol</span>
									</button>
								{/if}
							</div>
						</div>
					{/each}
					{#if history.length > 10}
						<div class="px-6 py-4 bg-gray-50 dark:bg-gray-700/50 text-center">
							<p class="text-sm text-gray-600 dark:text-gray-400">
								Showing 10 of {history.length} completed requests
							</p>
						</div>
					{/if}
				{/if}
			</div>
		</section>
	</div>
</div>
