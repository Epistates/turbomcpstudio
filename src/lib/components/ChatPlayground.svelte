<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { Send, Bot, User, AlertCircle, Loader2, Sparkles, DollarSign } from 'lucide-svelte';

	// Type definitions
	interface LLMProviderStatus {
		provider_id: string;
		display_name: string;
		provider_type: string;
		enabled: boolean;
		configured: boolean;
		active: boolean;
		available_models: string[];
		base_url?: string;
		last_error?: string;
	}

	// Message types
	interface Message {
		id: string;
		role: 'user' | 'assistant';
		content: string;
		timestamp: Date;
		provider?: string;
		usage?: {
			input_tokens: number;
			output_tokens: number;
		};
		cost?: number;
	}

	// Reactive state
	let messages = $state<Message[]>([]);
	let input = $state('');
	let providers = $state<LLMProviderStatus[]>([]);
	let selectedProvider = $state<string>('');
	let selectedModel = $state<string>('');
	let activeProvider = $state<string | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let totalCost = $state(0);

	// Refs
	let chatContainer: HTMLDivElement;
	let textarea: HTMLTextAreaElement;

	// Load providers on mount
	onMount(async () => {
		await loadProviders();
		await loadActiveProvider();
	});

	async function loadProviders() {
		try {
			const result = await invoke<LLMProviderStatus[]>('list_llm_providers');
			providers = result;

			// Auto-select first enabled provider
			const firstEnabled = result.find((p) => p.enabled);
			if (firstEnabled && !selectedProvider) {
				selectedProvider = firstEnabled.provider_id;
				// Auto-select first available model for this provider
				if (firstEnabled.available_models.length > 0) {
					selectedModel = firstEnabled.available_models[0];
				}
			}
		} catch (e) {
			error = `Failed to load providers: ${e}`;
		}
	}

	async function loadActiveProvider() {
		try {
			activeProvider = await invoke<string | null>('get_active_llm_provider');
		} catch (e) {
			console.error('Failed to load active provider:', e);
		}
	}

	async function sendMessage() {
		if (!input.trim() || loading) return;

		const userMessage: Message = {
			id: crypto.randomUUID(),
			role: 'user',
			content: input.trim(),
			timestamp: new Date()
		};

		messages = [...messages, userMessage];
		const userInput = input;
		input = '';
		loading = true;
		error = null;

		// Auto-resize textarea
		if (textarea) {
			textarea.style.height = 'auto';
		}

		try {
			const result = await invoke<any>('send_llm_message', {
				request: {
					messages: messages.map((m) => ({
						role: m.role,
						content: { type: 'text', text: m.content }
					})),
					modelPreferences: selectedModel ? { hints: [{ name: selectedModel }] } : undefined,
					maxTokens: 4000
				},
				providerId: selectedProvider || null
			});

			// Extract text content from response
			let assistantContent = '';
			if (result.content && Array.isArray(result.content)) {
				assistantContent = result.content.map((c: any) => c.text || '').join('');
			} else if (typeof result.content === 'string') {
				assistantContent = result.content;
			} else if (result.content?.text) {
				assistantContent = result.content.text;
			}

			const assistantMessage: Message = {
				id: crypto.randomUUID(),
				role: 'assistant',
				content: assistantContent,
				timestamp: new Date(),
				provider: selectedProvider,
				usage: result.usage,
				cost: calculateCost(result.usage, selectedProvider)
			};

			if (assistantMessage.cost) {
				totalCost += assistantMessage.cost;
			}

			messages = [...messages, assistantMessage];

			// Scroll to bottom
			setTimeout(() => {
				if (chatContainer) {
					chatContainer.scrollTop = chatContainer.scrollHeight;
				}
			}, 100);
		} catch (e: any) {
			error = e.toString();
			// Remove user message if failed
			messages = messages.filter((m) => m.id !== userMessage.id);
			// Restore input
			input = userInput;
		} finally {
			loading = false;
		}
	}

	function calculateCost(usage: any, providerId: string): number {
		if (!usage) return 0;

		// Simple cost calculation (approximate rates)
		const rates: Record<
			string,
			{ input: number; output: number }
		> = {
			'claude-3-7-sonnet': { input: 3.0 / 1_000_000, output: 15.0 / 1_000_000 },
			'claude-3-5-sonnet': { input: 3.0 / 1_000_000, output: 15.0 / 1_000_000 },
			'claude-3-opus': { input: 15.0 / 1_000_000, output: 75.0 / 1_000_000 },
			'claude-3-haiku': { input: 0.25 / 1_000_000, output: 1.25 / 1_000_000 },
			openai: { input: 2.5 / 1_000_000, output: 10.0 / 1_000_000 },
			'gemini-2.0-flash-exp': { input: 0.0, output: 0.0 }, // Free
			'gemini-1.5-flash': { input: 0.075 / 1_000_000, output: 0.3 / 1_000_000 },
			'gemini-1.5-pro': { input: 1.25 / 1_000_000, output: 5.0 / 1_000_000 }
		};

		const rate = rates[providerId] || { input: 0, output: 0 };
		return usage.input_tokens * rate.input + usage.output_tokens * rate.output;
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			sendMessage();
		}
	}

	function clearChat() {
		if (confirm('Clear all messages?')) {
			messages = [];
			totalCost = 0;
			error = null;
		}
	}

	function formatCost(cost: number): string {
		if (cost < 0.01) return '<$0.01';
		return `$${cost.toFixed(4)}`;
	}

	// Filter for enabled providers
	const enabledProviders = $derived(providers.filter((p) => p.enabled && p.configured));

	// Get available models for selected provider
	const currentProvider = $derived(providers.find((p) => p.provider_id === selectedProvider));
	const availableModels = $derived(currentProvider?.available_models ?? []);

	// Auto-select first model when provider changes
	$effect(() => {
		if (selectedProvider && availableModels.length > 0 && !availableModels.includes(selectedModel)) {
			selectedModel = availableModels[0];
		}
	});
</script>

<div class="flex h-full flex-col bg-zinc-50 dark:bg-zinc-900">
	<!-- Header -->
	<div
		class="flex items-center justify-between border-b border-zinc-200 bg-white px-6 py-4 dark:border-zinc-800 dark:bg-zinc-950"
	>
		<div class="flex items-center gap-3">
			<Sparkles class="h-6 w-6 text-purple-600 dark:text-purple-400" />
			<h1 class="text-2xl font-bold text-zinc-900 dark:text-zinc-100">LLM Playground</h1>
		</div>

		<div class="flex items-center gap-4">
			<!-- Cost Display -->
			{#if totalCost > 0}
				<div
					class="flex items-center gap-2 rounded-lg bg-green-50 px-3 py-1.5 text-sm dark:bg-green-900/20"
				>
					<DollarSign class="h-4 w-4 text-green-600 dark:text-green-400" />
					<span class="font-medium text-green-700 dark:text-green-300">
						{formatCost(totalCost)}
					</span>
				</div>
			{/if}

			<!-- Provider & Model Selection -->
			<div class="flex items-center gap-2">
				<label for="provider" class="text-sm font-medium text-zinc-700 dark:text-zinc-300">
					Provider:
				</label>
				<select
					id="provider"
					bind:value={selectedProvider}
					class="rounded-lg border border-zinc-300 bg-white px-3 py-1.5 text-sm focus:border-purple-500 focus:ring-2 focus:ring-purple-500/20 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-100"
				>
					{#if enabledProviders.length === 0}
						<option value="">No providers configured</option>
					{:else}
						{#each enabledProviders as provider}
							<option value={provider.provider_id}>
								{provider.display_name}
							</option>
						{/each}
					{/if}
				</select>
			</div>

			<!-- Model Selection -->
			{#if availableModels.length > 0}
				<div class="flex items-center gap-2">
					<label for="model" class="text-sm font-medium text-zinc-700 dark:text-zinc-300">
						Model:
					</label>
					<select
						id="model"
						bind:value={selectedModel}
						class="rounded-lg border border-zinc-300 bg-white px-3 py-1.5 text-sm focus:border-purple-500 focus:ring-2 focus:ring-purple-500/20 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-100"
					>
						{#each availableModels as model}
							<option value={model}>
								{model}
							</option>
						{/each}
					</select>
				</div>
			{/if}

			<!-- Clear Button -->
			{#if messages.length > 0}
				<button
					onclick={clearChat}
					class="rounded-lg border border-zinc-300 px-3 py-1.5 text-sm font-medium text-zinc-700 transition-colors hover:bg-zinc-100 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800"
				>
					Clear
				</button>
			{/if}
		</div>
	</div>

	<!-- Error Banner -->
	{#if error}
		<div class="flex items-center gap-2 bg-red-50 px-6 py-3 text-red-800 dark:bg-red-900/20 dark:text-red-300">
			<AlertCircle class="h-5 w-5 flex-shrink-0" />
			<p class="text-sm">{error}</p>
			<button
				onclick={() => (error = null)}
				class="ml-auto text-sm font-medium hover:underline"
			>
				Dismiss
			</button>
		</div>
	{/if}

	<!-- Chat Messages -->
	<div bind:this={chatContainer} class="flex-1 overflow-y-auto px-6 py-6">
		{#if messages.length === 0}
			<div class="flex h-full items-center justify-center">
				<div class="text-center">
					<Bot class="mx-auto mb-4 h-16 w-16 text-zinc-400" />
					<h2 class="mb-2 text-xl font-semibold text-zinc-700 dark:text-zinc-300">
						Start a conversation
					</h2>
					<p class="text-sm text-zinc-500 dark:text-zinc-400">
						Choose a provider and send a message to begin
					</p>
				</div>
			</div>
		{:else}
			<div class="mx-auto max-w-4xl space-y-6">
				{#each messages as message (message.id)}
					<div
						class="flex gap-4 {message.role === 'user'
							? 'justify-end'
							: 'justify-start'}"
					>
						{#if message.role === 'assistant'}
							<div
								class="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-purple-100 dark:bg-purple-900/30"
							>
								<Bot class="h-5 w-5 text-purple-600 dark:text-purple-400" />
							</div>
						{/if}

						<div class="flex flex-col gap-2 {message.role === 'user' ? 'items-end' : 'items-start'} max-w-[75%]">
							<div
								class="rounded-2xl px-4 py-3 {message.role === 'user'
									? 'bg-purple-600 text-white'
									: 'bg-white text-zinc-900 dark:bg-zinc-800 dark:text-zinc-100'} {message.role === 'assistant' ? 'border border-zinc-200 dark:border-zinc-700' : ''}"
							>
								<p class="whitespace-pre-wrap text-sm leading-relaxed">{message.content}</p>
							</div>

							<!-- Metadata -->
							<div class="flex items-center gap-3 text-xs text-zinc-500 dark:text-zinc-400">
								<span>{message.timestamp.toLocaleTimeString()}</span>
								{#if message.provider}
									<span>•</span>
									<span>{providers.find(p => p.provider_id === message.provider)?.display_name}</span>
								{/if}
								{#if message.usage}
									<span>•</span>
									<span>{message.usage.input_tokens + message.usage.output_tokens} tokens</span>
								{/if}
								{#if message.cost}
									<span>•</span>
									<span class="font-medium text-green-600 dark:text-green-400">
										{formatCost(message.cost)}
									</span>
								{/if}
							</div>
						</div>

						{#if message.role === 'user'}
							<div
								class="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-zinc-200 dark:bg-zinc-700"
							>
								<User class="h-5 w-5 text-zinc-600 dark:text-zinc-300" />
							</div>
						{/if}
					</div>
				{/each}

				{#if loading}
					<div class="flex gap-4">
						<div
							class="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-purple-100 dark:bg-purple-900/30"
						>
							<Bot class="h-5 w-5 text-purple-600 dark:text-purple-400" />
						</div>
						<div
							class="flex items-center gap-2 rounded-2xl border border-zinc-200 bg-white px-4 py-3 dark:border-zinc-700 dark:bg-zinc-800"
						>
							<Loader2 class="h-4 w-4 animate-spin text-purple-600 dark:text-purple-400" />
							<span class="text-sm text-zinc-500 dark:text-zinc-400">Thinking...</span>
						</div>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Input Area -->
	<div class="border-t border-zinc-200 bg-white px-6 py-4 dark:border-zinc-800 dark:bg-zinc-950">
		<div class="mx-auto max-w-4xl">
			<div class="flex gap-3">
				<textarea
					bind:this={textarea}
					bind:value={input}
					onkeydown={handleKeyDown}
					placeholder="Ask anything... (Enter to send, Shift+Enter for new line)"
					disabled={loading || enabledProviders.length === 0}
					class="min-h-[60px] max-h-[200px] flex-1 resize-none rounded-lg border border-zinc-300 bg-zinc-50 px-4 py-3 text-sm placeholder-zinc-400 focus:border-purple-500 focus:bg-white focus:ring-2 focus:ring-purple-500/20 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-100 dark:placeholder-zinc-500 dark:focus:bg-zinc-800"
					rows="2"
				></textarea>

				<button
					onclick={sendMessage}
					disabled={loading || !input.trim() || enabledProviders.length === 0}
					class="flex h-[60px] w-[60px] items-center justify-center rounded-lg bg-purple-600 text-white transition-all hover:bg-purple-700 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-purple-500 dark:hover:bg-purple-600"
					title="Send message (Enter)"
				>
					{#if loading}
						<Loader2 class="h-5 w-5 animate-spin" />
					{:else}
						<Send class="h-5 w-5" />
					{/if}
				</button>
			</div>

			{#if enabledProviders.length === 0}
				<p class="mt-2 text-center text-sm text-red-600 dark:text-red-400">
					No providers configured. Please add an API key in Settings.
				</p>
			{/if}
		</div>
	</div>
</div>
