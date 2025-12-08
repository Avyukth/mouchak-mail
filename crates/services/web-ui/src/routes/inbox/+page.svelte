<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { getProjects, getAgents, getInbox, type Project, type Agent, type Message } from '$lib/api/client';
	import ComposeMessage from '$lib/components/ComposeMessage.svelte';

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

	onMount(async () => {
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
	});

	async function loadAgentsForProject(projectSlug: string) {
		try {
			agents = await getAgents(projectSlug);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load agents';
			agents = [];
		}
	}

	async function handleProjectChange() {
		selectedAgent = '';
		messages = [];
		if (selectedProject) {
			await loadAgentsForProject(selectedProject);
			updateUrl();
		} else {
			agents = [];
		}
	}

	async function handleAgentChange() {
		updateUrl();
		if (selectedProject && selectedAgent) {
			await loadMessages();
		} else {
			messages = [];
		}
	}

	async function loadMessages() {
		if (!selectedProject || !selectedAgent) return;

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

	function getImportanceBadge(importance: string) {
		switch (importance) {
			case 'high':
				return 'bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300';
			case 'low':
				return 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400';
			default:
				return 'bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300';
		}
	}

	function truncateBody(body: string, maxLength: number = 100): string {
		if (body.length <= maxLength) return body;
		return body.substring(0, maxLength) + '...';
	}
</script>

<div class="space-y-6">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold text-gray-900 dark:text-white">Inbox</h1>
			<p class="text-gray-600 dark:text-gray-400">View messages for your agents</p>
		</div>
		{#if selectedProject && selectedAgent}
			<button
				onclick={() => showCompose = true}
				class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors flex items-center gap-2"
			>
				<span class="text-lg">✉️</span>
				<span>Compose</span>
			</button>
		{/if}
	</div>

	<!-- Filters -->
	<div class="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm border border-gray-200 dark:border-gray-700">
		<div class="flex flex-col md:flex-row gap-4">
			<!-- Project Selector -->
			<div class="flex-1">
				<label for="projectSelect" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
					Project
				</label>
				<select
					id="projectSelect"
					bind:value={selectedProject}
					onchange={handleProjectChange}
					class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
				>
					<option value="">Select a project...</option>
					{#each projects as project}
						<option value={project.slug}>{project.slug}</option>
					{/each}
				</select>
			</div>

			<!-- Agent Selector -->
			<div class="flex-1">
				<label for="agentSelect" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
					Agent
				</label>
				<select
					id="agentSelect"
					bind:value={selectedAgent}
					onchange={handleAgentChange}
					disabled={!selectedProject || agents.length === 0}
					class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed"
				>
					<option value="">Select an agent...</option>
					{#each agents as agent}
						<option value={agent.name}>{agent.name}</option>
					{/each}
				</select>
			</div>

			<!-- Refresh Button -->
			{#if selectedProject && selectedAgent}
				<div class="flex items-end">
					<button
						onclick={loadMessages}
						disabled={loadingMessages}
						class="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors disabled:opacity-50"
					>
						🔄 Refresh
					</button>
				</div>
			{/if}
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
		<div class="flex items-center justify-center py-12">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
		</div>
	{:else if !selectedProject || !selectedAgent}
		<!-- Selection Prompt -->
		<div class="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm border border-gray-200 dark:border-gray-700">
			<div class="text-4xl mb-4">📬</div>
			<h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">Select an Agent</h3>
			<p class="text-gray-600 dark:text-gray-400">
				Choose a project and agent to view their inbox.
			</p>
		</div>
	{:else if loadingMessages}
		<div class="flex items-center justify-center py-12">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
		</div>
	{:else if messages.length === 0}
		<!-- Empty Inbox -->
		<div class="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm border border-gray-200 dark:border-gray-700">
			<div class="text-4xl mb-4">📭</div>
			<h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">Inbox is empty</h3>
			<p class="text-gray-600 dark:text-gray-400 mb-4">
				No messages for {selectedAgent} yet.
			</p>
			<button
				onclick={() => showCompose = true}
				class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
			>
				Send a Message
			</button>
		</div>
	{:else}
		<!-- Messages List -->
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
			<div class="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
				<span class="text-sm text-gray-600 dark:text-gray-400">
					{messages.length} message{messages.length === 1 ? '' : 's'}
				</span>
			</div>
			<ul class="divide-y divide-gray-200 dark:divide-gray-700">
				{#each messages as message}
					<li>
						<a
							href="/inbox/{message.id}?project={selectedProject}&agent={selectedAgent}"
							class="block p-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
						>
							<div class="flex items-start justify-between gap-4">
								<div class="flex-1 min-w-0">
									<div class="flex items-center gap-2 mb-1">
										<h4 class="font-medium text-gray-900 dark:text-white truncate">
											{message.subject || '(No subject)'}
										</h4>
										{#if message.importance !== 'normal'}
											<span class="px-2 py-0.5 text-xs rounded-full {getImportanceBadge(message.importance)}">
												{message.importance}
											</span>
										{/if}
										{#if message.ack_required}
											<span class="px-2 py-0.5 text-xs rounded-full bg-amber-100 dark:bg-amber-900 text-amber-700 dark:text-amber-300">
												ACK
											</span>
										{/if}
										{#if message.thread_id}
											<span class="text-xs text-gray-400" title="Part of a thread">
												🧵
											</span>
										{/if}
									</div>
									<p class="text-sm text-gray-600 dark:text-gray-400 truncate">
										{truncateBody(message.body_md)}
									</p>
								</div>
								<div class="text-sm text-gray-500 dark:text-gray-400 whitespace-nowrap">
									{formatDate(message.created_ts)}
								</div>
							</div>
						</a>
					</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>

<!-- Compose Modal -->
{#if showCompose}
	<div class="fixed inset-0 bg-black/50 flex items-center justify-center p-4 z-50">
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-2xl w-full max-h-[90vh] overflow-hidden">
			<ComposeMessage
				projectSlug={selectedProject}
				senderName={selectedAgent}
				{agents}
				onClose={() => showCompose = false}
				onSent={() => { showCompose = false; loadMessages(); }}
			/>
		</div>
	</div>
{/if}
