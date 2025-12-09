<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { getMessage, getAgents, type Message, type Agent } from '$lib/api/client';
	import ComposeMessage from '$lib/components/ComposeMessage.svelte';

	let message = $state<Message | null>(null);
	let agents = $state<Agent[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Reply modal
	let showReply = $state(false);

	// Get context from URL params
	let projectSlug = $derived($page.url.searchParams.get('project') || '');
	let agentName = $derived($page.url.searchParams.get('agent') || '');
	let messageId = $derived(parseInt($page.params.id));

	onMount(async () => {
		await loadMessage();
		if (projectSlug) {
			await loadAgents();
		}
	});

	async function loadMessage() {
		loading = true;
		error = null;
		try {
			message = await getMessage(messageId);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load message';
		} finally {
			loading = false;
		}
	}

	async function loadAgents() {
		try {
			agents = await getAgents(projectSlug);
		} catch (e) {
			// Silently fail - reply functionality won't work but message display will
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleString('en-US', {
			weekday: 'short',
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
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

	function goBack() {
		const params = new URLSearchParams();
		if (projectSlug) params.set('project', projectSlug);
		if (agentName) params.set('agent', agentName);
		goto(`/inbox?${params.toString()}`);
	}
</script>

<div class="space-y-6">
	<!-- Breadcrumb / Back -->
	<nav class="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
		<button onclick={goBack} class="hover:text-primary-600 dark:hover:text-primary-400 flex items-center gap-1">
			<span>←</span>
			<span>Back to Inbox</span>
		</button>
		{#if agentName}
			<span>/</span>
			<span class="text-gray-900 dark:text-white font-medium">{agentName}</span>
		{/if}
	</nav>

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
	{:else if message}
		<!-- Message View -->
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
			<!-- Message Header -->
			<div class="p-6 border-b border-gray-200 dark:border-gray-700">
				<div class="flex items-start justify-between gap-4">
					<div class="flex-1">
						<h1 class="text-xl font-bold text-gray-900 dark:text-white mb-2">
							{message.subject || '(No subject)'}
						</h1>
						<div class="flex flex-wrap items-center gap-2 text-sm">
							{#if message.importance !== 'normal'}
								<span class="px-2 py-0.5 rounded-full {getImportanceBadge(message.importance)}">
									{message.importance} priority
								</span>
							{/if}
							{#if message.ack_required}
								<span class="px-2 py-0.5 rounded-full bg-amber-100 dark:bg-amber-900 text-amber-700 dark:text-amber-300">
									Acknowledgment required
								</span>
							{/if}
							{#if message.thread_id}
								<span class="px-2 py-0.5 rounded-full bg-purple-100 dark:bg-purple-900 text-purple-700 dark:text-purple-300">
									Thread: {message.thread_id}
								</span>
							{/if}
						</div>
					</div>
					{#if projectSlug && agentName && agents.length > 0}
						<button
							onclick={() => showReply = true}
							class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors flex items-center gap-2"
						>
							<span>↩️</span>
							<span>Reply</span>
						</button>
					{/if}
				</div>

				<div class="mt-4 text-sm text-gray-600 dark:text-gray-400">
					<div class="flex items-center gap-4">
						<span>Received: {formatDate(message.created_ts)}</span>
					</div>
				</div>
			</div>

			<!-- Message Body -->
			<div class="p-6">
				<div class="prose dark:prose-invert max-w-none">
					<!-- Render markdown as plain text for now - could add markdown rendering later -->
					<pre class="whitespace-pre-wrap font-sans text-gray-700 dark:text-gray-300 bg-transparent p-0 overflow-visible">{message.body_md}</pre>
				</div>
			</div>

			<!-- Message Metadata -->
			<div class="p-6 bg-gray-50 dark:bg-gray-700/50 border-t border-gray-200 dark:border-gray-700">
				<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">Message Details</h3>
				<dl class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
					<div>
						<dt class="text-gray-500 dark:text-gray-400">Message ID</dt>
						<dd class="font-mono text-gray-900 dark:text-white">{message.id}</dd>
					</div>
					<div>
						<dt class="text-gray-500 dark:text-gray-400">Project ID</dt>
						<dd class="font-mono text-gray-900 dark:text-white">{message.project_id}</dd>
					</div>
					<div>
						<dt class="text-gray-500 dark:text-gray-400">Sender ID</dt>
						<dd class="font-mono text-gray-900 dark:text-white">{message.sender_id}</dd>
					</div>
					<div>
						<dt class="text-gray-500 dark:text-gray-400">Thread ID</dt>
						<dd class="font-mono text-gray-900 dark:text-white">{message.thread_id || 'None'}</dd>
					</div>
				</dl>
			</div>
		</div>

		<!-- Quick Actions -->
		<div class="flex items-center gap-3">
			<button
				onclick={goBack}
				class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
			>
				← Back to Inbox
			</button>
			{#if projectSlug && agentName && agents.length > 0}
				<button
					onclick={() => showReply = true}
					class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
				>
					Reply to Message
				</button>
			{/if}
		</div>
	{:else}
		<!-- Not Found -->
		<div class="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm border border-gray-200 dark:border-gray-700">
			<div class="text-4xl mb-4">📭</div>
			<h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">Message not found</h3>
			<p class="text-gray-600 dark:text-gray-400 mb-4">
				The message you're looking for doesn't exist or has been deleted.
			</p>
			<button
				onclick={goBack}
				class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
			>
				Back to Inbox
			</button>
		</div>
	{/if}
</div>

<!-- Reply Modal -->
{#if showReply && message}
	<div class="fixed inset-0 bg-black/50 flex items-center justify-center p-4 z-50">
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-2xl w-full max-h-[90vh] overflow-hidden">
			<ComposeMessage
				{projectSlug}
				senderName={agentName}
				{agents}
				replyTo={{
					threadId: message.thread_id || `thread-${message.id}`,
					subject: message.subject
				}}
				onClose={() => showReply = false}
				onSent={() => { showReply = false; goBack(); }}
			/>
		</div>
	</div>
{/if}
