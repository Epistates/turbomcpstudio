<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { onMount } from 'svelte';
	import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
	import { Search, Download, Trash2, Filter, ChevronRight, ChevronDown, Clock, ArrowUpDown, ArrowUp, ArrowDown } from 'lucide-svelte';
	import JsonViewer from './ui/JsonViewer.svelte';

	interface MessageHistory {
		id: string;
		server_id: string;
		timestamp: string;
		direction: 'ClientToServer' | 'ServerToClient';
		content: string;
		size_bytes: number;
		processing_time_ms: number | null;
	}

	// Internal state - subscribe to serverStore like ToolExplorer
	let servers: ServerInfo[] = $state([]);
	let selectedServerId: string | undefined = $state(undefined);
	let messages = $state<MessageHistory[]>([]);
	let filter = $state<'all' | 'ClientToServer' | 'ServerToClient'>('all');
	let searchTerm = $state('');
	let expandedMessages = $state<Set<string>>(new Set());
	let loading = $state(false);
	let error = $state<string | null>(null);
	let sortBy = $state<'timestamp' | 'size' | 'latency' | 'direction'>('timestamp');
	let sortOrder = $state<'asc' | 'desc'>('desc');


	// Subscribe to serverStore to get selected server
	$effect(() => {
		const unsubscribe = serverStore.subscribe((state: any) => {
			servers = state.servers;
			const globalSelectedId = state.selectedServerId;

			// Sync with global selection
			if (globalSelectedId && selectedServerId !== globalSelectedId) {
				selectedServerId = globalSelectedId;
			}
			// Auto-select first connected server if none selected
			else if (!selectedServerId && state.servers.length > 0) {
				const connected = state.servers.filter((s: any) => s.status === 'connected');
				selectedServerId = connected.length > 0 ? connected[0].id : state.servers[0].id;
			}
		});

		return unsubscribe;
	});

	const selectedServer = $derived(
		servers.find(s => s.id === selectedServerId)
	);

	const serverName = $derived(
		selectedServer?.config.name || 'No Server Selected'
	);

	// Reload messages when selected server changes
	$effect(() => {
		if (selectedServerId) {
			loadMessages();
		}
	});

	async function loadMessages() {
		if (!selectedServerId) {
			messages = [];
			return;
		}

		loading = true;
		error = null;
		try {
			const result = await invoke<MessageHistory[]>('get_message_history', {
				serverId: selectedServerId,
				limit: 100,
				offset: 0
			});
			messages = result;
		} catch (e) {
			error = String(e);
			console.error('Failed to load message history:', e);
		} finally {
			loading = false;
		}
	}

	async function clearHistory() {
		if (!selectedServerId) return;
		if (!confirm('Clear all message history for this server?')) return;

		try {
			await invoke('clear_message_history', { serverId: selectedServerId });
			messages = [];
		} catch (e) {
			error = String(e);
			console.error('Failed to clear history:', e);
		}
	}

	async function exportMessages() {
		const data = JSON.stringify(filteredMessages, null, 2);
		const blob = new Blob([data], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `mcp-messages-${serverName}-${new Date().toISOString()}.json`;
		a.click();
		URL.revokeObjectURL(url);
	}

	function toggleMessage(id: string) {
		if (expandedMessages.has(id)) {
			expandedMessages.delete(id);
		} else {
			expandedMessages.add(id);
		}
		expandedMessages = new Set(expandedMessages);
	}

	function formatTimestamp(timestamp: string): string {
		const date = new Date(timestamp);
		return date.toLocaleTimeString('en-US', {
			hour12: false,
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit',
			fractionalSecondDigits: 3
		});
	}

	function formatLatency(ms: number | null): string {
		if (ms === null) return '-';
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}

	function parseMessageContent(jsonString: string): any {
		try {
			return JSON.parse(jsonString);
		} catch {
			return { error: 'Invalid JSON', raw: jsonString };
		}
	}

	const filteredMessages = $derived(
		messages
			.filter((m) => {
				if (filter !== 'all' && m.direction !== filter) return false;
				if (searchTerm && !m.content.toLowerCase().includes(searchTerm.toLowerCase()))
					return false;
				return true;
			})
			.sort((a, b) => {
				let comparison = 0;

				switch (sortBy) {
					case 'timestamp':
						comparison = new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
						break;
					case 'size':
						comparison = a.size_bytes - b.size_bytes;
						break;
					case 'latency':
						const aLatency = a.processing_time_ms ?? -1;
						const bLatency = b.processing_time_ms ?? -1;
						comparison = aLatency - bLatency;
						break;
					case 'direction':
						comparison = a.direction.localeCompare(b.direction);
						break;
				}

				return sortOrder === 'asc' ? comparison : -comparison;
			})
	);

	const stats = $derived({
		total: messages.length,
		requests: messages.filter((m) => m.direction === 'ClientToServer').length,
		responses: messages.filter((m) => m.direction === 'ServerToClient').length,
		avgLatency:
			messages
				.filter((m) => m.processing_time_ms !== null)
				.reduce((sum, m) => sum + (m.processing_time_ms || 0), 0) /
			messages.filter((m) => m.processing_time_ms !== null).length
	});

	onMount(() => {
		// Initial load
		loadMessages();

		// Set up real-time event listener for protocol messages
		let unlisten: (() => void) | undefined;

		listen('protocol_message', async () => {
			// Reload messages when new protocol message is captured
			await loadMessages();
		}).then((fn) => {
			unlisten = fn;
		});

		// Return cleanup function
		return () => {
			if (unlisten) unlisten();
		};
	});
</script>

<div class="flex flex-col h-full bg-white dark:bg-gray-900">
	<!-- Header with Stats -->
	<div class="flex-shrink-0 bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-gray-800 dark:to-gray-800 border-b border-gray-200 dark:border-gray-700 p-6">
		<div class="flex items-center justify-between mb-4">
			<div>
				<h2 class="text-xl font-semibold text-gray-900 dark:text-gray-100">Protocol Inspector</h2>
				<p class="text-sm text-gray-600 dark:text-gray-400 mt-1">{serverName}</p>
			</div>
		</div>

		<!-- Stats Grid -->
		<div class="grid grid-cols-4 gap-4">
			<div class="bg-white dark:bg-gray-800 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
				<div class="text-xs uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1">Total</div>
				<div class="text-2xl font-bold text-gray-900 dark:text-gray-100">{stats.total}</div>
			</div>
			<div class="bg-white dark:bg-gray-800 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
				<div class="text-xs uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1">Requests</div>
				<div class="text-2xl font-bold text-blue-600 dark:text-blue-400">{stats.requests}</div>
			</div>
			<div class="bg-white dark:bg-gray-800 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
				<div class="text-xs uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1">Responses</div>
				<div class="text-2xl font-bold text-green-600 dark:text-green-400">{stats.responses}</div>
			</div>
			<div class="bg-white dark:bg-gray-800 rounded-lg p-3 border border-gray-200 dark:border-gray-700">
				<div class="text-xs uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1">Avg Latency</div>
				<div class="text-2xl font-bold text-purple-600 dark:text-purple-400">{formatLatency(stats.avgLatency || null)}</div>
			</div>
		</div>
	</div>

	<!-- Controls Bar -->
	<div class="flex-shrink-0 flex items-center gap-3 p-4 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800">
		<!-- Search Box -->
		<div class="flex-1 relative">
			<Search size={16} class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 dark:text-gray-500" />
			<input
				type="text"
				bind:value={searchTerm}
				placeholder="Search messages (JSON content)..."
				class="w-full pl-10 pr-4 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-sm text-gray-900 dark:text-gray-100 placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400 transition-all"
			/>
		</div>

		<!-- Filter Buttons -->
		<div class="flex items-center gap-2">
			<button
				onclick={() => (filter = 'all')}
				class={`px-3 py-2 text-sm font-medium rounded-lg transition-all ${
					filter === 'all'
						? 'bg-blue-600 text-white shadow-sm'
						: 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600'
				}`}
			>
				All
			</button>
			<button
				onclick={() => (filter = 'ClientToServer')}
				class={`px-3 py-2 text-sm font-medium rounded-lg transition-all ${
					filter === 'ClientToServer'
						? 'bg-blue-600 text-white shadow-sm'
						: 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600'
				}`}
			>
				Requests
			</button>
			<button
				onclick={() => (filter = 'ServerToClient')}
				class={`px-3 py-2 text-sm font-medium rounded-lg transition-all ${
					filter === 'ServerToClient'
						? 'bg-green-600 text-white shadow-sm'
						: 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600'
				}`}
			>
				Responses
			</button>
		</div>

		<!-- Sort Controls -->
		<div class="flex items-center gap-2 border-l border-gray-300 dark:border-gray-600 pl-3">
			<select
				bind:value={sortBy}
				class="px-3 py-2 text-sm bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400 transition-all"
			>
				<option value="timestamp">Time</option>
				<option value="size">Size</option>
				<option value="latency">Latency</option>
				<option value="direction">Direction</option>
			</select>
			<button
				onclick={() => (sortOrder = sortOrder === 'asc' ? 'desc' : 'asc')}
				class="p-2 text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-all"
				title={sortOrder === 'asc' ? 'Ascending' : 'Descending'}
			>
				{#if sortOrder === 'asc'}
					<ArrowUp size={16} />
				{:else}
					<ArrowDown size={16} />
				{/if}
			</button>
		</div>

		<!-- Action Buttons -->
		<div class="flex items-center gap-2 border-l border-gray-300 dark:border-gray-600 pl-3">
			<button
				onclick={exportMessages}
				title="Export messages"
				class="p-2 text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-all"
			>
				<Download size={16} />
			</button>
			<button
				onclick={clearHistory}
				title="Clear history"
				class="p-2 text-gray-600 dark:text-gray-400 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-all"
			>
				<Trash2 size={16} />
			</button>
		</div>
	</div>

	<!-- Error Banner -->
	{#if error}
		<div class="flex-shrink-0 bg-red-50 dark:bg-red-900/20 border-b border-red-200 dark:border-red-800 px-4 py-3">
			<p class="text-sm text-red-800 dark:text-red-300">Error loading messages: {error}</p>
		</div>
	{/if}

	<!-- No Server Selected State -->
	{#if !selectedServerId}
		<div class="flex-1 flex items-center justify-center">
			<div class="text-center px-4">
				<Filter size={48} class="mx-auto text-gray-400 dark:text-gray-500 mb-4" />
				<p class="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">No server selected</p>
				<p class="text-sm text-gray-600 dark:text-gray-400">
					Select a server from the sidebar to view protocol messages
				</p>
			</div>
		</div>

	<!-- Loading State -->
	{:else if loading && messages.length === 0}
		<div class="flex-1 flex items-center justify-center">
			<div class="text-center">
				<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 dark:border-blue-400 mx-auto mb-4"></div>
				<p class="text-sm text-gray-600 dark:text-gray-400">Loading message history...</p>
			</div>
		</div>

	<!-- Empty State -->
	{:else if !loading && filteredMessages.length === 0}
		<div class="flex-1 flex items-center justify-center">
			<div class="text-center px-4">
				<Filter size={48} class="mx-auto text-gray-400 dark:text-gray-500 mb-4" />
				<p class="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">No messages to display</p>
				<p class="text-sm text-gray-600 dark:text-gray-400">
					{#if searchTerm}
						Try adjusting your search filter
					{:else if filter !== 'all'}
						Try changing the message direction filter
					{:else}
						Messages will appear here as you interact with the server
					{/if}
				</p>
			</div>
		</div>
	{/if}

	<!-- Message List -->
	{#if selectedServerId}
		<div class="flex-1 overflow-y-auto p-4 space-y-2">
			{#each filteredMessages as message (message.id)}
				{@const isExpanded = expandedMessages.has(message.id)}
				{@const isRequest = message.direction === 'ClientToServer'}

				<div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden transition-all hover:shadow-md {isRequest ? 'border-l-4 border-l-blue-500 dark:border-l-blue-400' : 'border-l-4 border-l-green-500 dark:border-l-green-400'}">
					<button
						onclick={() => toggleMessage(message.id)}
						class="w-full flex items-center justify-between p-4 text-left hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
					>
						<div class="flex items-center gap-3 flex-1 min-w-0">
							<div class="flex-shrink-0 text-gray-400 dark:text-gray-500">
								{#if isExpanded}
									<ChevronDown size={16} />
								{:else}
									<ChevronRight size={16} />
								{/if}
							</div>
							<span class={`px-2 py-1 rounded-md text-xs font-semibold uppercase tracking-wide ${
								isRequest
									? 'bg-blue-100 dark:bg-blue-900/50 text-blue-700 dark:text-blue-300'
									: 'bg-green-100 dark:bg-green-900/50 text-green-700 dark:text-green-300'
							}`}>
								{isRequest ? '→ Request' : '← Response'}
							</span>
							<span class="text-xs font-mono text-gray-600 dark:text-gray-400">{formatTimestamp(message.timestamp)}</span>
						</div>
						<div class="flex items-center gap-4 flex-shrink-0">
							{#if message.processing_time_ms !== null}
								<div class="flex items-center gap-1.5 text-xs font-mono text-gray-600 dark:text-gray-400">
									<Clock size={12} />
									<span>{formatLatency(message.processing_time_ms)}</span>
								</div>
							{/if}
							<span class="text-xs font-mono text-gray-500 dark:text-gray-500">{(message.size_bytes / 1024).toFixed(2)} KB</span>
						</div>
					</button>

					{#if isExpanded}
						<div class="border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50">
							<JsonViewer
								data={parseMessageContent(message.content)}
								expanded={true}
								showCopy={true}
								maxHeight="500px"
							/>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

