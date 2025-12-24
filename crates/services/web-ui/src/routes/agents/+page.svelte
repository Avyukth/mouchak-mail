<script lang="ts">
	import { browser } from '$app/environment';
	import { getProjects, getAgents, type Project, type Agent } from '$lib/api/client';
	import Bot from 'lucide-svelte/icons/bot';
	import Search from 'lucide-svelte/icons/search';
	import ArrowRight from 'lucide-svelte/icons/arrow-right';
	import { AgentCardSkeleton } from '$lib/components/skeletons';
	import { BlurFade, ShimmerButton } from '$lib/components/magic';
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Input } from '$lib/components/ui/input';
	import { FilterCombobox } from '$lib/components/ui/combobox';
	import { SortButton, type SortDirection } from '$lib/components/ui/sort-button';
	import { EmptyState } from '$lib/components/ui/empty-state';
	import { Button } from '$lib/components/ui/button';

	interface AgentWithProject extends Agent {
		projectSlug: string;
	}

	let allAgents = $state<AgentWithProject[]>([]);
	let projects = $state<Project[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Filters
	let selectedProject = $state<string>('');
	let searchQuery = $state('');

	// Sort state
	type SortField = 'name' | 'model' | 'activity';
	let sortField = $state<SortField>('activity');
	let sortDirection = $state<SortDirection>('desc');

	function handleSort(field: string, direction: SortDirection) {
		sortField = field as SortField;
		sortDirection = direction;
	}

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

	// Project options for combobox
	let projectOptions = $derived(projects.map(p => p.human_key));

	function handleProjectChange(humanKey: string) {
		if (!humanKey) {
			selectedProject = '';
		} else {
			const project = projects.find(p => p.human_key === humanKey);
			selectedProject = project?.slug ?? '';
		}
	}

	let filteredAgents = $derived(() => {
		let result = allAgents;

		if (selectedProject) {
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

		// Apply sorting
		return [...result].sort((a, b) => {
			let comparison = 0;
			if (sortField === 'name') {
				comparison = a.name.localeCompare(b.name);
			} else if (sortField === 'model') {
				comparison = a.model.localeCompare(b.model);
			} else if (sortField === 'activity') {
				comparison = new Date(a.last_active_ts).getTime() - new Date(b.last_active_ts).getTime();
			}
			return sortDirection === 'asc' ? comparison : -comparison;
		});
	});
</script>

<div class="space-y-4 md:space-y-6">
	<!-- Header (scrolls away) -->
	<BlurFade delay={0}>
		<div>
			<h1 class="text-xl md:text-2xl font-bold text-foreground">All Agents</h1>
			<p class="text-sm md:text-base text-muted-foreground">Browse agents across all projects</p>
		</div>
	</BlurFade>

	<!-- Sticky Toolbar: Search + Filter + Sort + Count -->
	<div class="sticky top-0 z-10 -mx-4 md:-mx-6 px-4 md:px-6 py-3 bg-background/95 backdrop-blur-sm border-b border-border">
		<div class="flex flex-col md:flex-row gap-3 md:gap-4">
			<!-- Search -->
			<div class="flex-1">
				<div class="relative">
					<Search class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
					<Input
						type="text"
						bind:value={searchQuery}
						placeholder="Search by name, program, model, or task..."
						class="pl-10"
					/>
				</div>
			</div>

			<!-- Project Filter -->
			<div class="md:w-64">
				<FilterCombobox
					value={selectedProject ? getProjectName(selectedProject) : ''}
					onValueChange={handleProjectChange}
					options={projectOptions}
					placeholder="All Projects"
					searchPlaceholder="Search projects..."
					emptyMessage="No projects found."
				/>
			</div>

			<!-- Sort Controls -->
			<div class="flex items-center gap-1">
				<span class="text-xs text-muted-foreground mr-2">Sort by:</span>
				<SortButton
					field="name"
					label="Name"
					currentField={sortField}
					currentDirection={sortDirection}
					onSort={handleSort}
				/>
				<SortButton
					field="model"
					label="Model"
					currentField={sortField}
					currentDirection={sortDirection}
					onSort={handleSort}
				/>
				<SortButton
					field="activity"
					label="Activity"
					currentField={sortField}
					currentDirection={sortDirection}
					onSort={handleSort}
				/>
			</div>
		</div>

		<!-- Stats (always visible when not loading) -->
		{#if !loading}
			<div class="flex items-center gap-4 text-sm text-muted-foreground mt-3">
				<span>Showing {filteredAgents().length} of {allAgents.length} agents</span>
				{#if selectedProject}
					<Badge variant="secondary">
						{getProjectName(selectedProject)}
					</Badge>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Error Message -->
	{#if error}
		<BlurFade delay={150}>
			<div class="bg-destructive/10 border border-destructive/30 rounded-xl p-4">
				<p class="text-destructive">{error}</p>
			</div>
		</BlurFade>
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
		<BlurFade delay={150}>
			<Card.Root class="p-8 md:p-12">
				{#if allAgents.length === 0}
					<EmptyState
						title="No agents yet"
						description="Create a project and register agents to get started."
						actionLabel="Go to Projects"
						onAction={() => window.location.href = '/projects'}
					>
						{#snippet icon()}
							<Bot class="h-12 w-12" />
						{/snippet}
					</EmptyState>
				{:else}
					<EmptyState
						title="No matching agents"
						description="Try adjusting your search or filter criteria."
						actionLabel="Clear filters"
						onAction={() => { searchQuery = ''; selectedProject = ''; }}
					>
						{#snippet icon()}
							<Bot class="h-12 w-12" />
						{/snippet}
					</EmptyState>
				{/if}
			</Card.Root>
		</BlurFade>
	{:else}
		<!-- Agents Grid -->
		<BlurFade delay={200}>
			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
				{#each filteredAgents() as agent, index}
					<div
						class="animate-in fade-in slide-in-from-bottom-2"
						style="animation-delay: calc({index} * var(--delay-stagger)); animation-fill-mode: both;"
					>
						<Card.Root class="h-full hover:shadow-md transition-shadow">
							<Card.Content class="p-5 md:p-6">
								<div class="flex items-start justify-between mb-4">
									<div class="flex items-center gap-3">
										<div class="w-10 h-10 bg-primary/10 rounded-full flex items-center justify-center">
											<Bot class="h-5 w-5 text-primary" />
										</div>
										<div>
											<h3 class="font-semibold text-foreground">{agent.name}</h3>
											<p class="text-sm text-muted-foreground">{agent.program}</p>
										</div>
									</div>
								</div>

								<div class="space-y-2 text-sm">
									<div class="flex justify-between">
										<span class="text-muted-foreground">Project</span>
										<a
											href="/projects/{agent.projectSlug}"
											class="text-primary hover:underline font-medium"
										>
											{getProjectName(agent.projectSlug)}
										</a>
									</div>
									<div class="flex justify-between">
										<span class="text-muted-foreground">Model</span>
										<span class="text-foreground font-mono">{agent.model}</span>
									</div>
									{#if agent.task_description}
										<div>
											<span class="text-muted-foreground">Task</span>
											<p class="text-foreground/80 mt-1 line-clamp-2">{agent.task_description}</p>
										</div>
									{/if}
									<div class="flex justify-between pt-2 border-t border-border">
										<span class="text-muted-foreground">Last Active</span>
										<span class="text-muted-foreground">{formatDate(agent.last_active_ts)}</span>
									</div>
								</div>

								<div class="mt-4 pt-4 border-t border-border">
									<a
										href="/inbox?project={agent.projectSlug}&agent={agent.name}"
										class="text-primary hover:text-primary/80 text-sm font-medium inline-flex items-center gap-1"
									>
										<span>View Inbox</span>
										<ArrowRight class="h-4 w-4" />
									</a>
								</div>
							</Card.Content>
						</Card.Root>
					</div>
				{/each}
			</div>
		</BlurFade>
	{/if}
</div>

<style>
	/* Staggered animation keyframes */
	@keyframes fade-in {
		from { opacity: 0; }
		to { opacity: 1; }
	}

	@keyframes slide-in-from-bottom-2 {
		from { transform: translateY(8px); }
		to { transform: translateY(0); }
	}

	.animate-in {
		animation: fade-in 300ms ease-out, slide-in-from-bottom-2 300ms ease-out;
	}

	/* Respect reduced motion */
	@media (prefers-reduced-motion: reduce) {
		.animate-in {
			animation: none;
		}
	}
</style>
