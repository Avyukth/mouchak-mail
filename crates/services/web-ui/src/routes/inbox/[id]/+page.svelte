<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { getMessage, getAgents, getThreadMessages, type Message, type Agent } from '$lib/api/client';
	import ComposeMessage from '$lib/components/ComposeMessage.svelte';

	// shadcn/ui components
	import { Button } from '$lib/components/ui/button/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import * as Sheet from '$lib/components/ui/sheet/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { BlurFade, ShimmerButton } from '$lib/components/magic';

	// Icons
	import ArrowLeft from 'lucide-svelte/icons/arrow-left';
	import Reply from 'lucide-svelte/icons/reply';
	import Inbox from 'lucide-svelte/icons/inbox';
	import Clock from 'lucide-svelte/icons/clock';
	import MessageSquare from 'lucide-svelte/icons/message-square';
	import Info from 'lucide-svelte/icons/info';

	import { marked } from 'marked';

	let message = $state<Message | null>(null);
	let threadMessages = $state<Message[]>([]);
	let agents = $state<Agent[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Reply modal
	let showReply = $state(false);

	// Mobile detection
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

	// Get context from URL params
	let projectSlug = $derived($page.url.searchParams.get('project') || '');
	let agentName = $derived($page.url.searchParams.get('agent') || '');
	let messageId = $derived(parseInt($page.params.id ?? '0'));

	// Use $effect for client-side data loading in Svelte 5
	$effect(() => {
		if (browser) {
			initPage();
		}
	});

	async function initPage() {
		await loadMessage();
		if (projectSlug) {
			await loadAgents();
		}
	}

	async function loadMessage() {
		loading = true;
		error = null;
		try {
			message = await getMessage(messageId);
			// Load thread messages if this message is part of a thread
			if (message?.thread_id && projectSlug && agentName) {
				try {
					threadMessages = await getThreadMessages(projectSlug, agentName, message.thread_id);
				} catch {
					// Thread loading failed - just show single message
					threadMessages = [message];
				}
			} else if (message) {
				threadMessages = [message];
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load message';
		} finally {
			loading = false;
		}
	}

	async function loadAgents() {
		try {
			agents = await getAgents(projectSlug);
		} catch {
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

	function formatRelativeTime(dateStr: string): string {
		const date = new Date(dateStr);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffMins = Math.floor(diffMs / (1000 * 60));
		const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
		const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

		if (diffMins < 60) return `${diffMins}m ago`;
		if (diffHours < 24) return `${diffHours}h ago`;
		return `${diffDays}d ago`;
	}

	function getImportanceVariant(importance: string): "default" | "secondary" | "destructive" | "outline" {
		switch (importance) {
			case 'high': return 'destructive';
			case 'low': return 'secondary';
			default: return 'default';
		}
	}

	function goBack() {
		const params = new URLSearchParams();
		if (projectSlug) params.set('project', projectSlug);
		if (agentName) params.set('agent', agentName);
		goto(`/inbox?${params.toString()}`);
	}

	function renderMarkdown(md: string): string {
		return marked.parse(md, { async: false }) as string;
	}
</script>

<div class="space-y-4 md:space-y-6">
	<!-- Breadcrumb / Back -->
	<BlurFade delay={0}>
		<nav class="flex items-center gap-2 text-sm text-muted-foreground">
			<Button
				variant="ghost"
				size="sm"
				onclick={goBack}
				class="-ml-2 gap-1"
			>
				<ArrowLeft class="h-4 w-4" />
				<span>Back to Inbox</span>
			</Button>
			{#if agentName}
				<span>/</span>
				<span class="text-foreground font-medium">{agentName}</span>
			{/if}
		</nav>
	</BlurFade>

	<!-- Error Message -->
	{#if error}
		<BlurFade delay={100}>
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
	{:else if message}
		<!-- Message View -->
		<BlurFade delay={100}>
			<Card.Root class="overflow-hidden">
				<!-- Message Header -->
				<div class="p-4 md:p-6 border-b border-border">
					<div class="flex flex-col sm:flex-row sm:items-start justify-between gap-4">
						<div class="flex-1 min-w-0">
							<h1 class="text-lg md:text-xl font-bold text-foreground mb-2 break-words">
								{message.subject || '(No subject)'}
							</h1>
							<div class="flex flex-wrap items-center gap-2 text-sm">
								{#if message.importance !== 'normal'}
									<Badge variant={getImportanceVariant(message.importance)}>
										{message.importance} priority
									</Badge>
								{/if}
								{#if message.ack_required}
									<Badge variant="outline" class="border-amber-500 text-amber-600 dark:text-amber-400">
										ACK required
									</Badge>
								{/if}
								{#if message.thread_id}
									<Badge variant="secondary">
										Thread
									</Badge>
								{/if}
							</div>
						</div>
						{#if projectSlug && agentName && agents.length > 0}
							<ShimmerButton
								size={isMobile ? 'sm' : 'md'}
								on:click={() => showReply = true}
							>
								<Reply class="h-4 w-4 mr-2" />
								Reply
							</ShimmerButton>
						{/if}
					</div>

					<div class="mt-4 flex items-center gap-2 text-sm text-muted-foreground">
						<Clock class="h-4 w-4" />
						<span>{formatDate(message.created_ts)}</span>
					</div>
				</div>

				<!-- Tabs: Message / Details / Thread -->
				<Tabs.Root value="message" class="w-full">
					<Tabs.List class="w-full justify-start border-b border-border px-4 md:px-6">
						<Tabs.Trigger value="message" class="min-h-[44px] px-4 flex items-center gap-2">
							<MessageSquare class="h-4 w-4" />
							<span class="hidden sm:inline">Message</span>
						</Tabs.Trigger>
						{#if threadMessages.length > 1}
							<Tabs.Trigger value="thread" class="min-h-[44px] px-4 flex items-center gap-2">
								<Reply class="h-4 w-4" />
								<span class="hidden sm:inline">Thread</span>
								<Badge variant="secondary" class="ml-1">
									{threadMessages.length}
								</Badge>
							</Tabs.Trigger>
						{/if}
						<Tabs.Trigger value="details" class="min-h-[44px] px-4 flex items-center gap-2">
							<Info class="h-4 w-4" />
							<span class="hidden sm:inline">Details</span>
						</Tabs.Trigger>
					</Tabs.List>

					<!-- Message Content Tab -->
					<Tabs.Content value="message" class="p-4 md:p-6">
						<article class="prose prose-sm md:prose dark:prose-invert max-w-none">
							{@html renderMarkdown(message.body_md)}
						</article>
					</Tabs.Content>

					<!-- Thread Timeline Tab -->
					{#if threadMessages.length > 1}
						<Tabs.Content value="thread" class="p-4 md:p-6">
							<div class="relative">
								<!-- Timeline line -->
								<div class="absolute left-4 top-0 bottom-0 w-0.5 bg-border" aria-hidden="true"></div>

								<ul class="space-y-4" role="list">
									{#each threadMessages as msg, index}
										<li
											class="relative pl-10 animate-in fade-in slide-in-from-left-2"
											style="animation-delay: calc({index} * var(--delay-stagger)); animation-fill-mode: both;"
										>
											<!-- Timeline dot -->
											<div
												class="absolute left-2.5 w-3 h-3 rounded-full border-2 {msg.id === message.id ? 'bg-primary border-primary' : 'bg-background border-muted-foreground/30'}"
												aria-hidden="true"
											></div>

											<div class="bg-muted/50 rounded-lg p-4 {msg.id === message.id ? 'ring-2 ring-primary' : ''}">
												<div class="flex items-center justify-between gap-2 mb-2">
													<span class="font-medium text-foreground text-sm">
														{msg.subject || '(No subject)'}
													</span>
													<span class="text-xs text-muted-foreground shrink-0">
														{formatRelativeTime(msg.created_ts)}
													</span>
												</div>
												<p class="text-sm text-muted-foreground line-clamp-2">
													{msg.body_md.slice(0, 150)}{msg.body_md.length > 150 ? '...' : ''}
												</p>
												{#if msg.id !== message.id}
													<a
														href="/inbox/{msg.id}?project={projectSlug}&agent={agentName}"
														class="inline-block mt-2 text-xs text-primary hover:underline"
													>
														View message
													</a>
												{/if}
											</div>
										</li>
									{/each}
								</ul>
							</div>
						</Tabs.Content>
					{/if}

					<!-- Details Tab -->
					<Tabs.Content value="details" class="p-4 md:p-6" data-testid="message-details">
						<dl class="grid grid-cols-1 sm:grid-cols-2 gap-4">
							<div class="bg-muted/50 rounded-lg p-4">
								<dt class="text-xs text-muted-foreground uppercase tracking-wider mb-1">From</dt>
								<dd class="text-sm text-foreground" data-testid="message-sender">{message.sender_name || agentName || 'Unknown'}</dd>
							</div>
							<div class="bg-muted/50 rounded-lg p-4">
								<dt class="text-xs text-muted-foreground uppercase tracking-wider mb-1">To</dt>
								<dd class="text-sm text-foreground">{message.recipients?.join(', ') || agentName || 'Unknown'}</dd>
							</div>
							{#if message.thread_id}
								<div class="bg-muted/50 rounded-lg p-4">
									<dt class="text-xs text-muted-foreground uppercase tracking-wider mb-1">Thread</dt>
									<dd class="text-sm text-foreground" data-testid="message-thread">{message.thread_id}</dd>
								</div>
							{/if}
							<div class="bg-muted/50 rounded-lg p-4">
								<dt class="text-xs text-muted-foreground uppercase tracking-wider mb-1">Importance</dt>
								<dd class="text-sm text-foreground capitalize">{message.importance}</dd>
							</div>
							<div class="bg-muted/50 rounded-lg p-4 sm:col-span-2">
								<dt class="text-xs text-muted-foreground uppercase tracking-wider mb-1">Sent</dt>
								<dd class="text-sm text-foreground" data-testid="message-timestamp">{formatDate(message.created_ts)}</dd>
							</div>
						</dl>
					</Tabs.Content>
				</Tabs.Root>
			</Card.Root>
		</BlurFade>
	{:else}
		<!-- Not Found -->
		<BlurFade delay={100}>
			<Card.Root class="p-8 md:p-12 text-center">
				<div class="mb-4 flex justify-center"><Inbox class="h-12 w-12 text-muted-foreground" /></div>
				<h3 class="text-lg font-semibold text-foreground mb-2">Message not found</h3>
				<p class="text-muted-foreground mb-4">
					The message you're looking for doesn't exist or has been deleted.
				</p>
				<ShimmerButton on:click={goBack}>
					Back to Inbox
				</ShimmerButton>
			</Card.Root>
		</BlurFade>
	{/if}
</div>

<!-- Reply Modal - Sheet on mobile, Dialog on desktop -->
{#if isMobile}
	<Sheet.Root bind:open={showReply}>
		<Sheet.Content side="bottom" class="h-[90vh] rounded-t-xl">
			<Sheet.Header class="pb-4">
				<Sheet.Title>Reply</Sheet.Title>
				<Sheet.Description>
					Reply from {agentName}
				</Sheet.Description>
			</Sheet.Header>
			<div class="flex-1 overflow-y-auto">
				{#if message}
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
				{/if}
			</div>
		</Sheet.Content>
	</Sheet.Root>
{:else if showReply && message}
	<Dialog.Root bind:open={showReply}>
		<Dialog.Content class="max-w-2xl">
			<Dialog.Header>
				<Dialog.Title>Reply</Dialog.Title>
				<Dialog.Description>
					Reply from {agentName}
				</Dialog.Description>
			</Dialog.Header>
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
		</Dialog.Content>
	</Dialog.Root>
{/if}

<style>
	/* Timeline animations */
	@keyframes fade-in {
		from { opacity: 0; }
		to { opacity: 1; }
	}

	@keyframes slide-in-from-left-2 {
		from { transform: translateX(-8px); }
		to { transform: translateX(0); }
	}

	.animate-in {
		animation: fade-in 300ms ease-out, slide-in-from-left-2 300ms ease-out;
	}

	/* Respect reduced motion */
	@media (prefers-reduced-motion: reduce) {
		.animate-in {
			animation: none;
		}
	}
</style>
