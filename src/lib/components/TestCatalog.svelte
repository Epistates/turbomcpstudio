<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import { Play, Plus, Trash2, RefreshCw, Clock, CheckCircle2, XCircle, Beaker } from 'lucide-svelte';

	// Props
	const { serverId = '' } = $props<{ serverId?: string }>();

	// Type definitions
	interface LLMProviderStatus {
		provider_id: string;
		display_name: string;
		provider_type: string;
		enabled: boolean;
		configured: boolean;
		has_api_key: boolean;
		available_models?: string[];
	}

	interface TestSuite {
		id: string;
		server_id: string;
		name: string;
		description: string | null;
		version: number;
		created_at: string;
		updated_at: string;
		generated_at: string | null;
		schema_hash: string | null;
	}

	interface Test {
		id: string;
		suite_id: string;
		name: string;
		description: string | null;
		kind: any;
		test_data: any;
		assertions: any[];
		category: string;
		complexity: string;
		auto_generated: boolean;
		created_at: string;
		edited_at: string | null;
	}

	interface TestRun {
		id: string;
		suite_id: string;
		started_at: string;
		completed_at: string | null;
		duration_ms: number | null;
		total_tests: number;
		passed: number;
		failed: number;
		errors: number;
		status: string;
		triggered_by: string;
	}

	interface TestResult {
		id: string;
		run_id: string;
		test_id: string;
		passed: boolean;
		error_message: string | null;
		actual_result: any | null;
		duration_ms: number;
		timestamp: string;
	}

	// State
	let suites = $state<TestSuite[]>([]);
	let selectedSuiteId = $state<string | null>(null);
	let tests = $state<Test[]>([]);
	let runs = $state<TestRun[]>([]);
	let results = $state<TestResult[]>([]);
	let loading = $state(false);
	let generating = $state(false);
	let running = $state(false);
	let error = $state<string | null>(null);
	let selectedView = $state<'tests' | 'runs'>('tests');

	// LLM Provider state
	let providers = $state<LLMProviderStatus[]>([]);
	let selectedProvider = $state<string>('');
	let selectedModel = $state<string>('');

	// Derived state
	const selectedSuite = $derived(suites.find((s) => s.id === selectedSuiteId));
	const latestRun = $derived(runs.length > 0 ? runs[0] : null);

	// Get available models for selected provider
	const currentProvider = $derived(providers.find((p) => p.provider_id === selectedProvider));
	const availableModels = $derived(currentProvider?.available_models ?? []);

	// Auto-select first model when provider changes
	$effect(() => {
		if (selectedProvider && availableModels.length > 0 && !availableModels.includes(selectedModel)) {
			selectedModel = availableModels[0];
		}
	});

	// Reload tests when server changes
	$effect(() => {
		if (serverId) {
			selectedSuiteId = null;
			tests = [];
			loadSuites();
		}
	});

	// Load suites and providers on mount
	onMount(async () => {
		await loadProviders();
	});

	async function loadProviders() {
		try {
			const result = await invoke<LLMProviderStatus[]>('list_llm_providers');
			providers = result;

			// Auto-select first enabled provider
			const firstEnabled = result.find((p) => p.enabled);
			if (firstEnabled && !selectedProvider) {
				selectedProvider = firstEnabled.provider_id;
			}
		} catch (e) {
			error = `Failed to load LLM providers: ${e}`;
		}
	}

	async function loadSuites() {
		try {
			loading = true;
			error = null;
			suites = await invoke('get_test_suites', { serverId });

			// Auto-select first suite
			if (suites.length > 0 && !selectedSuiteId) {
				selectedSuiteId = suites[0].id;
				await loadTests();
				await loadRuns();
			}
		} catch (e) {
			error = `Failed to load test suites: ${e}`;
		} finally {
			loading = false;
		}
	}

	async function loadTests() {
		if (!selectedSuiteId) return;

		try {
			tests = await invoke('get_tests', { suiteId: selectedSuiteId });
		} catch (e) {
			error = `Failed to load tests: ${e}`;
		}
	}

	async function loadRuns() {
		if (!selectedSuiteId) return;

		try {
			runs = await invoke('get_test_runs', {
				suiteId: selectedSuiteId,
				limit: 10
			});
		} catch (e) {
			error = `Failed to load test runs: ${e}`;
		}
	}

	async function generateTests() {
		try {
			generating = true;
			error = null;

			const suiteId = await invoke<string>('generate_test_suite', {
				serverId,
				providerId: selectedProvider || null,
				modelId: selectedModel || null
			});

			await loadSuites();
			selectedSuiteId = suiteId;
			await loadTests();
		} catch (e) {
			error = `Failed to generate tests: ${e}`;
		} finally {
			generating = false;
		}
	}

	async function runTests() {
		if (!selectedSuiteId) return;

		try {
			running = true;
			error = null;

			const runId = await invoke<string>('run_test_suite', {
				suiteId: selectedSuiteId,
				serverId
			});

			// Load results
			await loadRuns();
			results = await invoke('get_test_results', { runId });
		} catch (e) {
			error = `Failed to run tests: ${e}`;
		} finally {
			running = false;
		}
	}

	async function deleteSuite(suiteId: string) {
		if (!confirm('Are you sure you want to delete this test suite?')) return;

		try {
			await invoke('delete_test_suite', { suiteId });
			await loadSuites();

			if (selectedSuiteId === suiteId) {
				selectedSuiteId = suites.length > 0 ? suites[0].id : null;
				if (selectedSuiteId) {
					await loadTests();
					await loadRuns();
				}
			}
		} catch (e) {
			error = `Failed to delete suite: ${e}`;
		}
	}

	async function deleteTest(testId: string) {
		if (!confirm('Are you sure you want to delete this test?')) return;

		try {
			await invoke('delete_test', { testId });
			await loadTests();
		} catch (e) {
			error = `Failed to delete test: ${e}`;
		}
	}

	async function deleteRun(runId: string) {
		if (!confirm('Are you sure you want to delete this test run?')) return;

		try {
			await invoke('delete_test_run', { runId });
			await loadRuns();
		} catch (e) {
			error = `Failed to delete test run: ${e}`;
		}
	}

	async function deleteAllTests() {
		if (!selectedSuiteId) return;
		if (!confirm(`Are you sure you want to delete all ${tests.length} tests in this suite?`)) return;

		try {
			for (const test of tests) {
				await invoke('delete_test', { testId: test.id });
			}
			await loadTests();
		} catch (e) {
			error = `Failed to delete tests: ${e}`;
		}
	}

	async function runSingleTest(testId: string) {
		try {
			running = true;
			error = null;

			const result = await invoke('run_single_test', {
				testId,
				serverId
			});

			// Reload runs to show updated results
			await loadRuns();
		} catch (e) {
			error = `Failed to run test: ${e}`;
		} finally {
			running = false;
		}
	}

	function selectSuite(suiteId: string) {
		selectedSuiteId = suiteId;
		loadTests();
		loadRuns();
	}

	function getCategoryColor(category: string): string {
		switch (category) {
			case 'happy_path':
				return 'text-green-600 dark:text-green-400';
			case 'edge_case':
				return 'text-yellow-600 dark:text-yellow-400';
			case 'error':
				return 'text-red-600 dark:text-red-400';
			case 'security':
				return 'text-purple-600 dark:text-purple-400';
			case 'workflow':
				return 'text-blue-600 dark:text-blue-400';
			default:
				return 'text-gray-600 dark:text-gray-400';
		}
	}

	function formatDuration(ms: number): string {
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleString();
	}
</script>

<div class="flex h-full bg-white dark:bg-zinc-900">
	<!-- Sidebar: Test Suites List -->
	<div class="w-80 border-r border-zinc-200 dark:border-zinc-800 flex flex-col">
		<div class="p-4 border-b border-zinc-200 dark:border-zinc-800">
			<div class="mb-4">
				<h2 class="text-lg font-semibold text-zinc-900 dark:text-zinc-100 mb-3">Test Suites</h2>

				<!-- LLM Provider Selector -->
				<div class="mb-2">
					<label class="block text-xs font-medium text-zinc-700 dark:text-zinc-300 mb-1">
						Provider
					</label>
					<select
						bind:value={selectedProvider}
						class="w-full px-3 py-1.5 text-sm bg-white dark:bg-zinc-800 border border-zinc-300 dark:border-zinc-700 rounded-lg focus:ring-2 focus:ring-purple-500 focus:border-transparent"
					>
						{#if providers.length === 0}
							<option value="">No providers configured</option>
						{:else}
							{#each providers.filter(p => p.enabled) as provider}
								<option value={provider.provider_id}>
									{provider.display_name}
									{provider.provider_type === 'local' ? '(Local)' : ''}
								</option>
							{/each}
						{/if}
					</select>
				</div>

				<!-- Model Selector -->
				<div class="mb-3">
					<label class="block text-xs font-medium text-zinc-700 dark:text-zinc-300 mb-1">
						Model
					</label>
					<select
						bind:value={selectedModel}
						disabled={!selectedProvider || availableModels.length === 0}
						class="w-full px-3 py-1.5 text-sm bg-white dark:bg-zinc-800 border border-zinc-300 dark:border-zinc-700 rounded-lg focus:ring-2 focus:ring-purple-500 focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed"
					>
						{#if availableModels.length === 0}
							<option value="">No models available</option>
						{:else}
							{#each availableModels as model}
								<option value={model}>{model}</option>
							{/each}
						{/if}
					</select>
				</div>

				<button
					onclick={generateTests}
					disabled={generating || !serverId || !selectedProvider || !selectedModel}
					class="w-full flex items-center justify-center gap-2 px-3 py-2 text-sm font-medium text-white bg-purple-600 rounded-lg hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
				>
					{#if generating}
						<RefreshCw size={14} class="animate-spin" />
						Generating...
					{:else}
						<Plus size={14} />
						Generate Tests
					{/if}
				</button>
			</div>

			{#if error}
				<div class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
					<p class="text-sm text-red-600 dark:text-red-400">{error}</p>
				</div>
			{/if}
		</div>

		<!-- Suites List -->
		<div class="flex-1 overflow-y-auto">
			{#if loading}
				<div class="flex items-center justify-center h-32">
					<RefreshCw size={24} class="animate-spin text-purple-600" />
				</div>
			{:else if suites.length === 0}
				<div class="p-8 text-center">
					<Beaker size={48} class="mx-auto mb-4 text-zinc-400 dark:text-zinc-600" />
					<p class="text-sm text-zinc-600 dark:text-zinc-400 mb-4">
						No test suites yet. Generate tests using AI to get started!
					</p>
				</div>
			{:else}
				{#each suites as suite}
					<div class="w-full border-b border-zinc-200 dark:border-zinc-800 flex items-stretch">
						<button
							onclick={() => selectSuite(suite.id)}
							class="flex-1 p-4 text-left hover:bg-zinc-50 dark:hover:bg-zinc-800/50 transition-colors {selectedSuiteId === suite.id ? 'bg-purple-50 dark:bg-purple-900/20' : ''}"
						>
							<div class="flex-1 min-w-0">
								<h3 class="font-medium text-zinc-900 dark:text-zinc-100 truncate">
									{suite.name}
								</h3>
								{#if suite.description}
									<p class="text-sm text-zinc-600 dark:text-zinc-400 mt-1 line-clamp-2">
										{suite.description}
									</p>
								{/if}
								<p class="text-xs text-zinc-500 dark:text-zinc-500 mt-2">
									{formatDate(suite.created_at)}
								</p>
							</div>
						</button>
						<button
							onclick={(e) => {
								e.stopPropagation();
								deleteSuite(suite.id);
							}}
							class="px-3 text-zinc-400 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
							title="Delete suite"
						>
							<Trash2 size={16} />
						</button>
					</div>
				{/each}
			{/if}
		</div>
	</div>

	<!-- Main Content: Tests or Runs -->
	<div class="flex-1 flex flex-col">
		{#if !selectedSuiteId}
			<div class="flex-1 flex items-center justify-center">
				<div class="text-center">
					<Beaker size={64} class="mx-auto mb-4 text-zinc-300 dark:text-zinc-700" />
					<p class="text-zinc-600 dark:text-zinc-400">
						Select a test suite or generate tests to get started
					</p>
				</div>
			</div>
		{:else}
			<!-- Header -->
			<div class="p-6 border-b border-zinc-200 dark:border-zinc-800">
				<div class="flex items-center justify-between mb-4">
					<div>
						<h2 class="text-2xl font-bold text-zinc-900 dark:text-zinc-100">
							{selectedSuite?.name}
						</h2>
						{#if selectedSuite?.description}
							<p class="text-zinc-600 dark:text-zinc-400 mt-1">{selectedSuite.description}</p>
						{/if}
					</div>
					<div class="flex gap-2">
						<button
							onclick={runTests}
							disabled={running || tests.length === 0}
							class="flex items-center gap-2 px-4 py-2 text-sm font-medium text-white bg-green-600 rounded-lg hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
						>
							{#if running}
								<RefreshCw size={16} class="animate-spin" />
								Running Tests...
							{:else}
								<Play size={16} />
								Run All Tests ({tests.length})
							{/if}
						</button>
						<button
							onclick={deleteAllTests}
							disabled={tests.length === 0}
							class="flex items-center gap-2 px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-lg hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
							title="Delete all tests in this suite"
						>
							<Trash2 size={16} />
							Clear All
						</button>
					</div>
				</div>

				<!-- Tab Navigation -->
				<div class="flex gap-4 border-b border-zinc-200 dark:border-zinc-800">
					<button
						onclick={() => (selectedView = 'tests')}
						class="px-4 py-2 text-sm font-medium border-b-2 transition-colors"
						class:border-purple-600={selectedView === 'tests'}
						class:text-purple-600={selectedView === 'tests'}
						class:dark:text-purple-400={selectedView === 'tests'}
						class:border-transparent={selectedView !== 'tests'}
						class:text-zinc-600={selectedView !== 'tests'}
						class:dark:text-zinc-400={selectedView !== 'tests'}
					>
						Tests ({tests.length})
					</button>
					<button
						onclick={() => (selectedView = 'runs')}
						class="px-4 py-2 text-sm font-medium border-b-2 transition-colors"
						class:border-purple-600={selectedView === 'runs'}
						class:text-purple-600={selectedView === 'runs'}
						class:dark:text-purple-400={selectedView === 'runs'}
						class:border-transparent={selectedView !== 'runs'}
						class:text-zinc-600={selectedView !== 'runs'}
						class:dark:text-zinc-400={selectedView !== 'runs'}
					>
						Runs ({runs.length})
					</button>
				</div>
			</div>

			<!-- Content -->
			<div class="flex-1 overflow-y-auto p-6">
				{#if selectedView === 'tests'}
					<!-- Tests List -->
					{#if tests.length === 0}
						<div class="text-center py-12">
							<p class="text-zinc-600 dark:text-zinc-400">No tests in this suite</p>
						</div>
					{:else}
						<div class="space-y-4">
							{#each tests as test}
								<div
									class="p-4 border border-zinc-200 dark:border-zinc-800 rounded-lg hover:border-purple-300 dark:hover:border-purple-700 transition-colors"
								>
									<div class="flex items-start justify-between">
										<div class="flex-1">
											<h4 class="font-medium text-zinc-900 dark:text-zinc-100">
												{test.name}
											</h4>
											{#if test.description}
												<p class="text-sm text-zinc-600 dark:text-zinc-400 mt-1">
													{test.description}
												</p>
											{/if}
											<div class="flex gap-3 mt-3">
												<span class="text-xs px-2 py-1 rounded {getCategoryColor(test.category)} bg-zinc-100 dark:bg-zinc-800">
													{test.category}
												</span>
												<span class="text-xs px-2 py-1 rounded text-zinc-600 dark:text-zinc-400 bg-zinc-100 dark:bg-zinc-800">
													{test.complexity}
												</span>
												<span class="text-xs px-2 py-1 rounded text-zinc-600 dark:text-zinc-400 bg-zinc-100 dark:bg-zinc-800">
													{test.assertions.length} assertion{test.assertions.length !== 1 ? 's' : ''}
												</span>
											</div>
										</div>
										<div class="ml-4 flex gap-2">
											<button
												onclick={(e) => {
													e.stopPropagation();
													runSingleTest(test.id);
												}}
												disabled={running}
												class="p-2 text-zinc-400 hover:text-green-600 dark:hover:text-green-400 hover:bg-green-50 dark:hover:bg-green-900/20 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
												title="Run this test"
											>
												<Play size={18} />
											</button>
											<button
												onclick={(e) => {
													e.stopPropagation();
													deleteTest(test.id);
												}}
												class="p-2 text-zinc-400 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition-colors"
												title="Delete test"
											>
												<Trash2 size={18} />
											</button>
										</div>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				{:else}
					<!-- Runs List -->
					{#if runs.length === 0}
						<div class="text-center py-12">
							<p class="text-zinc-600 dark:text-zinc-400">No test runs yet. Run tests to see results!</p>
						</div>
					{:else}
						<div class="space-y-4">
							{#each runs as run}
								<div class="p-4 border border-zinc-200 dark:border-zinc-800 rounded-lg">
									<div class="flex items-center justify-between mb-3">
										<div class="flex items-center gap-3">
											{#if run.status === 'completed'}
												{#if run.failed === 0}
													<CheckCircle2 size={20} class="text-green-600 dark:text-green-400" />
												{:else}
													<XCircle size={20} class="text-red-600 dark:text-red-400" />
												{/if}
											{:else}
												<Clock size={20} class="text-yellow-600 dark:text-yellow-400" />
											{/if}
											<div>
												<p class="font-medium text-zinc-900 dark:text-zinc-100">
													{run.passed}/{run.total_tests} Passed
												</p>
												<p class="text-xs text-zinc-500 dark:text-zinc-500">
													{formatDate(run.started_at)}
												</p>
											</div>
										</div>
										<div class="flex items-center gap-3">
											{#if run.duration_ms}
												<span class="text-sm text-zinc-600 dark:text-zinc-400">
													{formatDuration(run.duration_ms)}
												</span>
											{/if}
											<button
												onclick={(e) => {
													e.stopPropagation();
													deleteRun(run.id);
												}}
												class="p-1.5 text-zinc-400 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition-colors"
												title="Delete run"
											>
												<Trash2 size={16} />
											</button>
										</div>
									</div>
									<div class="flex gap-2">
										<div class="flex-1 h-2 bg-zinc-200 dark:bg-zinc-800 rounded-full overflow-hidden">
											<div
												class="h-full bg-green-600 dark:bg-green-400 transition-all"
												style="width: {(run.passed / run.total_tests) * 100}%"
											></div>
										</div>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.line-clamp-2 {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style>
