<!--
  Elicitation View - User Input Request Management

  Shows active and completed elicitation requests for manual testing workflow.
  Integrates with HITL approval dialogs and Protocol Inspector.
-->
<script lang="ts">
	import { elicitationStore, elicitationReplayTemplates, type ActiveElicitationRequest, type CompletedElicitationRequest } from '$lib/stores/elicitationStore';
	import { uiStore } from '$lib/stores/uiStore';
	import { contextStore } from '$lib/stores/contextStore';
	import { Clock, CheckCircle2, XCircle, AlertCircle, ExternalLink, MessageCircle, FolderOpen, Trash2, Filter } from 'lucide-svelte';

	// Reactive store access
	const active = $derived($elicitationStore.active);
	const history = $derived($elicitationStore.history);
	const templates = $derived($elicitationReplayTemplates);
	const loading = $derived($elicitationStore.loading);
	const error = $derived($elicitationStore.error);

	// Context filtering
	const context = $derived($contextStore);
	const selectedServerId = $derived(context.selectedServerId);
	const selectedServer = $derived(context.selectedServer);

	// Filter requests by selected server (if any)
	const filteredActive = $derived(
		selectedServerId
			? active.filter(r => r.serverId === selectedServerId)
			: active
	);

	const filteredHistory = $derived(
		selectedServerId
			? history.filter(r => r.serverId === selectedServerId)
			: history
	);

	// Visual feedback message
	const filterMessage = $derived(
		selectedServerId && selectedServer
			? `Filtered to: ${selectedServer.config.name}`
			: 'Showing all servers'
	);

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

	function openRequest(request: ActiveElicitationRequest) {
		uiStore.showElicitationDialog(request);
	}

	function viewInProtocol(messageId: string | undefined) {
		if (messageId) {
			uiStore.jumpToProtocolInspector(messageId);
		}
	}

	function getStatusIcon(status: 'accepted' | 'declined' | 'cancelled') {
		return status === 'accepted' ? CheckCircle2 : status === 'declined' ? XCircle : AlertCircle;
	}

	function getStatusColor(status: 'accepted' | 'declined' | 'cancelled') {
		return status === 'accepted'
			? 'text-green-600 dark:text-green-400'
			: status === 'declined'
				? 'text-red-600 dark:text-red-400'
				: 'text-gray-600 dark:text-gray-400';
	}

	function getSchemaPreview(schema: any): string {
		if (!schema || !schema.properties) return 'No schema';
		const fields = Object.keys(schema.properties);
		return fields.length > 0
			? `${fields.length} field${fields.length !== 1 ? 's' : ''}: ${fields.slice(0, 3).join(', ')}${fields.length > 3 ? '...' : ''}`
			: 'No fields';
	}
</script>

<div class="h-full flex flex-col bg-gray-50 dark:bg-gray-900">
	<!-- Header -->
	<header class="flex-shrink-0 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
		<div class="flex items-center justify-between">
			<div>
				<h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">Elicitation</h1>
				<p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
					Server-initiated user input requests
				</p>
			</div>
			<div class="flex items-center gap-4">
				<div class="flex items-center gap-2 text-sm">
					<Filter size={16} class="text-gray-500 dark:text-gray-400" />
					<span class="text-gray-600 dark:text-gray-400">{filterMessage}</span>
				</div>
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

		<!-- Active Requests Section -->
		<section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
			<div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
				<div class="flex items-center gap-2">
					<MessageCircle size={20} class="text-gray-600 dark:text-gray-400" />
					<h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
						Active Requests
					</h2>
					{#if filteredActive.length > 0}
						<span class="ml-2 px-2 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded text-sm font-medium">
							{filteredActive.length}
						</span>
					{/if}
				</div>
			</div>

			<div class="divide-y divide-gray-200 dark:divide-gray-700">
				{#if filteredActive.length === 0}
					<div class="px-6 py-12 text-center">
						<MessageCircle size={48} class="mx-auto text-gray-400 dark:text-gray-600 mb-3" />
						<p class="text-gray-600 dark:text-gray-400">No active elicitation requests</p>
						<p class="text-sm text-gray-500 dark:text-gray-500 mt-1">
							{selectedServerId ? 'No requests from the selected server' : 'When a server requests user input, it will appear here'}
						</p>
					</div>
				{:else}
					{#each filteredActive as request}
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
									<p class="text-sm text-gray-600 dark:text-gray-400 mb-2">
										{elicitationStore.getMessagePreview(request)}
									</p>
									<div class="text-xs text-gray-500 dark:text-gray-400">
										Schema: {getSchemaPreview(request.requestedSchema)}
									</div>
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
										Provide Input →
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
					{#if filteredHistory.length > 0}
						<span class="ml-2 px-2 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded text-sm font-medium">
							{filteredHistory.length}
						</span>
					{/if}
				</div>
				{#if filteredHistory.length > 0}
					<button
						onclick={() => elicitationStore.clearHistory()}
						class="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
					>
						Clear History
					</button>
				{/if}
			</div>

			<div class="divide-y divide-gray-200 dark:divide-gray-700">
				{#if filteredHistory.length === 0}
					<div class="px-6 py-12 text-center">
						<CheckCircle2 size={48} class="mx-auto text-gray-400 dark:text-gray-600 mb-3" />
						<p class="text-gray-600 dark:text-gray-400">No completed requests</p>
						<p class="text-sm text-gray-500 dark:text-gray-500 mt-1">
							{selectedServerId ? 'No completed requests from the selected server' : 'Accepted, declined, and cancelled requests will appear here'}
						</p>
					</div>
				{:else}
					{#each filteredHistory.slice(0, 10) as request}
						{@const StatusIcon = getStatusIcon(request.status)}
						<div class="px-6 py-4 hover:bg-gray-50 dark:hover:bg-gray-700/30 transition-colors">
							<!-- Header with status and time -->
							<div class="flex items-center justify-between mb-3">
								<div class="flex items-center gap-3">
									<StatusIcon
										size={18}
										class={getStatusColor(request.status)}
									/>
									<span class="font-medium text-gray-900 dark:text-gray-100">
										{request.serverName}
									</span>
									<span class="text-sm text-gray-500 dark:text-gray-400">
										{formatTimeAgo(request.completedAt)}
									</span>
								</div>
							</div>

							<!-- Q+A+Outcome Card -->
							<div class="bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
								<!-- Request (Question) -->
								<div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
									<div class="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400 mb-1">
										📥 Request
									</div>
									<p class="text-sm text-gray-900 dark:text-gray-100 mb-2">
										{elicitationStore.getMessagePreview(request)}
									</p>
									<div class="text-xs text-gray-500 dark:text-gray-400">
										Schema: {getSchemaPreview(request.requestedSchema)}
									</div>
								</div>

								<!-- Response -->
								{#if request.response}
									<div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
										<div class="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400 mb-1">
											💬 Your Response
										</div>
										<p class="text-sm text-gray-900 dark:text-gray-100">
											{elicitationStore.getResponsePreview(request)}
										</p>
										<div class="mt-2 text-xs text-gray-500 dark:text-gray-400">
											Action: {request.response.action}
										</div>
									</div>
								{/if}

								<!-- Outcome -->
								{#if request.outcome}
									<div class="px-4 py-3 {request.outcome.status === 'success' ? 'bg-green-50 dark:bg-green-900/10' : 'bg-red-50 dark:bg-red-900/10'}">
										<div class="text-xs font-semibold uppercase tracking-wide {request.outcome.status === 'success' ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'} mb-1">
											{request.outcome.status === 'success' ? '✓ Outcome' : '⚠️ Outcome'}
										</div>
										<p class="text-sm {request.outcome.status === 'success' ? 'text-green-800 dark:text-green-200' : 'text-red-800 dark:text-red-200'}">
											{request.outcome.message || (request.outcome.status === 'success' ? 'Server processed response successfully' : 'Server reported an error')}
										</p>
									</div>
								{/if}
							</div>

							<!-- Actions -->
							<div class="flex items-center gap-2 mt-3">
								{#if request.response && request.response.action === 'accept' && request.response.content}
									<button
										onclick={() => elicitationStore.saveAsReplayTemplate(request)}
										class="px-3 py-1.5 text-sm font-medium text-blue-700 dark:text-blue-300 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-lg hover:bg-blue-100 dark:hover:bg-blue-900/50 transition-colors flex items-center gap-1.5"
									>
										🔄 Save as Replay
									</button>
								{/if}
								{#if request.protocolMessageId}
									<button
										onclick={() => viewInProtocol(request.protocolMessageId)}
										class="px-3 py-1.5 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors flex items-center gap-1.5"
										title="View in Protocol Inspector"
									>
										<ExternalLink size={14} />
										<span>Protocol</span>
									</button>
								{/if}
							</div>
						</div>
					{/each}
					{#if filteredHistory.length > 10}
						<div class="px-6 py-4 bg-gray-50 dark:bg-gray-700/50 text-center">
							<p class="text-sm text-gray-600 dark:text-gray-400">
								Showing 10 of {filteredHistory.length} completed requests
							</p>
						</div>
					{/if}
				{/if}
			</div>
		</section>

		<!-- Saved Replay Templates Section -->
		<section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
			<div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
				<div class="flex items-center gap-2">
					<FolderOpen size={20} class="text-gray-600 dark:text-gray-400" />
					<h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
						Saved Replay Templates
					</h2>
					{#if templates.length > 0}
						<span class="ml-2 px-2 py-0.5 bg-purple-100 dark:bg-purple-900 text-purple-700 dark:text-purple-300 rounded text-sm font-medium">
							{templates.length}
						</span>
					{/if}
				</div>
			</div>

			<div class="divide-y divide-gray-200 dark:divide-gray-700">
				{#if templates.length === 0}
					<div class="px-6 py-12 text-center">
						<FolderOpen size={48} class="mx-auto text-gray-400 dark:text-gray-600 mb-3" />
						<p class="text-gray-600 dark:text-gray-400">No saved templates</p>
						<p class="text-sm text-gray-500 dark:text-gray-500 mt-1">
							Click "Save as Replay" on any accepted elicitation to create reusable input templates
						</p>
					</div>
				{:else}
					{#each templates as template}
						<div class="px-6 py-4 hover:bg-gray-50 dark:hover:bg-gray-700/30 transition-colors">
							<div class="flex items-start justify-between gap-4">
								<div class="flex-1 min-w-0">
									<div class="flex items-center gap-3 mb-2">
										<span class="text-sm font-semibold text-purple-700 dark:text-purple-300">
											📁 {template.name}
										</span>
									</div>
									<p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-2 mb-2">
										{JSON.stringify(template.response).substring(0, 100)}...
									</p>
									<div class="flex items-center gap-4 text-xs text-gray-500 dark:text-gray-400">
										<span>Used {template.useCount} time{template.useCount !== 1 ? 's' : ''}</span>
										{#if template.lastUsed}
											<span>Last used: {formatTimeAgo(template.lastUsed)}</span>
										{:else}
											<span>Never used</span>
										{/if}
									</div>
								</div>
								<div class="flex items-center gap-2">
									{#if filteredActive.length > 0}
										<button
											onclick={async () => {
												const firstActive = filteredActive[0];
												try {
													await elicitationStore.useReplayTemplate(firstActive.id, template.id);
												} catch (e) {
													alert('Failed to use template: ' + e);
												}
											}}
											class="px-3 py-1.5 text-sm font-medium text-blue-700 dark:text-blue-300 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-lg hover:bg-blue-100 dark:hover:bg-blue-900/50 transition-colors flex items-center gap-1.5"
											title="Use this template on the first active request"
										>
											🔄 Use on Active
										</button>
									{/if}
									<button
										onclick={() => {
											if (confirm(`Delete template "${template.name}"?`)) {
												elicitationStore.deleteReplayTemplate(template.id);
											}
										}}
										class="px-3 py-1.5 text-sm text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300 border border-red-300 dark:border-red-700 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors flex items-center gap-1.5"
										title="Delete this template"
									>
										<Trash2 size={14} />
									</button>
								</div>
							</div>
						</div>
					{/each}
				{/if}
			</div>
		</section>
	</div>
</div>
