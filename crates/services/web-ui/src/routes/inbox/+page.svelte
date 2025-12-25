<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { getProjects, getAgents, getInbox, type Project, type Agent, type Message } from '$lib/api/client';
	import { toast } from 'svelte-sonner';
	import ComposeMessage from '$lib/components/ComposeMessage.svelte';

	// shadcn/ui components
	import { Button } from '$lib/components/ui/button/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Sheet from '$lib/components/ui/sheet/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { FilterCombobox } from '$lib/components/ui/combobox/index.js';
	import { ShimmerButton, BlurFade } from '$lib/components/magic';
	import { MessageListSkeleton } from '$lib/components/skeletons';
	import { EmptyState } from '$lib/components/ui/empty-state';

	// Icons
	import Inbox from 'lucide-svelte/icons/inbox';
	import PenSquare from 'lucide-svelte/icons/pen-square';
	import RefreshCw from 'lucide-svelte/icons/refresh-cw';
	import X from 'lucide-svelte/icons/x';

	let projects = $state<Project[]>([]);
	let agents = $state<Agent[]>([]);
	let messages = $state<Message[]>([]);
	let loading = $state(true);
	let loadingMessages = $state(false);
	let error = $state<string | null>(null);

	// Selected filters from URL or user selection
	let selectedProject = $state<string>('');
	let selectedAgent = $state<string>('');

	// Compose modal
	let showCompose = $state(false);

	// Detect mobile viewport
	let isMobile = $state(false);
	$effect(() => {
		if (browser) {
			const checkMobile = () => {
				isMobile = window.innerWidth < 768;
			};
			checkMobile();
			window.addEventListener('resize', checkMobile);
			return () => window.removeEventListener('resize', checkMobile);
		}
	});

	// Derived options for comboboxes
	let projectOptions = $derived(projects.map(p => p.human_key));
	let agentOptions = $derived(agents.map(a => a.name));

	// Map display values to slugs
	function getProjectSlug(humanKey: string): string {
		return projects.find(p => p.human_key === humanKey)?.slug ?? '';
	}
	function getProjectHumanKey(slug: string): string {
		return projects.find(p => p.slug === slug)?.human_key ?? '';
	}

	// Use $effect for client-side data loading in Svelte 5
	$effect(() => {
		if (browser) {
			initPage();
		}
	});

	async function initPage() {
		try {
			projects = await getProjects();

			// Check URL params for pre-selection
			const urlProject = $page.url.searchParams.get('project');
			const urlAgent = $page.url.searchParams.get('agent');

			if (urlProject) {
				selectedProject = urlProject;
				await loadAgentsForProject(urlProject);
				if (urlAgent) {
					selectedAgent = urlAgent;
					await loadMessages();
				}
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load data';
		} finally {
			loading = false;
		}
	}

	async function loadAgentsForProject(projectSlug: string) {
		try {
			agents = await getAgents(projectSlug);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load agents';
			agents = [];
		}
	}

	async function handleProjectChange(humanKey: string) {
		const slug = getProjectSlug(humanKey);
		selectedProject = slug;
		selectedAgent = '';
		messages = [];
		if (slug) {
			await loadAgentsForProject(slug);
			updateUrl();
		} else {
			agents = [];
		}
	}

	async function handleAgentChange(agentName: string) {
		selectedAgent = agentName;
		updateUrl();
		if (selectedProject && agentName) {
			await loadMessages();
		} else {
			messages = [];
		}
	}

	async function loadMessages() {
		if (!selectedProject || !selectedAgent) return;
		if (loadingMessages) return;

		loadingMessages = true;
		error = null;
		try {
			messages = await getInbox(selectedProject, selectedAgent);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load messages';
			messages = [];
		} finally {
			loadingMessages = false;
		}
	}

	function updateUrl() {
		const params = new URLSearchParams();
		if (selectedProject) params.set('project', selectedProject);
		if (selectedAgent) params.set('agent', selectedAgent);
		const newUrl = params.toString() ? `?${params.toString()}` : '/inbox';
		goto(newUrl, { replaceState: true, keepFocus: true });
	}

	function formatDate(dateStr: string): string {
		const date = new Date(dateStr);
		const now = new Date();
		const isToday = date.toDateString() === now.toDateString();

		if (isToday) {
			return date.toLocaleTimeString('en-US', {
				hour: '2-digit',
				minute: '2-digit'
			});
		}

		return date.toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric'
		});
	}

	function getImportanceVariant(importance: string): "default" | "secondary" | "destructive" | "outline" {
		switch (importance) {
			case 'high': return 'destructive';
			case 'low': return 'secondary';
			default: return 'default';
		}
	}

	function truncateBody(body: string | undefined, maxLength: number = 100): string {
		if (!body) return '';
		if (body.length <= maxLength) return body;
		return body.substring(0, maxLength) + '...';
	}

	function handleMessageSent() {
		showCompose = false;
		loadMessages();
		toast.success('Message sent successfully');
	}
</script>

<div class="pt-4 md:pt-6 pb-4 md:pb-6 space-y-4 md:space-y-6">
	<!-- Header -->
	<BlurFade delay={0}>
		<div class="flex items-center justify-between">
			<div>
				<h1 class="text-xl md:text-2xl font-bold text-foreground">Inbox</h1>
				<p class="text-sm md:text-base text-muted-foreground">View messages for your agents</p>
			</div>
			{#if selectedProject && selectedAgent}
				<ShimmerButton
					size={isMobile ? 'sm' : 'md'}
					on:click={() => showCompose = true}
				>
					<PenSquare class="h-4 w-4 mr-2" />
					<span class="hidden sm:inline">Compose</span>
					<span class="sm:hidden">New</span>
				</ShimmerButton>
			{/if}
		</div>
	</BlurFade>

	<!-- Filters -->
	<BlurFade delay={100}>
		<Card.Root>
			<Card.Content class="p-3 md:p-4">
				<div class="flex flex-col sm:flex-row gap-3 md:gap-4">
					<!-- Project Selector (Searchable Combobox) -->
					<div class="flex-1 space-y-1.5">
						<Label class="text-sm font-medium">Project</Label>
						<FilterCombobox
							value={getProjectHumanKey(selectedProject)}
							onValueChange={handleProjectChange}
							options={projectOptions}
							placeholder="Select a project..."
							searchPlaceholder="Search projects..."
							emptyMessage="No projects found."
						/>
					</div>

					<!-- Agent Selector (Searchable Combobox) -->
					<div class="flex-1 space-y-1.5">
						<Label class="text-sm font-medium">Agent</Label>
						<FilterCombobox
							value={selectedAgent}
							onValueChange={handleAgentChange}
							options={agentOptions}
							placeholder="Select an agent..."
							searchPlaceholder="Search agents..."
							emptyMessage="No agents found."
						/>
					</div>

					<!-- Refresh Button -->
					{#if selectedProject && selectedAgent}
						<div class="flex items-end">
							<Button
								variant="outline"
								onclick={loadMessages}
								disabled={loadingMessages}
								class="w-full sm:w-auto gap-2"
							>
								<RefreshCw class="h-4 w-4 {loadingMessages ? 'animate-spin' : ''}" />
								<span>Refresh</span>
							</Button>
						</div>
					{/if}
				</div>
			</Card.Content>
		</Card.Root>
	</BlurFade>

	<!-- Error Message -->
	{#if error}
		<BlurFade delay={200}>
			<div class="bg-destructive/10 border border-destructive/30 rounded-xl p-4">
				<p class="text-destructive">{error}</p>
			</div>
		</BlurFade>
	{/if}

	<!-- Loading State -->
	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
		</div>
	{:else if !selectedProject || !selectedAgent}
		<!-- Selection Prompt -->
		<BlurFade delay={200}>
			<Card.Root class="p-8 md:p-12">
				<EmptyState
					title="Select a Project and Agent"
					description="Choose a project and agent above to view their inbox."
				>
					{#snippet icon()}
						<Inbox class="h-12 w-12" />
					{/snippet}
				</EmptyState>
			</Card.Root>
		</BlurFade>
	{:else if loadingMessages}
		<MessageListSkeleton rows={5} />
	{:else if messages.length === 0}
		<!-- Empty Inbox -->
		<BlurFade delay={200}>
			<Card.Root class="p-8 md:p-12">
				<EmptyState
					title="Inbox is empty"
					description="No messages for {selectedAgent} yet."
					actionLabel="Send a Message"
					onAction={() => showCompose = true}
				>
					{#snippet icon()}
						<Inbox class="h-12 w-12" />
					{/snippet}
				</EmptyState>
			</Card.Root>
		</BlurFade>
	{:else}
		<!-- Messages List -->
		<BlurFade delay={200}>
			<Card.Root class="overflow-hidden">
				<div class="p-3 md:p-4 border-b border-border flex items-center justify-between">
					<span class="text-sm text-muted-foreground">
						{messages.length} message{messages.length === 1 ? '' : 's'}
					</span>
				</div>
				<ul class="divide-y divide-border" role="list">
					{#each messages as message, index}
						<li
							class="animate-in fade-in slide-in-from-bottom-2"
							style="animation-delay: calc({index} * var(--delay-stagger)); animation-fill-mode: both;"
						>
							<a
								href="/inbox/{message.id}?project={selectedProject}&agent={selectedAgent}"
								class="block min-h-[72px] p-3 md:p-4 hover:bg-muted/50 transition-colors active:bg-muted"
							>
								<div class="flex items-start justify-between gap-3 md:gap-4">
									<div class="flex-1 min-w-0">
										<div class="flex flex-wrap items-center gap-1.5 md:gap-2 mb-1">
											<h4 class="font-medium text-foreground truncate text-sm md:text-base">
												{message.subject || '(No subject)'}
											</h4>
											{#if message.importance !== 'normal'}
												<Badge variant={getImportanceVariant(message.importance)}>
													{message.importance}
												</Badge>
											{/if}
											{#if message.ack_required}
												<Badge variant="outline" class="border-amber-500 text-amber-600 dark:text-amber-400">
													ACK
												</Badge>
											{/if}
											{#if message.thread_id}
												<Badge variant="secondary">
													Thread
												</Badge>
											{/if}
										</div>
										<p class="text-xs md:text-sm text-muted-foreground truncate">
											{truncateBody(message.body_md)}
										</p>
									</div>
									<div class="text-xs md:text-sm text-muted-foreground whitespace-nowrap shrink-0">
										{formatDate(message.created_ts)}
									</div>
								</div>
							</a>
						</li>
					{/each}
				</ul>
			</Card.Root>
		</BlurFade>
	{/if}
</div>

<!-- Compose - Sheet on mobile, Dialog on desktop -->
{#if isMobile}
	<Sheet.Root bind:open={showCompose}>
		<Sheet.Content side="bottom" class="h-[90vh] rounded-t-xl">
			<Sheet.Header class="pb-4">
				<Sheet.Title>New Message</Sheet.Title>
				<Sheet.Description>
					Send a message from {selectedAgent}
				</Sheet.Description>
			</Sheet.Header>
			<div class="flex-1 overflow-y-auto">
				<ComposeMessage
					projectSlug={selectedProject}
					senderName={selectedAgent}
					{agents}
					onClose={() => showCompose = false}
					onSent={handleMessageSent}
				/>
			</div>
		</Sheet.Content>
	</Sheet.Root>
{:else}
	<Dialog.Root bind:open={showCompose}>
		<Dialog.Content class="max-w-2xl">
			<Dialog.Header>
				<Dialog.Title>New Message</Dialog.Title>
				<Dialog.Description>
					Send a message from {selectedAgent}
				</Dialog.Description>
			</Dialog.Header>
			<ComposeMessage
				projectSlug={selectedProject}
				senderName={selectedAgent}
				{agents}
				onClose={() => showCompose = false}
				onSent={handleMessageSent}
			/>
		</Dialog.Content>
	</Dialog.Root>
{/if}

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
