<script lang="ts">
	import { browser } from '$app/environment';
	import { getProjects, ensureProject, type Project } from '$lib/api/client';
	import { toast } from 'svelte-sonner';
	import FolderKanban from 'lucide-svelte/icons/folder-kanban';
	import ArrowRight from 'lucide-svelte/icons/arrow-right';
	import { ProjectTableSkeleton } from '$lib/components/skeletons';

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

<div class="space-y-6">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold text-gray-900 dark:text-white">Projects</h1>
			<p class="text-gray-600 dark:text-gray-400">Manage your agent mail projects</p>
		</div>
		<button
			onclick={() => showNewForm = !showNewForm}
			class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors flex items-center gap-2"
		>
			<span class="text-lg">+</span>
			<span>New Project</span>
		</button>
	</div>

	<!-- New Project Form -->
	{#if showNewForm}
		<div class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
			<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Create New Project</h2>
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
						class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
					/>
					<p class="mt-1 text-sm text-gray-500 dark:text-gray-400">
						The absolute path to your project directory
					</p>
				</div>
				<div class="flex gap-3">
					<button
						type="submit"
						disabled={creating || !newProjectPath.trim()}
						class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
					>
						{creating ? 'Creating...' : 'Create Project'}
					</button>
					<button
						type="button"
						onclick={() => { showNewForm = false; newProjectPath = ''; }}
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
		<ProjectTableSkeleton rows={3} />
	{:else if projects.length === 0}
		<!-- Empty State -->
		<div class="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm border border-gray-200 dark:border-gray-700">
			<div class="mb-4 flex justify-center"><FolderKanban class="h-12 w-12 text-gray-400" /></div>
			<h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">No projects yet</h3>
			<p class="text-gray-600 dark:text-gray-400 mb-4">
				Create your first project to start sending messages between agents.
			</p>
			<button
				onclick={() => showNewForm = true}
				class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
			>
				Create Project
			</button>
		</div>
	{:else}
		<!-- Projects List -->
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
			<table class="w-full">
				<thead class="bg-gray-50 dark:bg-gray-700">
					<tr>
						<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
							Project
						</th>
						<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
							Slug
						</th>
						<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
							Created
						</th>
						<th class="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
							Actions
						</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-gray-200 dark:divide-gray-700">
					{#each projects as project}
						<tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
							<td class="px-6 py-4">
								<a
									href="/projects/{project.slug}"
									class="text-primary-600 dark:text-primary-400 font-medium hover:underline"
								>
									{project.human_key}
								</a>
							</td>
							<td class="px-6 py-4 whitespace-nowrap">
								<span class="text-gray-500 dark:text-gray-500 text-xs font-mono bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded">
									{project.slug}
								</span>
							</td>
							<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
								{formatDate(project.created_at)}
							</td>
							<td class="px-6 py-4 whitespace-nowrap text-right">
								<a
									href="/projects/{project.slug}"
									class="text-primary-600 dark:text-primary-400 hover:text-primary-800 dark:hover:text-primary-300 text-sm font-medium inline-flex items-center gap-1"
								>
									<span>View Agents</span>
									<ArrowRight class="h-4 w-4" />
								</a>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>
