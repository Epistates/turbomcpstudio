<!--
  Sampling Approval Modal

  Displays server-initiated sampling requests for human-in-the-loop approval.
  Shows request details, estimated cost/tokens, and allows approve/reject/edit.
-->
<script lang="ts">
	import { samplingStore, type PendingSamplingRequest } from '$lib/stores/samplingStore';
	import { uiStore } from '$lib/stores/uiStore';
	import JsonViewer from './ui/JsonViewer.svelte';
	import { X, AlertTriangle, Server, ExternalLink, Edit, Save, Check } from 'lucide-svelte';

	// Props
	const { request, onClose }: { request: PendingSamplingRequest; onClose: () => void } = $props();

	// State
	let isEditing = $state(false);
	let modifiedRequest = $state({ ...request.request });
	let isProcessing = $state(false);

	// Response composition state
	let responseText = $state('');
	let responseModel = $state('manual-response');
	let responseStopReason = $state<'endTurn' | 'stopSequence' | 'maxTokens'>('endTurn');
	let showAdvanced = $state(false);

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
	function toggleEdit() {
		isEditing = !isEditing;
		if (!isEditing) {
			// Reset modifications on cancel
			modifiedRequest = { ...request.request };
		}
	}

	function saveEdits() {
		// Validate and save edits
		isEditing = false;
		// modifiedRequest is now ready to be used in approval
	}

	// Populate response textarea with template
	function useTemplate(text: string, model: string) {
		responseText = text;
		responseModel = model;
	}

	// Send the composed response
	async function handleSendResponse() {
		if (!responseText.trim()) {
			alert('Please enter a response text');
			return;
		}

		if (isProcessing) return;
		isProcessing = true;

		try {
			const response = {
				role: 'assistant' as const,
				content: {
					type: 'text' as const,
					text: responseText
				},
				model: responseModel,
				stopReason: responseStopReason
			};

			await samplingStore.submitManual(request.requestId, response);
			onClose();
		} catch (error) {
			alert('Failed to submit manual response: ' + error);
		} finally {
			isProcessing = false;
		}
	}

	// Reject the sampling request with a reason (sends JSON-RPC error)
	async function handleReject(reason: string) {
		if (isProcessing) return;
		isProcessing = true;

		try {
			await samplingStore.reject(request.requestId, reason);
			onClose();
		} catch (error) {
			alert('Failed to reject request: ' + error);
		} finally {
			isProcessing = false;
		}
	}

	// Update message content
	function updateMessageContent(index: number, newText: string) {
		const messages = [...modifiedRequest.messages];
		if (messages[index] && messages[index].content.type === 'text') {
			messages[index] = {
				...messages[index],
				content: {
					...messages[index].content,
					text: newText
				}
			};
			modifiedRequest = { ...modifiedRequest, messages };
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
					<div class="flex items-center gap-2">
						<h2 class="text-xl font-semibold text-gray-900 dark:text-gray-100">
							Sampling Request
						</h2>
						{#if request.retryCount && request.retryCount > 1}
							<span class="px-2 py-0.5 text-xs font-semibold bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300 rounded-full border border-orange-300 dark:border-orange-700">
								🔄 Retry #{request.retryCount}
							</span>
						{/if}
					</div>
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
			<!-- Info Banner -->
			<div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4 flex items-start gap-3">
				<AlertTriangle size={20} class="text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
				<div class="flex-1">
					<h3 class="text-sm font-semibold text-blue-900 dark:text-blue-100 mb-1">
						You Are Acting As The LLM
					</h3>
					<p class="text-sm text-blue-800 dark:text-blue-200">
						Compose the response the LLM would provide. Use quick templates below or write your own custom response.
					</p>
				</div>
			</div>

			<!-- Response Composition Section -->
			<div class="bg-gradient-to-br from-purple-50 to-blue-50 dark:from-purple-900/20 dark:to-blue-900/20 rounded-lg border-2 border-purple-200 dark:border-purple-800 p-4">
				<div class="flex items-center justify-between mb-3">
					<h3 class="text-sm font-semibold text-purple-900 dark:text-purple-100">
						💬 Compose LLM Response
					</h3>
					<label class="flex items-center gap-2 text-xs font-medium text-purple-700 dark:text-purple-300 cursor-pointer hover:text-purple-900 dark:hover:text-purple-100 transition-colors">
						<input
							type="checkbox"
							bind:checked={showAdvanced}
							class="w-3.5 h-3.5 text-purple-600 bg-white dark:bg-gray-800 border-purple-300 dark:border-purple-600 rounded focus:ring-2 focus:ring-purple-500"
						/>
						<span>Advanced Options</span>
					</label>
				</div>

				<!-- Quick Response Templates (for manual LLM responses) -->
				<div class="mb-3">
					<p class="text-xs text-purple-700 dark:text-purple-300 mb-2">Response Templates (act as LLM):</p>
					<div class="flex flex-wrap gap-2">
						<button
							onclick={() => useTemplate('Yes, I approve this action. Please proceed.', 'manual-approval')}
							class="px-3 py-1 text-xs font-medium text-green-700 dark:text-green-300 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded hover:bg-green-100 dark:hover:bg-green-900/30 transition-colors"
						>
							✅ Approve
						</button>
						<button
							onclick={() => useTemplate('No, I cannot approve this action.', 'manual-response')}
							class="px-3 py-1 text-xs font-medium text-orange-700 dark:text-orange-300 bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded hover:bg-orange-100 dark:hover:bg-orange-900/30 transition-colors"
						>
							🚫 Deny (as LLM)
						</button>
					</div>
				</div>

				<!-- Quick Reject Reasons (reject entire request with error) -->
				<div class="mb-3">
					<p class="text-xs text-red-700 dark:text-red-300 mb-2">Reject Request (sends error):</p>
					<div class="flex flex-wrap gap-2">
						<button
							onclick={() => handleReject('User rejected sampling request')}
							class="px-3 py-1 text-xs font-medium text-red-700 dark:text-red-300 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded hover:bg-red-100 dark:hover:bg-red-900/30 transition-colors"
							disabled={isProcessing}
						>
							❌ User Rejected
						</button>
						<button
							onclick={() => handleReject('Request timeout - operation took too long')}
							class="px-3 py-1 text-xs font-medium text-yellow-700 dark:text-yellow-300 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded hover:bg-yellow-100 dark:hover:bg-yellow-900/30 transition-colors"
							disabled={isProcessing}
						>
							⏰ Timeout
						</button>
						<button
							onclick={() => handleReject('Permission denied - insufficient privileges')}
							class="px-3 py-1 text-xs font-medium text-red-700 dark:text-red-300 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded hover:bg-red-100 dark:hover:bg-red-900/30 transition-colors"
							disabled={isProcessing}
						>
							⚠️ Permission Denied
						</button>
						<button
							onclick={() => handleReject('Resource not available')}
							class="px-3 py-1 text-xs font-medium text-gray-700 dark:text-gray-300 bg-gray-50 dark:bg-gray-900/20 border border-gray-200 dark:border-gray-700 rounded hover:bg-gray-100 dark:hover:bg-gray-900/30 transition-colors"
							disabled={isProcessing}
						>
							🔒 Resource Unavailable
						</button>
					</div>
				</div>

				<!-- Response Text -->
				<div class="mb-3">
					<label for="responseText" class="block text-xs font-medium text-purple-700 dark:text-purple-300 mb-1">
						Response Text:
					</label>
					<textarea
						id="responseText"
						bind:value={responseText}
						class="w-full px-3 py-2 text-sm bg-white dark:bg-gray-800 border-2 border-purple-300 dark:border-purple-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-purple-500 resize-none"
						rows="4"
						placeholder="Enter the response the LLM would provide..."
					></textarea>
				</div>

				<!-- Advanced: Response Metadata -->
				{#if showAdvanced}
					<div class="grid grid-cols-2 gap-3">
						<div>
							<label for="responseModel" class="block text-xs font-medium text-purple-700 dark:text-purple-300 mb-1">
								Model:
							</label>
							<input
								id="responseModel"
								type="text"
								bind:value={responseModel}
								class="w-full px-2 py-1 text-xs bg-white dark:bg-gray-800 border border-purple-300 dark:border-purple-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-purple-500"
								placeholder="manual-response"
							/>
						</div>
						<div>
							<label for="responseStopReason" class="block text-xs font-medium text-purple-700 dark:text-purple-300 mb-1">
								Stop Reason:
							</label>
							<select
								id="responseStopReason"
								bind:value={responseStopReason}
								class="w-full px-2 py-1 text-xs bg-white dark:bg-gray-800 border border-purple-300 dark:border-purple-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-purple-500"
							>
								<option value="endTurn">End Turn</option>
								<option value="stopSequence">Stop Sequence</option>
								<option value="maxTokens">Max Tokens</option>
							</select>
						</div>
					</div>
				{/if}
			</div>

			<!-- Messages -->
			<div>
				<h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
					<span>Messages</span>
					<span class="text-xs font-normal text-gray-500 dark:text-gray-400">
						({modifiedRequest.messages.length})
					</span>
					{#if isEditing}
						<span class="text-xs font-medium text-blue-600 dark:text-blue-400">
							(Editing Mode)
						</span>
					{/if}
				</h3>
				<div class="space-y-3">
					{#each modifiedRequest.messages as message, i}
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
							{#if message.content.type === 'text' && message.content.text !== undefined}
								{#if isEditing}
									<textarea
										value={message.content.text}
										oninput={(e) => updateMessageContent(i, e.currentTarget.value)}
										class="w-full px-3 py-2 text-sm bg-white dark:bg-gray-800 border border-blue-300 dark:border-blue-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
										rows="4"
										placeholder="Enter message text..."
									></textarea>
								{:else}
									<div class="text-sm text-gray-900 dark:text-gray-100 whitespace-pre-wrap">
										{message.content.text}
									</div>
								{/if}
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

			<!-- System Prompt (if present or in edit mode) -->
			{#if hasSystemPrompt || isEditing}
				<div>
					<h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
						System Prompt {isEditing ? '(Optional)' : ''}
					</h3>
					<div
						class="bg-purple-50 dark:bg-purple-900/20 rounded-lg p-4 border border-purple-200 dark:border-purple-800"
					>
						{#if isEditing}
							<textarea
								bind:value={modifiedRequest.systemPrompt}
								class="w-full px-3 py-2 text-sm bg-white dark:bg-gray-800 border border-purple-300 dark:border-purple-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-purple-500 resize-none"
								rows="3"
								placeholder="Enter system prompt (optional)..."
							></textarea>
						{:else}
							<p class="text-sm text-gray-900 dark:text-gray-100 whitespace-pre-wrap">
								{modifiedRequest.systemPrompt || '(None)'}
							</p>
						{/if}
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

			<!-- Request Parameters (Advanced Only) -->
			{#if showAdvanced}
				<div>
					<h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
						Request Parameters
					</h3>
					<div class="grid grid-cols-2 gap-3">
					<div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
						<div class="text-xs text-gray-500 dark:text-gray-400 mb-1">Include Context</div>
						{#if isEditing}
							<select
								bind:value={modifiedRequest.includeContext}
								class="w-full px-2 py-1 text-sm bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
							>
								<option value="none">None (messages only)</option>
								<option value="thisServer">This Server Only</option>
								<option value="allServers">All Connected Servers</option>
							</select>
						{:else}
							<div class="text-sm font-medium text-gray-900 dark:text-gray-100">
								{includeContextLabel}
							</div>
						{/if}
					</div>
					<div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
						<div class="text-xs text-gray-500 dark:text-gray-400 mb-1">Temperature</div>
						{#if isEditing}
							<input
								type="number"
								bind:value={modifiedRequest.temperature}
								min="0"
								max="2"
								step="0.1"
								class="w-full px-2 py-1 text-sm bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
								placeholder="Default"
							/>
						{:else}
							<div class="text-sm font-medium text-gray-900 dark:text-gray-100">
								{modifiedRequest.temperature ?? 'Default'}
							</div>
						{/if}
					</div>
					<div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
						<div class="text-xs text-gray-500 dark:text-gray-400 mb-1">Max Tokens</div>
						{#if isEditing}
							<input
								type="number"
								bind:value={modifiedRequest.maxTokens}
								min="1"
								step="1"
								class="w-full px-2 py-1 text-sm bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
								placeholder="Default"
							/>
						{:else}
							<div class="text-sm font-medium text-gray-900 dark:text-gray-100">
								{modifiedRequest.maxTokens ?? 'Default'}
							</div>
						{/if}
					</div>
					<div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
						<div class="text-xs text-gray-500 dark:text-gray-400 mb-1">Stop Sequences</div>
						<div class="text-sm font-medium text-gray-900 dark:text-gray-100">
							{modifiedRequest.stopSequences?.length || 'None'}
						</div>
					</div>
				</div>
				</div>
			{/if}

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
			<!-- Left side: Edit/Save controls -->
			<div>
				{#if isEditing}
					<div class="flex gap-2">
						<button
							onclick={saveEdits}
							class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600 rounded-lg transition-colors flex items-center gap-2"
						>
							<Save size={16} />
							Save Changes
						</button>
						<button
							onclick={toggleEdit}
							class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
						>
							Cancel
						</button>
					</div>
				{:else}
					<button
						onclick={toggleEdit}
						class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors flex items-center gap-2"
						disabled={isProcessing}
					>
						<Edit size={16} />
						Edit Request
					</button>
				{/if}
			</div>

			<!-- Right side: Send Response Button -->
			{#if !isEditing}
				<button
					onclick={handleSendResponse}
					class="px-6 py-2.5 text-sm font-medium text-white bg-purple-600 hover:bg-purple-700 dark:bg-purple-500 dark:hover:bg-purple-600 rounded-lg transition-colors flex items-center gap-2 shadow-sm disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-purple-600 dark:disabled:hover:bg-purple-500"
					disabled={isProcessing || !responseText.trim()}
					title={!responseText.trim() ? 'Enter a response to enable' : ''}
				>
					{#if isProcessing}
						<div class="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full"></div>
						Sending...
					{:else}
						<Check size={16} />
						Send Response
					{/if}
				</button>
			{/if}
		</div>
	</div>
</div>
