<script lang="ts">
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { getAgents, registerAgent, getProjectInfo, type Agent, type Project } from '$lib/api/client';
	import { toast } from 'svelte-sonner';
	import Bot from 'lucide-svelte/icons/bot';
	import ArrowRight from 'lucide-svelte/icons/arrow-right';
	import ArrowLeft from 'lucide-svelte/icons/arrow-left';
	import Plus from 'lucide-svelte/icons/plus';
	import Clock from 'lucide-svelte/icons/clock';
	import Cpu from 'lucide-svelte/icons/cpu';
	import Inbox from 'lucide-svelte/icons/inbox';
	import { AgentCardSkeleton } from '$lib/components/skeletons';
	import { BlurFade, ShimmerButton, NumberTicker } from '$lib/components/magic';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';

	let agents = $state<Agent[]>([]);
	let project = $state<Project | null>(null);
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

	// Derived project name - uses human_key if available, falls back to slug
	let projectName = $derived(project?.human_key ?? $page.params.slug);

	// Use $effect for client-side data loading in Svelte 5
	$effect(() => {
		if (browser) {
			loadProjectData();
		}
	});

	async function loadProjectData() {
		loading = true;
		error = null;
		try {
			const [projectInfo, agentList] = await Promise.all([
				getProjectInfo($page.params.slug ?? ''),
				getAgents($page.params.slug ?? '')
			]);
			project = projectInfo;
			agents = agentList;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load project data';
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
			await loadProjectData();
			toast.success(`Agent "${newAgent.name}" registered successfully`);
			newAgent = { name: '', program: '', model: '', task_description: '' };
			showNewForm = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create agent';
			toast.error(error);
		} finally {
			creating = false;
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="space-y-4 md:space-y-6">
	<!-- Breadcrumb -->
	<BlurFade delay={0}>
		<nav class="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
			<a
				href="/projects"
				class="min-h-[44px] px-2 -ml-2 flex items-center gap-1 hover:text-primary-600 dark:hover:text-primary-400 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
			>
				<ArrowLeft class="h-4 w-4" />
				<span>Projects</span>
			</a>
			<span>/</span>
			<span class="text-gray-900 dark:text-white font-medium truncate">{projectName}</span>
		</nav>
	</BlurFade>

	<!-- Header -->
	<BlurFade delay={50}>
		<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
			<div>
				<h1 class="text-xl md:text-2xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
					{projectName}
					{#if !loading}
						<Badge variant="secondary">
							<NumberTicker value={agents.length} delay={100} /> agents
						</Badge>
					{/if}
				</h1>
				<p class="text-sm md:text-base text-gray-600 dark:text-gray-400 flex items-center gap-2">
					<span>Registered agents</span>
					{#if project}
						<code class="text-xs font-mono bg-gray-100 dark:bg-gray-700 px-2 py-0.5 rounded">{project.slug}</code>
					{/if}
				</p>
			</div>
			<ShimmerButton on:click={() => showNewForm = true}>
				<Plus class="h-4 w-4 mr-2" />
				Register Agent
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
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
			{#each Array(3) as _}
				<AgentCardSkeleton />
			{/each}
		</div>
	{:else if agents.length === 0}
		<!-- Empty State -->
		<BlurFade delay={100}>
			<div class="bg-white dark:bg-gray-800 rounded-xl p-8 md:p-12 text-center shadow-sm border border-gray-200 dark:border-gray-700">
				<div class="mb-4 flex justify-center"><Bot class="h-12 w-12 text-gray-400" /></div>
				<h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">No agents yet</h3>
				<p class="text-gray-600 dark:text-gray-400 mb-4">
					Register your first agent to start sending and receiving messages.
				</p>
				<ShimmerButton on:click={() => showNewForm = true}>
					<Plus class="h-4 w-4 mr-2" />
					Register Agent
				</ShimmerButton>
			</div>
		</BlurFade>
	{:else}
		<!-- Agents Grid -->
		<BlurFade delay={100}>
			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
				{#each agents as agent, index}
					<div
						class="group bg-white dark:bg-gray-800 rounded-xl p-5 md:p-6 shadow-sm border border-gray-200 dark:border-gray-700 hover:shadow-lg hover:border-primary-300 dark:hover:border-primary-700 transition-all duration-200 animate-in fade-in slide-in-from-bottom-2"
						style="animation-delay: {index * 50}ms; animation-fill-mode: both;"
					>
						<div class="flex items-start justify-between mb-4">
							<div class="flex items-center gap-3">
								<div class="w-12 h-12 bg-primary-100 dark:bg-primary-900 rounded-xl flex items-center justify-center group-hover:bg-primary-200 dark:group-hover:bg-primary-800 transition-colors">
									<Bot class="h-6 w-6 text-primary-600 dark:text-primary-400" />
								</div>
								<div class="min-w-0">
									<h3 class="font-semibold text-gray-900 dark:text-white truncate">{agent.name}</h3>
									<p class="text-sm text-gray-500 dark:text-gray-400 truncate">{agent.program}</p>
								</div>
							</div>
						</div>

						<div class="space-y-3 text-sm">
							<div class="flex items-center gap-2 text-gray-500 dark:text-gray-400">
								<Cpu class="h-4 w-4 shrink-0" />
								<span class="font-mono truncate">{agent.model}</span>
							</div>
							{#if agent.task_description}
								<p class="text-gray-600 dark:text-gray-400 line-clamp-2">{agent.task_description}</p>
							{/if}
							<div class="flex items-center gap-2 pt-2 border-t border-gray-200 dark:border-gray-700 text-gray-500 dark:text-gray-400">
								<Clock class="h-4 w-4 shrink-0" />
								<span class="text-xs">Active {formatDate(agent.last_active_ts)}</span>
							</div>
						</div>

						<div class="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
							<a
								href="/inbox?project={$page.params.slug}&agent={agent.name}"
								class="min-h-[44px] w-full flex items-center justify-center gap-2 px-4 py-2 bg-primary-50 dark:bg-primary-900/20 text-primary-600 dark:text-primary-400 rounded-lg hover:bg-primary-100 dark:hover:bg-primary-900/40 transition-colors font-medium"
							>
								<Inbox class="h-4 w-4" />
								<span>View Inbox</span>
								<ArrowRight class="h-4 w-4" />
							</a>
						</div>
					</div>
				{/each}
			</div>
		</BlurFade>
	{/if}
</div>

<!-- New Agent Dialog -->
<Dialog.Root bind:open={showNewForm}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>Register New Agent</Dialog.Title>
			<Dialog.Description>
				Add a new agent to {projectName}
			</Dialog.Description>
		</Dialog.Header>
		<form onsubmit={(e) => { e.preventDefault(); createAgent(); }} class="space-y-4">
			<div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
				<div class="sm:col-span-2">
					<label for="agentName" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
						Agent Name *
					</label>
					<input
						id="agentName"
						type="text"
						bind:value={newAgent.name}
						placeholder="BlueStone"
						class="w-full min-h-[44px] px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
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
						class="w-full min-h-[44px] px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
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
						class="w-full min-h-[44px] px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
					/>
				</div>
				<div class="sm:col-span-2">
					<label for="agentTask" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
						Task Description
					</label>
					<input
						id="agentTask"
						type="text"
						bind:value={newAgent.task_description}
						placeholder="Research and implement features"
						class="w-full min-h-[44px] px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
					/>
				</div>
			</div>
			<Dialog.Footer class="flex-col sm:flex-row gap-2">
				<Button
					type="button"
					variant="outline"
					onclick={() => { showNewForm = false; newAgent = { name: '', program: '', model: '', task_description: '' }; }}
					class="w-full sm:w-auto min-h-[44px]"
				>
					Cancel
				</Button>
				<Button
					type="submit"
					disabled={creating || !newAgent.name.trim()}
					class="w-full sm:w-auto min-h-[44px]"
				>
					{creating ? 'Registering...' : 'Register Agent'}
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
