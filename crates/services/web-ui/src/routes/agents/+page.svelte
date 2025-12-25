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

<div class="pb-4 md:pb-6">
	<!-- Page Header (scrolls away) -->
	<div class="pt-4 md:pt-6 pb-4">
		<h1 class="text-xl md:text-2xl font-bold text-foreground">All Agents</h1>
		<p class="text-sm text-muted-foreground">Browse agents across all projects</p>
	</div>

	<!-- Sticky Toolbar (sticks directly below header on mobile, breadcrumb on desktop) -->
	<div class="sticky top-0 z-20 -mx-4 md:-mx-6 px-4 md:px-6 py-3 bg-background border-b border-border">
		<div class="flex flex-col sm:flex-row gap-3 sm:items-center">
			<div class="flex-1">
				<div class="relative">
					<Search class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
					<Input
						type="text"
						bind:value={searchQuery}
						placeholder="Search agents..."
						class="pl-10 h-9"
					/>
				</div>
			</div>

			<div class="flex items-center gap-2">
				<FilterCombobox
					value={selectedProject ? getProjectName(selectedProject) : ''}
					onValueChange={handleProjectChange}
					options={projectOptions}
					placeholder="All Projects"
					searchPlaceholder="Search projects..."
					emptyMessage="No projects found."
				/>

				<div class="flex items-center gap-1 border-l border-border pl-2">
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
		</div>

		{#if !loading}
			<div class="flex items-center gap-2 mt-2 text-xs text-muted-foreground">
				<span>{filteredAgents().length} of {allAgents.length} agents</span>
				{#if selectedProject}
					<Badge variant="secondary" class="text-xs">
						{getProjectName(selectedProject)}
					</Badge>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Content area with spacing after sticky toolbar -->
	<div class="mt-4 space-y-4">
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
		<BlurFade delay={200}>
			<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-4">
				{#each filteredAgents() as agent, index}
					<a
						href="/inbox?project={agent.projectSlug}&agent={agent.name}"
						class="group block animate-in fade-in slide-in-from-bottom-2"
						style="animation-delay: calc({index} * var(--delay-stagger)); animation-fill-mode: both;"
					>
						<Card.Root class="h-full shadow-material hover:shadow-material-hover hover:border-primary/30 transition-all duration-300 hover:-translate-y-1">
							<Card.Content class="p-4">
								<div class="flex items-start gap-3 mb-3">
									<div class="flex-shrink-0 w-11 h-11 bg-gradient-to-br from-primary/20 to-primary/10 rounded-xl flex items-center justify-center group-hover:from-primary/30 group-hover:to-primary/20 transition-all duration-300">
										<Bot class="h-5 w-5 text-primary" />
									</div>
									<div class="flex-1 min-w-0">
										<h3 class="font-semibold text-sm text-foreground truncate group-hover:text-primary transition-colors" title={agent.name}>
											{agent.name}
										</h3>
										<p class="text-xs text-muted-foreground truncate">{agent.program}</p>
									</div>
								</div>

								<div class="space-y-1.5 text-xs mb-3">
									<div class="flex justify-between items-center">
										<span class="text-muted-foreground">Project</span>
										<span class="text-foreground font-medium truncate ml-2 max-w-[120px]" title={getProjectName(agent.projectSlug)}>
											{getProjectName(agent.projectSlug)}
										</span>
									</div>
									<div class="flex justify-between items-center">
										<span class="text-muted-foreground">Model</span>
										<span class="text-foreground font-mono truncate ml-2 max-w-[120px]" title={agent.model}>{agent.model}</span>
									</div>
								</div>

								<div class="h-10 mb-3">
									{#if agent.task_description}
										<p class="text-xs text-muted-foreground line-clamp-2" title={agent.task_description}>{agent.task_description}</p>
									{:else}
										<p class="text-xs text-muted-foreground/50 italic">No task description</p>
									{/if}
								</div>

								<div class="flex items-center justify-between pt-3 border-t border-border/40">
									<span class="text-[10px] text-muted-foreground">{formatDate(agent.last_active_ts)}</span>
									<span class="text-xs text-primary font-medium inline-flex items-center gap-1 group-hover:gap-2 transition-all">
										Inbox
										<ArrowRight class="h-3 w-3" />
									</span>
								</div>
							</Card.Content>
						</Card.Root>
					</a>
				{/each}
			</div>
		</BlurFade>
	{/if}
	</div>
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
