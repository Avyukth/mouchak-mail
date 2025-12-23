<script lang="ts">
	import { browser } from '$app/environment';
	import { getProjects, ensureProject, type Project } from '$lib/api/client';
	import { toast } from 'svelte-sonner';
	import FolderKanban from 'lucide-svelte/icons/folder-kanban';
	import ArrowRight from 'lucide-svelte/icons/arrow-right';
	import Plus from 'lucide-svelte/icons/plus';
	import Calendar from 'lucide-svelte/icons/calendar';
	import { ProjectTableSkeleton } from '$lib/components/skeletons';
	import { BlurFade, ShimmerButton, NumberTicker } from '$lib/components/magic';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';

	let projects = $state<Project[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// New project form
	let showNewForm = $state(false);
	let newProjectPath = $state('');
	let creating = $state(false);

	// Use $effect for client-side data loading in Svelte 5
	$effect(() => {
		if (browser) {
			loadProjects();
		}
	});

	async function loadProjects() {
		loading = true;
		error = null;
		try {
			projects = await getProjects();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load projects';
		} finally {
			loading = false;
		}
	}

	async function createProject() {
		if (!newProjectPath.trim()) return;

		creating = true;
		error = null;
		try {
			await ensureProject(newProjectPath.trim());
			await loadProjects();
			toast.success('Project created successfully');
			newProjectPath = '';
			showNewForm = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create project';
			toast.error(error);
		} finally {
			creating = false;
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}
</script>

<div class="space-y-4 md:space-y-6">
	<!-- Header -->
	<BlurFade delay={0}>
		<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
			<div>
				<h1 class="text-xl md:text-2xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
					Projects
					{#if !loading}
						<Badge variant="secondary">
							<NumberTicker value={projects.length} delay={100} />
						</Badge>
					{/if}
				</h1>
				<p class="text-sm md:text-base text-gray-600 dark:text-gray-400">Manage your agent mail projects</p>
			</div>
			<ShimmerButton on:click={() => showNewForm = true}>
				<Plus class="h-4 w-4 mr-2" />
				New Project
			</ShimmerButton>
		</div>
	</BlurFade>

	<!-- Error Message -->
	{#if error}
		<BlurFade delay={100}>
			<div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl p-4">
				<p class="text-red-700 dark:text-red-400">{error}</p>
			</div>
		</BlurFade>
	{/if}

	<!-- Loading State -->
	{#if loading}
		<ProjectTableSkeleton rows={3} />
	{:else if projects.length === 0}
		<!-- Empty State -->
		<BlurFade delay={100}>
			<div class="bg-white dark:bg-gray-800 rounded-xl p-8 md:p-12 text-center shadow-sm border border-gray-200 dark:border-gray-700">
				<div class="mb-4 flex justify-center"><FolderKanban class="h-12 w-12 text-gray-400" /></div>
				<h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">No projects yet</h3>
				<p class="text-gray-600 dark:text-gray-400 mb-4">
					Create your first project to start sending messages between agents.
				</p>
				<ShimmerButton on:click={() => showNewForm = true}>
					<Plus class="h-4 w-4 mr-2" />
					Create Project
				</ShimmerButton>
			</div>
		</BlurFade>
	{:else}
		<!-- Projects Grid - Cards with hover effects -->
		<BlurFade delay={100}>
			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
				{#each projects as project, index}
					<a
						href="/projects/{project.slug}"
						class="group block animate-in fade-in slide-in-from-bottom-2"
						style="animation-delay: {index * 50}ms; animation-fill-mode: both;"
					>
						<div class="h-full bg-white dark:bg-gray-800 rounded-xl p-5 md:p-6 shadow-sm border border-gray-200 dark:border-gray-700 hover:shadow-lg hover:border-primary-300 dark:hover:border-primary-700 transition-all duration-200 hover:-translate-y-1">
							<div class="flex items-start justify-between mb-4">
								<div class="flex items-center gap-3">
									<div class="w-10 h-10 bg-primary-100 dark:bg-primary-900 rounded-lg flex items-center justify-center group-hover:bg-primary-200 dark:group-hover:bg-primary-800 transition-colors">
										<FolderKanban class="h-5 w-5 text-primary-600 dark:text-primary-400" />
									</div>
									<div class="min-w-0">
										<h3 class="font-semibold text-gray-900 dark:text-white truncate group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
											{project.human_key}
										</h3>
										<p class="text-xs font-mono text-gray-500 dark:text-gray-500 truncate">{project.slug}</p>
									</div>
								</div>
								<ArrowRight class="h-5 w-5 text-gray-400 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors opacity-0 group-hover:opacity-100 -translate-x-2 group-hover:translate-x-0 transition-all" />
							</div>

							<div class="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
								<Calendar class="h-4 w-4" />
								<span>Created {formatDate(project.created_at)}</span>
							</div>
						</div>
					</a>
				{/each}
			</div>
		</BlurFade>
	{/if}
</div>

<!-- New Project Dialog -->
<Dialog.Root bind:open={showNewForm}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>Create New Project</Dialog.Title>
			<Dialog.Description>
				Enter the absolute path to your project directory.
			</Dialog.Description>
		</Dialog.Header>
		<form onsubmit={(e) => { e.preventDefault(); createProject(); }} class="space-y-4">
			<div>
				<label for="projectPath" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
					Project Path (human_key)
				</label>
				<input
					id="projectPath"
					type="text"
					bind:value={newProjectPath}
					placeholder="/path/to/your/project"
					class="w-full min-h-[44px] px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
				/>
			</div>
			<Dialog.Footer class="flex-col sm:flex-row gap-2">
				<Button
					type="button"
					variant="outline"
					onclick={() => { showNewForm = false; newProjectPath = ''; }}
					class="w-full sm:w-auto min-h-[44px]"
				>
					Cancel
				</Button>
				<Button
					type="submit"
					disabled={creating || !newProjectPath.trim()}
					class="w-full sm:w-auto min-h-[44px]"
				>
					{creating ? 'Creating...' : 'Create Project'}
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>

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
