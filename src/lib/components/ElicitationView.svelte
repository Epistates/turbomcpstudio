<!--
  Elicitation View - User Input Request Management

  Shows information about elicitation requests and links to Protocol Inspector.
  Elicitation requests are handled via modal dialogs when they arrive.
-->
<script lang="ts">
	import { uiStore } from '$lib/stores/uiStore';
	import { MessageCircle, ExternalLink, Info } from 'lucide-svelte';

	// Note: Elicitation requests are currently handled via modals and not stored in a dedicated store
	// This view provides information and links to Protocol Inspector where all messages are logged
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
		<!-- Info Section -->
		<section class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-6">
			<div class="flex items-start gap-3">
				<Info size={24} class="text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
				<div class="flex-1">
					<h3 class="text-lg font-semibold text-blue-900 dark:text-blue-100 mb-2">
						How Elicitation Works
					</h3>
					<div class="space-y-2 text-sm text-blue-800 dark:text-blue-200">
						<p>
							<strong>Elicitation</strong> is when an MCP server asks for user input during an operation.
						</p>
						<p>
							When a server sends an elicitation request, a modal dialog will appear asking for your input.
							The schema defines what information is required.
						</p>
						<p>
							All elicitation messages are logged to the <strong>Protocol Inspector</strong> where you can
							view the full JSON-RPC message flow.
						</p>
					</div>
				</div>
			</div>
		</section>

		<!-- Current State Section -->
		<section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
			<div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
				<div class="flex items-center gap-2">
					<MessageCircle size={20} class="text-gray-600 dark:text-gray-400" />
					<h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
						Active Requests
					</h2>
				</div>
			</div>

			<div class="px-6 py-12 text-center">
				<MessageCircle size={48} class="mx-auto text-gray-400 dark:text-gray-600 mb-3" />
				<p class="text-gray-600 dark:text-gray-400 mb-2">No active elicitation requests</p>
				<p class="text-sm text-gray-500 dark:text-gray-500">
					When a server requests user input, a modal dialog will appear
				</p>
			</div>
		</section>

		<!-- Protocol Inspector Link Section -->
		<section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
			<div class="flex items-center justify-between">
				<div>
					<h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-1">
						View Elicitation Messages
					</h3>
					<p class="text-sm text-gray-600 dark:text-gray-400">
						See all elicitation requests and responses in the Protocol Inspector
					</p>
				</div>
				<button
					onclick={() => uiStore.setView('protocol')}
					class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600 rounded-lg transition-colors flex items-center gap-2"
				>
					<ExternalLink size={16} />
					<span>Open Protocol Inspector</span>
				</button>
			</div>
		</section>

		<!-- Testing Info Section -->
		<section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
			<h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3">
				Testing Elicitation
			</h3>
			<div class="space-y-3 text-sm text-gray-600 dark:text-gray-400">
				<div class="flex items-start gap-3">
					<div class="flex-shrink-0 w-6 h-6 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-400 flex items-center justify-center text-xs font-semibold">
						1
					</div>
					<div>
						<strong class="text-gray-900 dark:text-gray-100">Connect a server</strong>
						<p class="mt-0.5">Connect an MCP server that supports elicitation</p>
					</div>
				</div>
				<div class="flex items-start gap-3">
					<div class="flex-shrink-0 w-6 h-6 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-400 flex items-center justify-center text-xs font-semibold">
						2
					</div>
					<div>
						<strong class="text-gray-900 dark:text-gray-100">Trigger an operation</strong>
						<p class="mt-0.5">Call a tool or resource that requires user input</p>
					</div>
				</div>
				<div class="flex items-start gap-3">
					<div class="flex-shrink-0 w-6 h-6 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-400 flex items-center justify-center text-xs font-semibold">
						3
					</div>
					<div>
						<strong class="text-gray-900 dark:text-gray-100">Respond to request</strong>
						<p class="mt-0.5">A modal will appear asking for your input based on the schema</p>
					</div>
				</div>
				<div class="flex items-start gap-3">
					<div class="flex-shrink-0 w-6 h-6 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-400 flex items-center justify-center text-xs font-semibold">
						4
					</div>
					<div>
						<strong class="text-gray-900 dark:text-gray-100">View in Protocol Inspector</strong>
						<p class="mt-0.5">See the full JSON-RPC message exchange</p>
					</div>
				</div>
			</div>
		</section>
	</div>
</div>
