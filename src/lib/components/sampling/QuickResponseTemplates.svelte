<!--
  Quick Response Templates for HITL Sampling

  Provides one-click responses for common testing scenarios:
  - Approve (proceed)
  - Reject (cancel)
  - Error responses (timeout, auth, permission)
  - Response history (reuse previous)
-->
<script lang="ts">
	import { createLogger } from '$lib/utils/logger';
	import { Check, XCircle, Clock, Lock, AlertCircle, History } from 'lucide-svelte';

	import type { CreateMessageResult } from '$lib/types/sampling';


	// Initialize scoped logger
	const logger = createLogger('QuickResponseTemplates');

	// Props
	const {
		onRespond,
		showHistory = false
	}: {
		onRespond: (response: CreateMessageResult) => void;
		showHistory?: boolean;
	} = $props();

	// Previous responses from localStorage
	let previousResponses = $state<Array<{ text: string; timestamp: string; preview: string }>>([]);
	let showPreviousDropdown = $state(false);

	// Load previous responses on mount
	$effect(() => {
		try {
			const stored = localStorage.getItem('sampling_response_history');
			if (stored) {
				previousResponses = JSON.parse(stored);
			}
		} catch (e) {
			logger.error('Failed to load response history:', e);
		}
	});

	// Quick response templates
	const templates = [
		{
			label: 'Approve',
			icon: Check,
			color: 'green',
			response: {
				role: 'assistant' as const,
				content: {
					type: 'text' as const,
					text: 'Yes, I approve this action. Please proceed.'
				},
				model: 'manual-approval',
				stopReason: 'endTurn' as const
			}
		},
		{
			label: 'Reject',
			icon: XCircle,
			color: 'red',
			response: {
				role: 'assistant' as const,
				content: {
					type: 'text' as const,
					text: 'No, I do not approve this action. Please cancel.'
				},
				model: 'manual-rejection',
				stopReason: 'endTurn' as const
			}
		},
		{
			label: 'Timeout',
			icon: Clock,
			color: 'yellow',
			response: {
				role: 'assistant' as const,
				content: {
					type: 'text' as const,
					text: 'Request timeout - operation took too long to complete.'
				},
				model: 'manual-error',
				stopReason: 'endTurn' as const
			}
		},
		{
			label: 'Auth Required',
			icon: Lock,
			color: 'orange',
			response: {
				role: 'assistant' as const,
				content: {
					type: 'text' as const,
					text: 'Authentication required. Please provide valid credentials.'
				},
				model: 'manual-error',
				stopReason: 'endTurn' as const
			}
		},
		{
			label: 'Permission Denied',
			icon: AlertCircle,
			color: 'red',
			response: {
				role: 'assistant' as const,
				content: {
					type: 'text' as const,
					text: 'Permission denied. You do not have access to perform this action.'
				},
				model: 'manual-error',
				stopReason: 'endTurn' as const
			}
		}
	];

	function handleTemplateClick(response: CreateMessageResult) {
		// Save to history
		saveToHistory(response);
		// Call the response handler
		onRespond(response);
	}

	function saveToHistory(response: CreateMessageResult) {
		try {
			const text = typeof response.content === 'object' && 'text' in response.content && response.content.text
				? response.content.text
				: JSON.stringify(response.content);

			const newEntry = {
				text: text,
				timestamp: new Date().toISOString(),
				preview: text.substring(0, 50) + (text.length > 50 ? '...' : '')
			};

			const updated = [newEntry, ...previousResponses.slice(0, 9)]; // Keep last 10
			previousResponses = updated;
			localStorage.setItem('sampling_response_history', JSON.stringify(updated));
		} catch (e) {
			logger.error('Failed to save response to history:', e);
		}
	}

	function usePreviousResponse(text: string) {
		const response: CreateMessageResult = {
			role: 'assistant',
			content: {
				type: 'text',
				text
			},
			model: 'manual-previous',
			stopReason: 'endTurn'
		};
		onRespond(response);
		showPreviousDropdown = false;
	}

	function getColorClasses(color: string) {
		switch (color) {
			case 'green':
				return 'bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300 border-green-200 dark:border-green-800 hover:bg-green-100 dark:hover:bg-green-900/30';
			case 'red':
				return 'bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300 border-red-200 dark:border-red-800 hover:bg-red-100 dark:hover:bg-red-900/30';
			case 'yellow':
				return 'bg-yellow-50 dark:bg-yellow-900/20 text-yellow-700 dark:text-yellow-300 border-yellow-200 dark:border-yellow-800 hover:bg-yellow-100 dark:hover:bg-yellow-900/30';
			case 'orange':
				return 'bg-orange-50 dark:bg-orange-900/20 text-orange-700 dark:text-orange-300 border-orange-200 dark:border-orange-800 hover:bg-orange-100 dark:hover:bg-orange-900/30';
			default:
				return 'bg-gray-50 dark:bg-gray-900/20 text-gray-700 dark:text-gray-300 border-gray-200 dark:border-gray-800 hover:bg-gray-100 dark:hover:bg-gray-900/30';
		}
	}
</script>

<div class="space-y-4">
	<!-- Quick Response Buttons -->
	<div>
		<h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
			Quick Responses
		</h4>
		<div class="grid grid-cols-2 gap-3">
			{#each templates as template}
				<button
					onclick={() => handleTemplateClick(template.response)}
					class="px-4 py-3 text-sm font-medium border rounded-lg transition-all flex items-center gap-2 {getColorClasses(
						template.color
					)}"
				>
					<svelte:component this={template.icon} size={18} />
					<span>{template.label}</span>
				</button>
			{/each}
		</div>
	</div>

	<!-- Response History -->
	{#if showHistory && previousResponses.length > 0}
		<div class="border-t border-gray-200 dark:border-gray-700 pt-4">
			<button
				onclick={() => (showPreviousDropdown = !showPreviousDropdown)}
				class="flex items-center gap-2 text-sm font-semibold text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-gray-100 transition-colors"
			>
				<History size={16} />
				<span>Reuse Previous Response ({previousResponses.length})</span>
				<span class="ml-auto text-xs text-gray-500">
					{showPreviousDropdown ? '▼' : '▶'}
				</span>
			</button>

			{#if showPreviousDropdown}
				<div class="mt-3 space-y-2">
					{#each previousResponses as prev}
						<button
							onclick={() => usePreviousResponse(prev.text)}
							class="w-full text-left px-3 py-2 text-sm bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
						>
							<div class="font-medium text-gray-900 dark:text-gray-100 truncate">
								{prev.preview}
							</div>
							<div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
								{new Date(prev.timestamp).toLocaleString()}
							</div>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	/* Add smooth hover transitions */
	button {
		transition: all 0.2s ease;
	}

	button:active {
		transform: scale(0.98);
	}
</style>
