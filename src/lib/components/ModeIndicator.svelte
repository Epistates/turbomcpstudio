<!--
  Mode Indicator Banner

  Shows the current testing mode for MCP Studio.
  Provides clear visibility of whether the user is in manual testing mode,
  which affects how sampling and elicitation requests are handled.
-->
<script lang="ts">
	import { Activity, Info, Settings } from 'lucide-svelte';

	// Props
	const {
		mode = 'manual',
		compact = false
	}: {
		mode?: 'manual' | 'ai' | 'hybrid';
		compact?: boolean;
	} = $props();

	let expanded = $state(!compact);

	// Mode configuration
	const modeConfig = {
		manual: {
			label: 'Manual Testing Mode',
			icon: Activity,
			description: 'You review and respond to all requests - full control for testing',
			color: 'blue',
			bgClass: 'bg-blue-50 dark:bg-blue-900/20',
			borderClass: 'border-blue-200 dark:border-blue-800',
			textClass: 'text-blue-900 dark:text-blue-100',
			secondaryClass: 'text-blue-700 dark:text-blue-300',
			iconClass: 'text-blue-600 dark:text-blue-400'
		},
		ai: {
			label: 'AI Mode',
			icon: Settings,
			description: 'Requests automatically forwarded to configured LLM - production mode',
			color: 'green',
			bgClass: 'bg-green-50 dark:bg-green-900/20',
			borderClass: 'border-green-200 dark:border-green-800',
			textClass: 'text-green-900 dark:text-green-100',
			secondaryClass: 'text-green-700 dark:text-green-300',
			iconClass: 'text-green-600 dark:text-green-400'
		},
		hybrid: {
			label: 'Hybrid Mode',
			icon: Settings,
			description: 'Some requests require approval, others auto-approved',
			color: 'purple',
			bgClass: 'bg-purple-50 dark:bg-purple-900/20',
			borderClass: 'border-purple-200 dark:border-purple-800',
			textClass: 'text-purple-900 dark:text-purple-100',
			secondaryClass: 'text-purple-700 dark:text-purple-300',
			iconClass: 'text-purple-600 dark:text-purple-400'
		}
	};

	const config = $derived(modeConfig[mode]);

	function toggleExpanded() {
		expanded = !expanded;
	}
</script>

<div
	class="border {config.borderClass} {config.bgClass} transition-all"
	class:rounded-lg={!compact}
	class:rounded-none={compact}
>
	{#if compact}
		<!-- Compact Mode - Just a bar -->
		<button
			onclick={toggleExpanded}
			class="w-full px-4 py-2 flex items-center justify-between hover:opacity-80 transition-opacity"
		>
			<div class="flex items-center gap-2">
				<svelte:component this={config.icon} size={16} class={config.iconClass} />
				<span class="text-sm font-medium {config.secondaryClass}">
					{config.label}
				</span>
			</div>
			<Info size={14} class={config.iconClass} />
		</button>

		{#if expanded}
			<div class="px-4 pb-3 border-t {config.borderClass}">
				<p class="text-sm {config.secondaryClass} mt-2">
					{config.description}
				</p>
			</div>
		{/if}
	{:else}
		<!-- Full Mode - Card with details -->
		<div class="p-4">
			<div class="flex items-start gap-3">
				<div class="flex-shrink-0">
					<div
						class="w-10 h-10 rounded-lg {config.bgClass} border {config.borderClass} flex items-center justify-center"
					>
						<svelte:component this={config.icon} size={20} class={config.iconClass} />
					</div>
				</div>
				<div class="flex-1 min-w-0">
					<h3 class="text-base font-semibold {config.textClass} mb-1">
						{config.label}
					</h3>
					<p class="text-sm {config.secondaryClass}">
						{config.description}
					</p>
				</div>
			</div>
		</div>
	{/if}
</div>
