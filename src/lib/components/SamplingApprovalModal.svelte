<!--
  Sampling Approval Modal

  Displays server-initiated sampling requests for human-in-the-loop approval.
  Shows request details, estimated cost/tokens, and allows approve/reject/edit.
-->
<script lang="ts">
	import { samplingStore, type PendingSamplingRequest } from '$lib/stores/samplingStore';
	import { uiStore } from '$lib/stores/uiStore';
	import QuickResponseTemplates from './sampling/QuickResponseTemplates.svelte';
	import JsonViewer from './ui/JsonViewer.svelte';
	import { X, Edit, Check, XCircle, AlertTriangle, DollarSign, Hash, Server, ExternalLink } from 'lucide-svelte';

	// Props
	const { request, onClose }: { request: PendingSamplingRequest; onClose: () => void } = $props();

	// State
	let isEditing = $state(false);
	let modifiedRequest = $state({ ...request.request });
	let rejectReason = $state('');
	let showRejectDialog = $state(false);
	let isProcessing = $state(false);

	// Computed
	const hasModelPreferences = $derived(!!request.request.modelPreferences);
	const hasSystemPrompt = $derived(!!request.request.systemPrompt);
	const includeContextLabel = $derived.by(() => {
		switch (request.request.includeContext) {
			case 'none':
				return 'None (messages only)';
			case 'thisServer':
				return 'This Server Only';
			case 'allServers':
				return 'All Connected Servers';
			default:
				return 'Not specified';
		}
	});

	// Actions
	async function handleApprove() {
		if (isProcessing) return;
		isProcessing = true;

		try {
			await samplingStore.approve(request.requestId, isEditing ? modifiedRequest : undefined);
			onClose();
		} catch (error) {
			alert('Failed to approve sampling request: ' + error);
		} finally {
			isProcessing = false;
		}
	}

	async function handleReject() {
		if (!rejectReason.trim()) {
			alert('Please provide a reason for rejection');
			return;
		}

		if (isProcessing) return;
		isProcessing = true;

		try {
			await samplingStore.reject(request.requestId, rejectReason);
			onClose();
		} catch (error) {
			alert('Failed to reject sampling request: ' + error);
		} finally {
			isProcessing = false;
		}
	}

	function toggleEdit() {
		isEditing = !isEditing;
		if (!isEditing) {
			// Reset modifications
			modifiedRequest = { ...request.request };
		}
	}

	// Handle quick response from templates
	async function handleQuickResponse(response: any) {
		if (isProcessing) return;
		isProcessing = true;

		try {
			await samplingStore.submitManual(request.requestId, response);
			onClose();
		} catch (error) {
			alert('Failed to submit manual response: ' + error);
		} finally {
			isProcessing = false;
		}
	}
</script>

<!-- Modal Overlay -->
<div class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4">
	<div
		class="bg-white dark:bg-gray-800 rounded-lg shadow-2xl max-w-5xl w-full max-h-[90vh] flex flex-col"
	>
		<!-- Header -->
		<div
			class="p-6 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between flex-shrink-0"
		>
			<div class="flex items-center gap-3">
				<div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
					<Server size={24} class="text-blue-600 dark:text-blue-400" />
				</div>
				<div>
					<h2 class="text-xl font-semibold text-gray-900 dark:text-gray-100">
						Sampling Request
					</h2>
					<p class="text-sm text-gray-600 dark:text-gray-400 mt-0.5">
						from <span class="font-medium">{request.serverName}</span>
					</p>
				</div>
			</div>
			<div class="flex items-center gap-3">
				{#if request.protocolMessageId}
					<button
						onclick={() => {
							uiStore.jumpToProtocolInspector(request.protocolMessageId);
							onClose();
						}}
						class="px-3 py-1.5 text-sm font-medium text-blue-700 dark:text-blue-300 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-lg hover:bg-blue-100 dark:hover:bg-blue-900/50 transition-colors flex items-center gap-2"
						title="View in Protocol Inspector"
					>
						<ExternalLink size={16} />
						<span>View Protocol</span>
					</button>
				{/if}
				<button
					onclick={onClose}
					class="text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 transition-colors"
					aria-label="Close"
				>
					<X size={24} />
				</button>
			</div>
		</div>

		<!-- Content -->
		<div class="flex-1 overflow-y-auto p-6 space-y-6">
			<!-- Quick Response Templates (Testing Tool Feature) -->
			<QuickResponseTemplates onRespond={handleQuickResponse} showHistory={true} />

			<!-- Warning Banner -->
			<div class="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-4 flex items-start gap-3">
				<AlertTriangle size={20} class="text-yellow-600 dark:text-yellow-400 flex-shrink-0 mt-0.5" />
				<div class="flex-1">
					<h3 class="text-sm font-semibold text-yellow-900 dark:text-yellow-100 mb-1">
						Human-in-the-Loop Approval Required
					</h3>
					<p class="text-sm text-yellow-800 dark:text-yellow-200">
						This server is requesting LLM inference. Review the request carefully before approving.
						Your configured LLM provider will be charged.
					</p>
				</div>
			</div>

			<!-- Estimates -->
			<div class="grid grid-cols-2 gap-4">
				<div class="bg-gradient-to-br from-blue-50 to-blue-100 dark:from-blue-900/20 dark:to-blue-900/10 rounded-lg p-4 border border-blue-200 dark:border-blue-800">
					<div class="flex items-center gap-2 mb-2">
						<Hash size={16} class="text-blue-600 dark:text-blue-400" />
						<div class="text-xs font-medium text-blue-600 dark:text-blue-400 uppercase tracking-wide">
							Estimated Tokens
						</div>
					</div>
					<div class="text-3xl font-bold text-blue-900 dark:text-blue-100">
						{request.estimatedTokens?.toLocaleString() || '—'}
					</div>
				</div>
				<div class="bg-gradient-to-br from-green-50 to-green-100 dark:from-green-900/20 dark:to-green-900/10 rounded-lg p-4 border border-green-200 dark:border-green-800">
					<div class="flex items-center gap-2 mb-2">
						<DollarSign size={16} class="text-green-600 dark:text-green-400" />
						<div class="text-xs font-medium text-green-600 dark:text-green-400 uppercase tracking-wide">
							Estimated Cost
						</div>
					</div>
					<div class="text-3xl font-bold text-green-900 dark:text-green-100">
						${request.estimatedCost?.toFixed(4) || '—'}
					</div>
				</div>
			</div>

			<!-- Messages -->
			<div>
				<h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
					<span>Messages</span>
					<span class="text-xs font-normal text-gray-500 dark:text-gray-400">
						({request.request.messages.length})
					</span>
				</h3>
				<div class="space-y-3">
					{#each request.request.messages as message, i}
						<div
							class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4 border border-gray-200 dark:border-gray-700"
						>
							<div class="flex items-center gap-2 mb-2">
								<span
									class="px-2 py-0.5 rounded text-xs font-semibold uppercase tracking-wide {message.role ===
									'user'
										? 'bg-blue-100 text-blue-700 dark:bg-blue-900/50 dark:text-blue-300'
										: 'bg-purple-100 text-purple-700 dark:bg-purple-900/50 dark:text-purple-300'}"
								>
									{message.role}
								</span>
								<span class="text-xs text-gray-500 dark:text-gray-400">Message {i + 1}</span>
							</div>
							{#if message.content.type === 'text' && message.content.text}
								<div class="text-sm text-gray-900 dark:text-gray-100 whitespace-pre-wrap">
									{message.content.text}
								</div>
							{:else}
								<div class="text-xs text-gray-500 dark:text-gray-400 mb-2">
									Content Type: {message.content.type}
									{#if message.content.mimeType}
										<span class="ml-2">({message.content.mimeType})</span>
									{/if}
								</div>
								<JsonViewer data={message.content} expanded={false} maxHeight="200px" />
							{/if}
						</div>
					{/each}
				</div>
			</div>

			<!-- System Prompt (if present) -->
			{#if hasSystemPrompt}
				<div>
					<h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
						System Prompt
					</h3>
					<div
						class="bg-purple-50 dark:bg-purple-900/20 rounded-lg p-4 border border-purple-200 dark:border-purple-800"
					>
						<p class="text-sm text-gray-900 dark:text-gray-100 whitespace-pre-wrap">
							{request.request.systemPrompt}
						</p>
					</div>
				</div>
			{/if}

			<!-- Model Preferences -->
			{#if hasModelPreferences}
				<div>
					<h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
						Model Preferences
					</h3>
					<div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4 border border-gray-200 dark:border-gray-700">
						<JsonViewer
							data={request.request.modelPreferences}
							expanded={true}
							maxHeight="300px"
						/>
					</div>
				</div>
			{/if}

			<!-- Request Parameters -->
			<div>
				<h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
					Request Parameters
				</h3>
				<div class="grid grid-cols-2 gap-3">
					<div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
						<div class="text-xs text-gray-500 dark:text-gray-400 mb-1">Include Context</div>
						<div class="text-sm font-medium text-gray-900 dark:text-gray-100">
							{includeContextLabel}
						</div>
					</div>
					<div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
						<div class="text-xs text-gray-500 dark:text-gray-400 mb-1">Temperature</div>
						<div class="text-sm font-medium text-gray-900 dark:text-gray-100">
							{request.request.temperature ?? 'Default'}
						</div>
					</div>
					<div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
						<div class="text-xs text-gray-500 dark:text-gray-400 mb-1">Max Tokens</div>
						<div class="text-sm font-medium text-gray-900 dark:text-gray-100">
							{request.request.maxTokens ?? 'Default'}
						</div>
					</div>
					<div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
						<div class="text-xs text-gray-500 dark:text-gray-400 mb-1">Stop Sequences</div>
						<div class="text-sm font-medium text-gray-900 dark:text-gray-100">
							{request.request.stopSequences?.length || 'None'}
						</div>
					</div>
				</div>
			</div>

			<!-- Request Metadata -->
			<div class="text-xs text-gray-500 dark:text-gray-400">
				<div>Request ID: <span class="font-mono">{request.requestId}</span></div>
				<div class="mt-1">Created: {new Date(request.createdAt).toLocaleString()}</div>
			</div>
		</div>

		<!-- Actions Footer -->
		<div
			class="p-6 border-t border-gray-200 dark:border-gray-700 flex items-center justify-between flex-shrink-0 bg-gray-50 dark:bg-gray-900"
		>
			<button
				onclick={toggleEdit}
				class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors flex items-center gap-2"
				disabled={isProcessing}
			>
				<Edit size={16} />
				{isEditing ? 'Cancel Edit' : 'Edit Request'}
			</button>

			<div class="flex gap-3">
				<button
					onclick={() => (showRejectDialog = true)}
					class="px-5 py-2 text-sm font-medium text-red-700 dark:text-red-300 bg-white dark:bg-gray-800 border border-red-300 dark:border-red-700 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors flex items-center gap-2"
					disabled={isProcessing}
				>
					<XCircle size={16} />
					Reject
				</button>
				<button
					onclick={handleApprove}
					class="px-6 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600 rounded-lg transition-colors flex items-center gap-2 shadow-sm"
					disabled={isProcessing}
				>
					<Check size={16} />
					{isProcessing ? 'Processing...' : 'Approve & Send to LLM'}
				</button>
			</div>
		</div>
	</div>
</div>

<!-- Reject Dialog -->
{#if showRejectDialog}
	<div class="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-[60] p-4">
		<div class="bg-white dark:bg-gray-800 rounded-lg shadow-2xl max-w-md w-full p-6">
			<h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
				Reject Sampling Request
			</h3>
			<p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
				Please provide a reason for rejecting this request. This will be logged for debugging.
			</p>
			<textarea
				bind:value={rejectReason}
				placeholder="e.g., Request violates policy, cost too high, etc."
				class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-red-500 dark:focus:ring-red-400 transition-all resize-none"
				rows="3"
			></textarea>
			<div class="flex gap-3 mt-6">
				<button
					onclick={() => {
						showRejectDialog = false;
						rejectReason = '';
					}}
					class="flex-1 px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
				>
					Cancel
				</button>
				<button
					onclick={handleReject}
					class="flex-1 px-4 py-2 text-sm font-medium text-white bg-red-600 hover:bg-red-700 dark:bg-red-500 dark:hover:bg-red-600 rounded-lg transition-colors"
					disabled={!rejectReason.trim() || isProcessing}
				>
					{isProcessing ? 'Rejecting...' : 'Confirm Rejection'}
				</button>
			</div>
		</div>
	</div>
{/if}
