<script lang="ts">
	import { browser } from '$app/environment';
	import { getProjects, getAgents, type Project, type Agent } from '$lib/api/client';
	import Bot from 'lucide-svelte/icons/bot';
	import Search from 'lucide-svelte/icons/search';
	import ArrowRight from 'lucide-svelte/icons/arrow-right';
	import { AgentCardSkeleton } from '$lib/components/skeletons';

	interface AgentWithProject extends Agent {
		projectSlug: string;
	}

	let allAgents = $state<AgentWithProject[]>([]);
	let projects = $state<Project[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Filters
	let selectedProject = $state<string>('all');
	let searchQuery = $state('');

	// Use $effect for client-side data loading in Svelte 5
	$effect(() => {
		if (browser) {
			loadAllAgents();
		}
	});

	async function loadAllAgents() {
		loading = true;
		error = null;
		try {
			projects = await getProjects();

			// Load agents for each project
			const agentPromises = projects.map(async (project) => {
				const agents = await getAgents(project.slug);
				return agents.map(agent => ({ ...agent, projectSlug: project.slug }));
			});

			const agentArrays = await Promise.all(agentPromises);
			allAgents = agentArrays.flat();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load agents';
		} finally {
			loading = false;
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

	// Create a map from slug to human_key
	let projectNameMap = $derived(() => {
		const map = new Map<string, string>();
		for (const p of projects) {
			map.set(p.slug, p.human_key);
		}
		return map;
	});

	function getProjectName(slug: string): string {
		return projectNameMap().get(slug) ?? slug;
	}

	let filteredAgents = $derived(() => {
		let result = allAgents;

		if (selectedProject !== 'all') {
			result = result.filter(a => a.projectSlug === selectedProject);
		}

		if (searchQuery.trim()) {
			const query = searchQuery.toLowerCase();
			result = result.filter(a =>
				a.name.toLowerCase().includes(query) ||
				a.program.toLowerCase().includes(query) ||
				a.model.toLowerCase().includes(query) ||
				(a.task_description && a.task_description.toLowerCase().includes(query))
			);
		}

		return result;
	});
</script>

<div class="space-y-6">
	<!-- Header -->
	<div>
		<h1 class="text-2xl font-bold text-gray-900 dark:text-white">All Agents</h1>
		<p class="text-gray-600 dark:text-gray-400">Browse agents across all projects</p>
	</div>

	<!-- Filters -->
	<div class="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm border border-gray-200 dark:border-gray-700">
		<div class="flex flex-col md:flex-row gap-4">
			<!-- Search -->
			<div class="flex-1">
				<label for="search" class="sr-only">Search agents</label>
				<div class="relative">
					<Search class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
					<input
						id="search"
						type="text"
						bind:value={searchQuery}
						placeholder="Search by name, program, model, or task..."
						class="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
					/>
				</div>
			</div>

			<!-- Project Filter -->
			<div class="md:w-64">
				<label for="projectFilter" class="sr-only">Filter by project</label>
				<select
					id="projectFilter"
					bind:value={selectedProject}
					class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
				>
					<option value="all">All Projects</option>
					{#each projects as project}
						<option value={project.slug}>{project.human_key}</option>
					{/each}
				</select>
			</div>
		</div>
	</div>

	<!-- Error Message -->
	{#if error}
		<div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl p-4">
			<p class="text-red-700 dark:text-red-400">{error}</p>
		</div>
	{/if}

	<!-- Loading State -->
	{#if loading}
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
			{#each Array(6) as _}
				<AgentCardSkeleton />
			{/each}
		</div>
	{:else if filteredAgents().length === 0}
		<!-- Empty State -->
		<div class="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm border border-gray-200 dark:border-gray-700">
			<div class="mb-4 flex justify-center"><Bot class="h-12 w-12 text-gray-400" /></div>
			{#if allAgents.length === 0}
				<h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">No agents yet</h3>
				<p class="text-gray-600 dark:text-gray-400 mb-4">
					Create a project and register agents to get started.
				</p>
				<a
					href="/projects"
					class="inline-block px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
				>
					Go to Projects
				</a>
			{:else}
				<h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">No matching agents</h3>
				<p class="text-gray-600 dark:text-gray-400">
					Try adjusting your search or filter criteria.
				</p>
			{/if}
		</div>
	{:else}
		<!-- Stats -->
		<div class="flex items-center gap-4 text-sm text-gray-600 dark:text-gray-400">
			<span>Showing {filteredAgents().length} of {allAgents.length} agents</span>
			{#if selectedProject !== 'all'}
				<span class="px-2 py-1 bg-primary-100 dark:bg-primary-900 text-primary-700 dark:text-primary-300 rounded-full text-xs">
					{getProjectName(selectedProject)}
				</span>
			{/if}
		</div>

		<!-- Agents Grid -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
			{#each filteredAgents() as agent}
				<div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700 hover:shadow-md transition-shadow">
					<div class="flex items-start justify-between mb-4">
						<div class="flex items-center gap-3">
							<div class="w-10 h-10 bg-primary-100 dark:bg-primary-900 rounded-full flex items-center justify-center">
								<Bot class="h-5 w-5 text-primary-600 dark:text-primary-400" />
							</div>
							<div>
								<h3 class="font-semibold text-gray-900 dark:text-white">{agent.name}</h3>
								<p class="text-sm text-gray-500 dark:text-gray-400">{agent.program}</p>
							</div>
						</div>
					</div>

					<div class="space-y-2 text-sm">
						<div class="flex justify-between">
							<span class="text-gray-500 dark:text-gray-400">Project</span>
							<a
								href="/projects/{agent.projectSlug}"
								class="text-primary-600 dark:text-primary-400 hover:underline font-medium"
							>
								{getProjectName(agent.projectSlug)}
							</a>
						</div>
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
							href="/inbox?project={agent.projectSlug}&agent={agent.name}"
							class="text-primary-600 dark:text-primary-400 hover:text-primary-800 dark:hover:text-primary-300 text-sm font-medium inline-flex items-center gap-1"
						>
							<span>View Inbox</span>
							<ArrowRight class="h-4 w-4" />
						</a>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
