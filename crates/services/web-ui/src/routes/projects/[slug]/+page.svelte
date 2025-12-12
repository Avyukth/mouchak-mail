<script lang="ts">
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { getAgents, registerAgent, type Agent } from '$lib/api/client';

	let agents = $state<Agent[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// New agent form
	let showNewForm = $state(false);
	let newAgent = $state({
		name: '',
		program: '',
		model: '',
		task_description: ''
	});
	let creating = $state(false);

	// Use $effect for client-side data loading in Svelte 5
	$effect(() => {
		if (browser) {
			loadAgents();
		}
	});

	async function loadAgents() {
		loading = true;
		error = null;
		try {
			agents = await getAgents($page.params.slug ?? '');
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load agents';
		} finally {
			loading = false;
		}
	}

	async function createAgent() {
		if (!newAgent.name.trim()) return;

		creating = true;
		error = null;
		try {
			await registerAgent(
				$page.params.slug ?? '',
				newAgent.name.trim(),
				newAgent.program.trim() || 'unknown',
				newAgent.model.trim() || 'unknown',
				newAgent.task_description.trim() || ''
			);
			await loadAgents();
			newAgent = { name: '', program: '', model: '', task_description: '' };
			showNewForm = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create agent';
		} finally {
			creating = false;
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="space-y-6">
	<!-- Breadcrumb -->
	<nav class="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
		<a href="/projects" class="hover:text-primary-600 dark:hover:text-primary-400">Projects</a>
		<span>/</span>
		<span class="text-gray-900 dark:text-white font-medium">{$page.params.slug}</span>
	</nav>

	<!-- Header -->
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold text-gray-900 dark:text-white">
				{$page.params.slug}
			</h1>
			<p class="text-gray-600 dark:text-gray-400">Agents in this project</p>
		</div>
		<button
			onclick={() => showNewForm = !showNewForm}
			class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors flex items-center gap-2"
		>
			<span class="text-lg">+</span>
			<span>Register Agent</span>
		</button>
	</div>

	<!-- New Agent Form -->
	{#if showNewForm}
		<div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
			<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Register New Agent</h2>
			<form onsubmit={(e) => { e.preventDefault(); createAgent(); }} class="space-y-4">
				<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
					<div>
						<label for="agentName" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
							Agent Name *
						</label>
						<input
							id="agentName"
							type="text"
							bind:value={newAgent.name}
							placeholder="BlueStone"
							class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
						/>
					</div>
					<div>
						<label for="agentProgram" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
							Program
						</label>
						<input
							id="agentProgram"
							type="text"
							bind:value={newAgent.program}
							placeholder="antigravity"
							class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
						/>
					</div>
					<div>
						<label for="agentModel" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
							Model
						</label>
						<input
							id="agentModel"
							type="text"
							bind:value={newAgent.model}
							placeholder="gemini-2.0-pro"
							class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
						/>
					</div>
					<div>
						<label for="agentTask" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
							Task Description
						</label>
						<input
							id="agentTask"
							type="text"
							bind:value={newAgent.task_description}
							placeholder="Research and implement features"
							class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
						/>
					</div>
				</div>
				<div class="flex gap-3">
					<button
						type="submit"
						disabled={creating || !newAgent.name.trim()}
						class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
					>
						{creating ? 'Registering...' : 'Register Agent'}
					</button>
					<button
						type="button"
						onclick={() => { showNewForm = false; newAgent = { name: '', program: '', model: '', task_description: '' }; }}
						class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
					>
						Cancel
					</button>
				</div>
			</form>
		</div>
	{/if}

	<!-- Error Message -->
	{#if error}
		<div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl p-4">
			<p class="text-red-700 dark:text-red-400">{error}</p>
		</div>
	{/if}

	<!-- Loading State -->
	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
		</div>
	{:else if agents.length === 0}
		<!-- Empty State -->
		<div class="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm border border-gray-200 dark:border-gray-700">
			<div class="text-4xl mb-4">🤖</div>
			<h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">No agents yet</h3>
			<p class="text-gray-600 dark:text-gray-400 mb-4">
				Register your first agent to start sending and receiving messages.
			</p>
			<button
				onclick={() => showNewForm = true}
				class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
			>
				Register Agent
			</button>
		</div>
	{:else}
		<!-- Agents Grid -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
			{#each agents as agent}
				<div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700 hover:shadow-md transition-shadow">
					<div class="flex items-start justify-between mb-4">
						<div class="flex items-center gap-3">
							<div class="w-10 h-10 bg-primary-100 dark:bg-primary-900 rounded-full flex items-center justify-center">
								<span class="text-lg">🤖</span>
							</div>
							<div>
								<h3 class="font-semibold text-gray-900 dark:text-white">{agent.name}</h3>
								<p class="text-sm text-gray-500 dark:text-gray-400">{agent.program}</p>
							</div>
						</div>
					</div>

					<div class="space-y-2 text-sm">
						<div class="flex justify-between">
							<span class="text-gray-500 dark:text-gray-400">Model</span>
							<span class="text-gray-900 dark:text-white font-mono">{agent.model}</span>
						</div>
						{#if agent.task_description}
							<div>
								<span class="text-gray-500 dark:text-gray-400">Task</span>
								<p class="text-gray-700 dark:text-gray-300 mt-1 line-clamp-2">{agent.task_description}</p>
							</div>
						{/if}
						<div class="flex justify-between pt-2 border-t border-gray-200 dark:border-gray-700">
							<span class="text-gray-500 dark:text-gray-400">Last Active</span>
							<span class="text-gray-600 dark:text-gray-400">{formatDate(agent.last_active_ts)}</span>
						</div>
					</div>

					<div class="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
						<a
							href="/inbox?project={$page.params.slug}&agent={agent.name}"
							class="text-primary-600 dark:text-primary-400 hover:text-primary-800 dark:hover:text-primary-300 text-sm font-medium"
						>
							View Inbox →
						</a>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
