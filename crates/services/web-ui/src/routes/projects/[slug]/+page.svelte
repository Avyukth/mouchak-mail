<script lang="ts">
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { getAgents, registerAgent, getProjectInfo, deleteAgent, type Agent, type Project } from '$lib/api/client';
	import { toast } from 'svelte-sonner';
	import Bot from 'lucide-svelte/icons/bot';
	import ArrowRight from 'lucide-svelte/icons/arrow-right';
	import ArrowLeft from 'lucide-svelte/icons/arrow-left';
	import Plus from 'lucide-svelte/icons/plus';
	import Clock from 'lucide-svelte/icons/clock';
	import Cpu from 'lucide-svelte/icons/cpu';
	import Inbox from 'lucide-svelte/icons/inbox';
	import MoreVertical from 'lucide-svelte/icons/more-vertical';
	import Trash2 from 'lucide-svelte/icons/trash-2';
	import { AgentCardSkeleton } from '$lib/components/skeletons';
	import { BlurFade, ShimmerButton, NumberTicker } from '$lib/components/magic';
	import * as Dialog from '$lib/components/ui/dialog';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
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

	// Delete agent state
	let showDeleteDialog = $state(false);
	let agentToDelete = $state<Agent | null>(null);
	let deleting = $state(false);
	let openDropdownName = $state<string | null>(null);

	function handleDeleteClick(e: Event, agent: Agent) {
		e.preventDefault();
		e.stopPropagation();
		// Close the dropdown first
		openDropdownName = null;
		// Small delay to let dropdown close animation complete
		setTimeout(() => {
			agentToDelete = agent;
			showDeleteDialog = true;
		}, 50);
	}

	async function confirmDelete() {
		if (!agentToDelete) return;
		deleting = true;
		try {
			await deleteAgent($page.params.slug ?? '', agentToDelete.name);
			toast.success(`Agent "${agentToDelete.name}" deleted`);
			await loadProjectData();
		} catch (e) {
			toast.error(e instanceof Error ? e.message : 'Failed to delete agent');
		} finally {
			deleting = false;
			showDeleteDialog = false;
			agentToDelete = null;
		}
	}
</script>

<div class="pt-4 md:pt-6 pb-4 md:pb-6 space-y-4 md:space-y-6">
	<!-- Breadcrumb -->
	<BlurFade delay={0}>
		<nav class="flex items-center gap-2 text-sm text-muted-foreground">
			<a
				href="/projects"
				class="min-h-touch px-2 -ml-2 flex items-center gap-1 hover:text-primary rounded-lg hover:bg-muted transition-colors"
			>
				<ArrowLeft class="h-4 w-4" />
				<span>Projects</span>
			</a>
			<span>/</span>
			<span class="text-foreground font-medium truncate">{projectName}</span>
		</nav>
	</BlurFade>

	<!-- Header -->
	<BlurFade delay={50}>
		<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
			<div>
				<h1 class="text-xl md:text-2xl font-bold text-foreground flex items-center gap-2">
					{projectName}
					{#if !loading}
						<Badge variant="secondary">
							<NumberTicker value={agents.length} delay={100} /> agents
						</Badge>
					{/if}
				</h1>
				<p class="text-sm md:text-base text-muted-foreground">
					Registered agents for this project
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
			<div class="bg-card rounded-xl p-8 md:p-12 text-center shadow-sm border border-border">
				<div class="mb-4 flex justify-center"><Bot class="h-12 w-12 text-muted-foreground" /></div>
				<h3 class="text-lg font-semibold text-foreground mb-2">No agents yet</h3>
				<p class="text-muted-foreground mb-4">
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
						class="group bg-card rounded-xl p-5 md:p-6 shadow-sm border border-border hover:shadow-lg hover:border-primary/30 transition-all duration-200 animate-in fade-in slide-in-from-bottom-2"
						style="animation-delay: calc({index} * var(--delay-stagger)); animation-fill-mode: both;"
					>
						<div class="flex items-start justify-between mb-4">
							<div class="flex items-center gap-3">
								<div class="w-12 h-12 bg-primary/10 rounded-xl flex items-center justify-center group-hover:bg-primary/20 transition-colors">
									<Bot class="h-6 w-6 text-primary" />
								</div>
								<div class="min-w-0">
									<h3 class="font-semibold text-foreground truncate">{agent.name}</h3>
									<p class="text-sm text-muted-foreground truncate">{agent.program}</p>
								</div>
							</div>
							<DropdownMenu.Root
								open={openDropdownName === agent.name}
								onOpenChange={(open) => {
									openDropdownName = open ? agent.name : null;
								}}
							>
								<DropdownMenu.Trigger>
									{#snippet child({ props })}
										<Button
											{...props}
											variant="ghost"
											size="icon"
											class="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity"
										>
											<MoreVertical class="h-4 w-4" />
											<span class="sr-only">More options</span>
										</Button>
									{/snippet}
								</DropdownMenu.Trigger>
								<DropdownMenu.Content align="end">
									<DropdownMenu.Item
										class="text-destructive focus:text-destructive"
										onclick={(e: Event) => handleDeleteClick(e, agent)}
									>
										<Trash2 class="h-4 w-4 mr-2" />
										Delete
									</DropdownMenu.Item>
								</DropdownMenu.Content>
							</DropdownMenu.Root>
						</div>

						<div class="space-y-3 text-sm">
							<div class="flex items-center gap-2 text-muted-foreground">
								<Cpu class="h-4 w-4 shrink-0" />
								<span class="font-mono truncate">{agent.model}</span>
							</div>
							{#if agent.task_description}
								<p class="text-muted-foreground line-clamp-2">{agent.task_description}</p>
							{/if}
							<div class="flex items-center gap-2 pt-2 border-t border-border text-muted-foreground">
								<Clock class="h-4 w-4 shrink-0" />
								<span class="text-xs">Active {formatDate(agent.last_active_ts)}</span>
							</div>
						</div>

						<div class="mt-4 pt-4 border-t border-border">
							<a
								href="/inbox?project={$page.params.slug}&agent={agent.name}"
								class="min-h-touch w-full flex items-center justify-center gap-2 px-4 py-2 bg-primary/10 text-primary rounded-lg hover:bg-primary/20 transition-colors font-medium"
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
					<label for="agentName" class="block text-sm font-medium text-foreground mb-1">
						Agent Name *
					</label>
					<input
						id="agentName"
						type="text"
						bind:value={newAgent.name}
						placeholder="BlueStone"
						class="w-full min-h-touch px-4 py-2 border border-input rounded-lg bg-background text-foreground focus:ring-2 focus:ring-ring focus:border-transparent"
					/>
				</div>
				<div>
					<label for="agentProgram" class="block text-sm font-medium text-foreground mb-1">
						Program
					</label>
					<input
						id="agentProgram"
						type="text"
						bind:value={newAgent.program}
						placeholder="antigravity"
						class="w-full min-h-touch px-4 py-2 border border-input rounded-lg bg-background text-foreground focus:ring-2 focus:ring-ring focus:border-transparent"
					/>
				</div>
				<div>
					<label for="agentModel" class="block text-sm font-medium text-foreground mb-1">
						Model
					</label>
					<input
						id="agentModel"
						type="text"
						bind:value={newAgent.model}
						placeholder="gemini-2.0-pro"
						class="w-full min-h-touch px-4 py-2 border border-input rounded-lg bg-background text-foreground focus:ring-2 focus:ring-ring focus:border-transparent"
					/>
				</div>
				<div class="sm:col-span-2">
					<label for="agentTask" class="block text-sm font-medium text-foreground mb-1">
						Task Description
					</label>
					<input
						id="agentTask"
						type="text"
						bind:value={newAgent.task_description}
						placeholder="Research and implement features"
						class="w-full min-h-touch px-4 py-2 border border-input rounded-lg bg-background text-foreground focus:ring-2 focus:ring-ring focus:border-transparent"
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

<!-- Delete Agent Confirmation Dialog -->
<AlertDialog.Root bind:open={showDeleteDialog}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Delete Agent</AlertDialog.Title>
			<AlertDialog.Description>
				Are you sure you want to delete <strong>{agentToDelete?.name}</strong>?
				This action cannot be undone. All messages and data associated with this agent will be permanently deleted.
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel
				onclick={() => {
					showDeleteDialog = false;
					agentToDelete = null;
				}}
			>
				Cancel
			</AlertDialog.Cancel>
			<AlertDialog.Action
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
				disabled={deleting}
				onclick={confirmDelete}
			>
				{deleting ? 'Deleting...' : 'Delete Agent'}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

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
