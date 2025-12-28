<script lang="ts">
	import { browser } from '$app/environment';
	import { slide } from 'svelte/transition';
	import { get } from 'svelte/store';
	import { dataProvider } from '$lib/data';
	import {
		allMessages,
		autoRefreshEnabled,
		filters,
		filtersActive,
		filteredMessages,
		isFullscreen,
		isRefreshing,
		searchQuery,
		selectedMessage,
		selectedMessages,
		showFilters,
		uniqueProjects,
		uniqueRecipients,
		uniqueSenders,
		viewMode,
		toggleSelectAll,
		clearFilters,
		selectNextMessage,
		selectPreviousMessage,
		markMessagesAsRead
	} from '$lib/stores/unifiedInbox';

	// shadcn/ui components
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import { FilterCombobox } from '$lib/components/ui/combobox/index.js';

	// Icons
	import Search from 'lucide-svelte/icons/search';
	import Filter from 'lucide-svelte/icons/filter';
	import LayoutGrid from 'lucide-svelte/icons/layout-grid';
	import List from 'lucide-svelte/icons/list';
	import Maximize from 'lucide-svelte/icons/maximize';
	import Minimize from 'lucide-svelte/icons/minimize';
	import RefreshCw from 'lucide-svelte/icons/refresh-cw';
	import Copy from 'lucide-svelte/icons/copy';
	import ExternalLink from 'lucide-svelte/icons/external-link';
	import X from 'lucide-svelte/icons/x';
	import CheckSquare from 'lucide-svelte/icons/check-square';
	import Mail from 'lucide-svelte/icons/mail';
	import MailOpen from 'lucide-svelte/icons/mail-open';
	import Inbox from 'lucide-svelte/icons/inbox';

	let loading = $state(true);
	let error = $state<string | null>(null);
	let searchInput = $state<HTMLInputElement | null>(null);
	let refreshTimer: ReturnType<typeof setInterval> | null = null;

	$effect(() => {
		if (!browser) return;
		void loadMessages(true);
	});

	$effect(() => {
		if (!browser) return;
		if (refreshTimer) {
			clearInterval(refreshTimer);
			refreshTimer = null;
		}
		if ($autoRefreshEnabled) {
			refreshTimer = setInterval(() => {
				void loadMessages(false);
			}, 45000);
		}
		return () => {
			if (refreshTimer) {
				clearInterval(refreshTimer);
				refreshTimer = null;
			}
		};
	});

	$effect(() => {
		const messages = $filteredMessages;
		const current = $selectedMessage;
		if (messages.length === 0) {
			if (current) {
				selectedMessage.set(null);
			}
			return;
		}
		if (!current || !messages.some((message) => message.id === current.id)) {
			selectedMessage.set(messages[0]);
		}
	});

	$effect(() => {
		if (!browser) return;
		const handler = (event: KeyboardEvent) => {
			const target = event.target as HTMLElement | null;
			const isTypingTarget =
				target?.tagName === 'INPUT' ||
				target?.tagName === 'TEXTAREA' ||
				target?.isContentEditable === true;

			if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
				event.preventDefault();
				searchInput?.focus();
				return;
			}

			if (event.key === '/' && !isTypingTarget) {
				event.preventDefault();
				searchInput?.focus();
				return;
			}

			if (isTypingTarget) {
				return;
			}

			switch (event.key) {
				case 'j':
					selectNextMessage();
					break;
				case 'k':
					selectPreviousMessage();
					break;
				case 'f':
					isFullscreen.update((value) => !value);
					break;
				case 'Escape':
					if (get(isFullscreen)) {
						isFullscreen.set(false);
					} else {
						selectedMessage.set(null);
						selectedMessages.set([]);
					}
					break;
				default:
					break;
			}
		};

		window.addEventListener('keydown', handler);
		return () => window.removeEventListener('keydown', handler);
	});

	async function loadMessages(initial: boolean) {
		if (get(isRefreshing)) return;
		if (initial) {
			loading = true;
		}
		error = null;
		isRefreshing.set(true);
		try {
			const response = await dataProvider.fetchUnifiedInbox();
			allMessages.set(response.messages ?? []);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load unified inbox';
		} finally {
			isRefreshing.set(false);
			loading = false;
		}
	}

	function formatTimestamp(message: { created_relative?: string; created_ts?: string }): string {
		if (message.created_relative) return message.created_relative;
		if (!message.created_ts) return '';
		return new Date(message.created_ts).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric'
		});
	}

	function formatRecipients(message: { recipients?: string[]; recipient_names?: string[] }): string {
		const recipients = message.recipients ?? message.recipient_names ?? [];
		if (recipients.length === 0) return 'No recipients';
		return recipients.join(', ');
	}

	function toggleMessageSelection(id: number) {
		selectedMessages.update((current) =>
			current.includes(id) ? current.filter((value) => value !== id) : [...current, id]
		);
	}

	async function markSelectedRead() {
		const ids = get(selectedMessages);
		if (ids.length === 0) return;
		selectedMessages.set([]);
		await markMessagesAsRead(ids);
	}

	function setFilter(key: 'project' | 'sender' | 'recipient' | 'importance' | 'hasThread', value: string) {
		filters.update((current) => ({ ...current, [key]: value }));
	}

	function clearAllFilters() {
		searchQuery.set('');
		clearFilters();
	}

	function copyMessageBody() {
		const message = get(selectedMessage);
		if (!message || !browser) return;
		navigator.clipboard?.writeText(message.body_md ?? '');
	}

	function openMessage() {
		const message = get(selectedMessage);
		if (!message) return;
		const params = new URLSearchParams();
		params.set('project', message.project_slug);
		const url = `/inbox/${message.id}?${params.toString()}`;
		window.open(url, '_blank');
	}

	function getImportanceVariant(importance?: string): 'default' | 'destructive' | 'secondary' | 'outline' {
		switch (importance) {
			case 'high':
				return 'destructive';
			case 'low':
				return 'secondary';
			default:
				return 'outline';
		}
	}
</script>

<div
	data-testid="mail-root"
	data-fullscreen={$isFullscreen}
	class={$isFullscreen ? 'fixed inset-0 z-40 bg-background p-6 overflow-auto' : 'pt-4 md:pt-6 pb-4 md:pb-6 space-y-4'}
>
	{#if !$isFullscreen}
		<div>
			<h1 class="text-xl md:text-2xl font-bold text-foreground">Unified Inbox</h1>
			<p class="text-sm text-muted-foreground">All messages across projects in one place</p>
		</div>
	{/if}

	<div class={$isFullscreen
		? 'pb-4 border-b border-border'
		: 'sticky top-0 z-20 -mx-4 md:-mx-6 px-4 md:px-6 py-3 bg-background border-b border-border'}>
		<div class="flex flex-wrap items-center gap-3">
			<div class="flex-1 min-w-[200px]">
				<div class="relative">
					<Search class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
					<Input
						bind:ref={searchInput}
						bind:value={$searchQuery}
						data-testid="mail-search-input"
						aria-label="Search mail"
						placeholder="Search mail..."
						class="pl-10 h-9"
					/>
				</div>
			</div>

			<div class="flex items-center gap-2">
				<Button
					variant="outline"
					size="sm"
					onclick={() => showFilters.update((value) => !value)}
				>
					<Filter class="h-4 w-4" />
					{$showFilters ? 'Hide' : 'Filters'}
				</Button>
				<Button
					variant="outline"
					size="sm"
					onclick={() => viewMode.update((mode) => (mode === 'split' ? 'list' : 'split'))}
				>
					{#if $viewMode === 'split'}
						<List class="h-4 w-4" />
					{:else}
						<LayoutGrid class="h-4 w-4" />
					{/if}
				</Button>
				<Button
					variant="outline"
					size="sm"
					onclick={() => isFullscreen.update((value) => !value)}
				>
					{#if $isFullscreen}
						<Minimize class="h-4 w-4" />
					{:else}
						<Maximize class="h-4 w-4" />
					{/if}
				</Button>
				<Button
					variant="outline"
					size="sm"
					onclick={() => loadMessages(false)}
					disabled={$isRefreshing}
				>
					<RefreshCw class="h-4 w-4 {$isRefreshing ? 'animate-spin' : ''}" />
				</Button>
			</div>
		</div>

		<div class="flex items-center gap-4 mt-2 text-xs text-muted-foreground">
			<label class="flex items-center gap-2 cursor-pointer">
				<Checkbox
					checked={$autoRefreshEnabled}
					onCheckedChange={(checked) => autoRefreshEnabled.set(checked === true)}
				/>
				<span>Auto-refresh</span>
			</label>
			<span class="hidden sm:inline text-muted-foreground/60">
				<kbd class="px-1 py-0.5 bg-muted rounded text-2xs">j</kbd>/<kbd class="px-1 py-0.5 bg-muted rounded text-2xs">k</kbd> nav
			</span>
		</div>
	</div>
	{#if $showFilters}
		<div
			class="bg-card border border-border rounded-xl p-4 shadow-sm"
			transition:slide={{ duration: 180 }}
		>
			<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-4">
				<!-- Project Filter (Searchable Combobox) -->
				<div class="space-y-1.5">
					<Label class="text-xs font-medium">Project</Label>
					<FilterCombobox
						value={$filters.project}
						onValueChange={(value) => setFilter('project', value)}
						options={$uniqueProjects}
						placeholder="All projects"
						searchPlaceholder="Search projects..."
						emptyMessage="No projects found."
					/>
				</div>

				<!-- Sender Filter (Searchable Combobox) -->
				<div class="space-y-1.5">
					<Label class="text-xs font-medium">Sender</Label>
					<FilterCombobox
						value={$filters.sender}
						onValueChange={(value) => setFilter('sender', value)}
						options={$uniqueSenders}
						placeholder="All senders"
						searchPlaceholder="Search senders..."
						emptyMessage="No senders found."
					/>
				</div>

				<!-- Recipient Filter (Searchable Combobox) -->
				<div class="space-y-1.5">
					<Label class="text-xs font-medium">Recipient</Label>
					<FilterCombobox
						value={$filters.recipient}
						onValueChange={(value) => setFilter('recipient', value)}
						options={$uniqueRecipients}
						placeholder="All recipients"
						searchPlaceholder="Search recipients..."
						emptyMessage="No recipients found."
					/>
				</div>

				<!-- Importance Filter (Select) -->
				<div class="space-y-1.5">
					<Label class="text-xs font-medium">Importance</Label>
					<Select.Root
						type="single"
						value={$filters.importance}
						onValueChange={(value) => setFilter('importance', value ?? '')}
					>
						<Select.Trigger class="w-full">
							{$filters.importance || 'All'}
						</Select.Trigger>
						<Select.Content>
							<Select.Item value="">All</Select.Item>
							<Select.Item value="high">High</Select.Item>
							<Select.Item value="normal">Normal</Select.Item>
							<Select.Item value="low">Low</Select.Item>
						</Select.Content>
					</Select.Root>
				</div>

				<!-- Thread Filter (Select) -->
				<div class="space-y-1.5">
					<Label class="text-xs font-medium">Thread</Label>
					<Select.Root
						type="single"
						value={$filters.hasThread}
						onValueChange={(value) => setFilter('hasThread', value ?? '')}
					>
						<Select.Trigger class="w-full">
							{$filters.hasThread === 'yes' ? 'Has thread' : $filters.hasThread === 'no' ? 'No thread' : 'All'}
						</Select.Trigger>
						<Select.Content>
							<Select.Item value="">All</Select.Item>
							<Select.Item value="yes">Has thread</Select.Item>
							<Select.Item value="no">No thread</Select.Item>
						</Select.Content>
					</Select.Root>
				</div>
			</div>

			<div class="mt-4 flex items-center gap-3">
				<Button
					variant="outline"
					size="sm"
					onclick={clearAllFilters}
					disabled={!$filtersActive && !$searchQuery}
				>
					<X class="h-4 w-4" />
					Clear filters
				</Button>
				<span class="text-xs text-muted-foreground">{$filteredMessages.length} messages</span>
			</div>
		</div>
	{/if}

	<!-- Error Alert -->
	{#if error}
		<div class="bg-destructive/10 border border-destructive/30 rounded-xl p-4">
			<p class="text-destructive">{error}</p>
		</div>
	{/if}

	<!-- Loading State -->
	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
		</div>
	{:else}
		<!-- Mail Grid -->
		<div
			class={`grid gap-4 ${
				$viewMode === 'split'
					? 'lg:grid-cols-[minmax(0,35%)_minmax(0,65%)]'
					: 'grid-cols-1'
			}`}
		>
			<!-- Message List -->
			<div
				data-testid="mail-list"
				class="bg-card border border-border rounded-xl overflow-hidden"
			>
				<div class="flex items-center justify-between px-4 py-3 border-b border-border">
					<div class="flex items-center gap-3">
						<Button variant="ghost" size="sm" onclick={toggleSelectAll}>
							<CheckSquare class="h-4 w-4" />
							Select all
						</Button>
						<Button
							variant="ghost"
							size="sm"
							onclick={markSelectedRead}
							disabled={$selectedMessages.length === 0}
						>
							<Mail class="h-4 w-4" />
							Mark read
						</Button>
					</div>
					<Badge variant="secondary">{$selectedMessages.length} selected</Badge>
				</div>

				{#if $filteredMessages.length === 0}
					<div class="p-8 text-center text-sm text-muted-foreground">
						<Inbox class="h-12 w-12 mx-auto text-muted-foreground/50" />
						<p class="mt-4 font-medium">No messages match this view.</p>
						<p class="mt-1">Try adjusting filters or wait for new mail.</p>
					</div>
				{:else}
					<div class="divide-y divide-border overflow-y-auto max-h-[calc(100vh-20rem)]">
						{#each $filteredMessages as message}
							<button
								data-testid={`mail-item-${message.id}`}
								onclick={() => selectedMessage.set(message)}
								class={`w-full text-left px-4 py-3 hover:bg-muted/50 transition ${
									$selectedMessage?.id === message.id
										? 'bg-muted/70'
										: ''
								} ${message.is_read ? 'opacity-70' : ''}`}
							>
								<div class="flex items-start gap-3">
									<Checkbox
										checked={$selectedMessages.includes(message.id)}
										onCheckedChange={() => toggleMessageSelection(message.id)}
										onclick={(event: MouseEvent) => event.stopPropagation()}
										class="mt-1"
									/>
									<div class="flex-1 min-w-0">
										<div class="flex items-center justify-between gap-2">
											<div class="min-w-0">
												<p class="text-sm font-semibold text-foreground truncate">
													{message.sender_name}
													<span class="text-xs font-normal text-muted-foreground">
														→ {formatRecipients(message)}
													</span>
												</p>
												<div class="flex flex-wrap items-center gap-1.5 mt-1">
													<Badge variant="secondary" class="text-2xs px-1.5 py-0">
														{message.project_slug}
													</Badge>
													{#if message.importance && message.importance !== 'normal'}
														<Badge variant={getImportanceVariant(message.importance)} class="text-2xs px-1.5 py-0">
															{message.importance}
														</Badge>
													{/if}
													{#if message.thread_id}
														<Badge variant="outline" class="text-2xs px-1.5 py-0 border-purple-500/50 text-purple-600 dark:text-purple-400">
															Thread
														</Badge>
													{/if}
												</div>
											</div>
											<span class="text-xs text-muted-foreground whitespace-nowrap">
												{formatTimestamp(message)}
											</span>
										</div>
										<p class="mt-2 text-sm font-medium text-foreground truncate">
											{message.subject || '(No subject)'}
										</p>
										<p class="mt-1 text-xs text-muted-foreground line-clamp-2">{message.excerpt ?? message.body_md ?? ''}</p>
									</div>
								</div>
							</button>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Message Detail -->
			<div
				data-testid="mail-detail"
				class="bg-card border border-border rounded-xl p-6 overflow-y-auto max-h-[calc(100vh-16rem)]"
			>
				{#if $selectedMessage}
					<div class="flex items-start justify-between gap-4">
						<div class="min-w-0">
							<h2 data-testid="mail-detail-subject" class="text-xl font-semibold text-foreground">
								{$selectedMessage.subject || '(No subject)'}
							</h2>
							<p class="mt-1 text-sm text-muted-foreground">
								From {$selectedMessage.sender_name} • {formatTimestamp($selectedMessage)}
							</p>
						</div>
						<div class="flex items-center gap-2 shrink-0">
							<Button variant="outline" size="sm" onclick={copyMessageBody}>
								<Copy class="h-4 w-4" />
								Copy
							</Button>
							<Button variant="outline" size="sm" onclick={openMessage}>
								<ExternalLink class="h-4 w-4" />
								Open
							</Button>
						</div>
					</div>

					<div class="mt-6 grid grid-cols-2 gap-4 text-xs text-muted-foreground">
						<div>
							<p class="font-medium text-foreground">Project</p>
							<p>{$selectedMessage.project_slug}</p>
						</div>
						<div>
							<p class="font-medium text-foreground">Recipients</p>
							<p>{formatRecipients($selectedMessage)}</p>
						</div>
						<div>
							<p class="font-medium text-foreground">Importance</p>
							<Badge variant={getImportanceVariant($selectedMessage.importance)} class="mt-0.5">
								{$selectedMessage.importance ?? 'normal'}
							</Badge>
						</div>
						<div>
							<p class="font-medium text-foreground">Thread</p>
							<p>{$selectedMessage.thread_id ?? 'None'}</p>
						</div>
					</div>

					<div class="mt-6">
						<h3 class="text-sm font-semibold text-foreground">Message</h3>
						<pre class="mt-2 whitespace-pre-wrap text-sm text-muted-foreground bg-muted/30 rounded-lg p-4 overflow-x-auto">{$selectedMessage.body_md ?? ''}</pre>
					</div>
				{:else}
					<div data-testid="mail-empty-detail" class="text-center text-sm text-muted-foreground py-12">
						<MailOpen class="h-12 w-12 mx-auto text-muted-foreground/50" />
						<p class="mt-4 font-medium">Select a message to see details.</p>
						<p class="mt-1">Use <kbd class="px-1 py-0.5 bg-muted rounded text-2xs">j</kbd>/<kbd class="px-1 py-0.5 bg-muted rounded text-2xs">k</kbd> to move through the list.</p>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>
