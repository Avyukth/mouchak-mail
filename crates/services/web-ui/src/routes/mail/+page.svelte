<script lang="ts">
	import { browser } from '$app/environment';
	import { slide } from 'svelte/transition';
	import { get } from 'svelte/store';
	import { fetchUnifiedInbox } from '$lib/api/client';
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
		selectPreviousMessage
	} from '$lib/stores/unifiedInbox';

	let loading = $state(true);
	let error = $state<string | null>(null);
	let searchInput: HTMLInputElement | null = null;
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
			const response = await fetchUnifiedInbox();
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

	function markSelectedRead() {
		const ids = get(selectedMessages);
		if (ids.length === 0) return;
		allMessages.update((messages) =>
			messages.map((message) =>
				ids.includes(message.id) ? { ...message, is_read: true } : message
			)
		);
		selectedMessages.set([]);
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

	function getImportanceBadge(importance?: string) {
		switch (importance) {
			case 'high':
				return 'bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300';
			case 'low':
				return 'bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-300';
			default:
				return 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300';
		}
	}
</script>

<div
	data-testid="mail-root"
	data-fullscreen={$isFullscreen}
	class={`space-y-4 ${$isFullscreen ? 'fixed inset-0 z-40 bg-background p-6 overflow-auto' : ''}`}
>
	<div class="flex flex-col gap-3 sticky top-0 z-20 bg-background/95 backdrop-blur -mx-6 px-6 py-4 border-b border-border">
		<div class="flex flex-wrap items-center gap-3">
			<div class="flex-1 min-w-[240px]">
				<div class="relative">
					<input
						bind:this={searchInput}
						bind:value={$searchQuery}
						data-testid="mail-search-input"
						aria-label="Search mail"
						placeholder="Search across all mail..."
						class="w-full rounded-xl border border-border bg-card px-4 py-2.5 pr-20 text-sm text-foreground placeholder:text-muted-foreground shadow-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
					/>
					<span class="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
						⌘K / /
					</span>
				</div>
			</div>

			<div class="flex flex-wrap items-center gap-2">
				<button
					onclick={() => showFilters.update((value) => !value)}
					class="px-3 py-2 text-xs font-medium rounded-lg border border-border bg-card hover:bg-muted"
				>
					{$showFilters ? 'Hide Filters' : 'Show Filters'}
				</button>
				<button
					onclick={() => viewMode.update((mode) => (mode === 'split' ? 'list' : 'split'))}
					class="px-3 py-2 text-xs font-medium rounded-lg border border-border bg-card hover:bg-muted"
				>
					{$viewMode === 'split' ? 'List View' : 'Split View'}
				</button>
				<button
					onclick={() => isFullscreen.update((value) => !value)}
					class="px-3 py-2 text-xs font-medium rounded-lg border border-border bg-card hover:bg-muted"
				>
					{$isFullscreen ? 'Exit Fullscreen' : 'Fullscreen'}
				</button>
				<button
					onclick={() => loadMessages(false)}
					class="px-3 py-2 text-xs font-medium rounded-lg border border-border bg-card hover:bg-muted"
					disabled={$isRefreshing}
				>
					{$isRefreshing ? 'Refreshing...' : 'Refresh'}
				</button>
			</div>
		</div>

		<div class="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
			<label class="flex items-center gap-2">
				<input
					type="checkbox"
					checked={$autoRefreshEnabled}
					onchange={(event) => autoRefreshEnabled.set(event.currentTarget.checked)}
					class="h-4 w-4 rounded border border-border"
				/>
				<span>Auto-refresh every 45s</span>
			</label>
			<span class="text-muted-foreground/70">Shortcut: f (fullscreen), j/k (navigate)</span>
		</div>
	</div>

	{#if $showFilters}
		<div
			class="bg-card border border-border rounded-xl p-4 shadow-sm"
			transition:slide={{ duration: 180 }}
		>
			<div class="flex flex-wrap gap-4">
				<div class="min-w-[160px] flex-1">
					<label class="text-xs font-medium text-muted-foreground">Project</label>
					<select
						class="mt-1 w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
						value={$filters.project}
						onchange={(event) => setFilter('project', event.currentTarget.value)}
					>
						<option value="">All projects</option>
						{#each $uniqueProjects as project}
							<option value={project}>{project}</option>
						{/each}
					</select>
				</div>
				<div class="min-w-[160px] flex-1">
					<label class="text-xs font-medium text-muted-foreground">Sender</label>
					<select
						class="mt-1 w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
						value={$filters.sender}
						onchange={(event) => setFilter('sender', event.currentTarget.value)}
					>
						<option value="">All senders</option>
						{#each $uniqueSenders as sender}
							<option value={sender}>{sender}</option>
						{/each}
					</select>
				</div>
				<div class="min-w-[160px] flex-1">
					<label class="text-xs font-medium text-muted-foreground">Recipient</label>
					<select
						class="mt-1 w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
						value={$filters.recipient}
						onchange={(event) => setFilter('recipient', event.currentTarget.value)}
					>
						<option value="">All recipients</option>
						{#each $uniqueRecipients as recipient}
							<option value={recipient}>{recipient}</option>
						{/each}
					</select>
				</div>
				<div class="min-w-[160px] flex-1">
					<label class="text-xs font-medium text-muted-foreground">Importance</label>
					<select
						class="mt-1 w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
						value={$filters.importance}
						onchange={(event) => setFilter('importance', event.currentTarget.value)}
					>
						<option value="">All</option>
						<option value="high">High</option>
						<option value="normal">Normal</option>
						<option value="low">Low</option>
					</select>
				</div>
				<div class="min-w-[160px] flex-1">
					<label class="text-xs font-medium text-muted-foreground">Thread</label>
					<select
						class="mt-1 w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
						value={$filters.hasThread}
						onchange={(event) => setFilter('hasThread', event.currentTarget.value)}
					>
						<option value="">All</option>
						<option value="yes">Has thread</option>
						<option value="no">No thread</option>
					</select>
				</div>
			</div>

			<div class="mt-4 flex items-center gap-3">
				<button
					onclick={clearAllFilters}
					class="px-3 py-2 text-xs font-medium rounded-lg border border-border bg-card hover:bg-muted"
					disabled={!$filtersActive && !$searchQuery}
				>
					Clear filters
				</button>
				<span class="text-xs text-muted-foreground">{$filteredMessages.length} messages</span>
			</div>
		</div>
	{/if}

	{#if error}
		<div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl p-4">
			<p class="text-red-700 dark:text-red-400">{error}</p>
		</div>
	{/if}

	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
		</div>
	{:else}
		<div
			class={`grid gap-4 ${
				$viewMode === 'split'
					? 'lg:grid-cols-[minmax(0,35%)_minmax(0,65%)]'
					: 'grid-cols-1'
			}`}
		>
			<div
				data-testid="mail-list"
				class="bg-card border border-border rounded-xl overflow-hidden"
			>
				<div class="flex items-center justify-between px-4 py-3 border-b border-border">
					<div class="flex items-center gap-3">
						<button
							onclick={toggleSelectAll}
							class="text-xs font-medium text-primary-600 hover:text-primary-700"
						>
							Select all
						</button>
						<button
							onclick={markSelectedRead}
							class="text-xs font-medium text-muted-foreground hover:text-foreground"
							disabled={$selectedMessages.length === 0}
						>
							Mark read
						</button>
					</div>
					<span class="text-xs text-muted-foreground">{$selectedMessages.length} selected</span>
				</div>

				{#if $filteredMessages.length === 0}
					<div class="p-8 text-center text-sm text-muted-foreground">
						<p class="text-lg">📭</p>
						<p class="mt-2">No messages match this view.</p>
						<p class="mt-1">Try adjusting filters or wait for new mail.</p>
					</div>
				{:else}
					<div class="divide-y divide-border">
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
									<input
										type="checkbox"
										class="mt-1 h-4 w-4 rounded border border-border"
										checked={$selectedMessages.includes(message.id)}
										onclick={(event) => event.stopPropagation()}
										onchange={() => toggleMessageSelection(message.id)}
									/>
									<div class="flex-1">
										<div class="flex items-center justify-between gap-2">
											<div>
												<p class="text-sm font-semibold text-foreground">
													{message.sender_name}
													<span class="text-xs font-normal text-muted-foreground">
														→ {formatRecipients(message)}
													</span>
												</p>
												<div class="flex flex-wrap items-center gap-2 mt-1">
													<span class="px-2 py-0.5 rounded-full text-[10px] uppercase tracking-wide bg-muted text-muted-foreground">
														{message.project_slug}
													</span>
													{#if message.importance && message.importance !== 'normal'}
														<span class={`px-2 py-0.5 rounded-full text-[10px] ${getImportanceBadge(message.importance)}`}>
															{message.importance}
														</span>
													{/if}
													{#if message.thread_id}
														<span class="px-2 py-0.5 rounded-full text-[10px] bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300">
															Thread
														</span>
													{/if}
												</div>
											</div>
											<span class="text-xs text-muted-foreground">
												{formatTimestamp(message)}
											</span>
										</div>
										<p class="mt-2 text-sm font-medium text-foreground">
											{message.subject || '(No subject)'}
										</p>
										<p class="mt-1 text-xs text-muted-foreground line-clamp-2">
											{message.excerpt ?? message.body_md ?? ''}
										</p>
									</div>
								</div>
							</button>
						{/each}
					</div>
				{/if}
			</div>

			<div
				data-testid="mail-detail"
				class="bg-card border border-border rounded-xl p-6"
			>
				{#if $selectedMessage}
					<div class="flex items-start justify-between gap-4">
						<div>
							<h2 data-testid="mail-detail-subject" class="text-xl font-semibold text-foreground">
								{$selectedMessage.subject || '(No subject)'}
							</h2>
							<p class="mt-1 text-sm text-muted-foreground">
								From {$selectedMessage.sender_name} • {formatTimestamp($selectedMessage)}
							</p>
						</div>
						<div class="flex items-center gap-2">
							<button
								onclick={copyMessageBody}
								class="px-3 py-2 text-xs font-medium rounded-lg border border-border bg-card hover:bg-muted"
							>
								Copy
							</button>
							<button
								onclick={openMessage}
								class="px-3 py-2 text-xs font-medium rounded-lg border border-border bg-card hover:bg-muted"
							>
								Open
							</button>
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
							<p>{$selectedMessage.importance ?? 'normal'}</p>
						</div>
						<div>
							<p class="font-medium text-foreground">Thread</p>
							<p>{$selectedMessage.thread_id ?? 'None'}</p>
						</div>
					</div>

					<div class="mt-6">
						<h3 class="text-sm font-semibold text-foreground">Message</h3>
						<pre class="mt-2 whitespace-pre-wrap text-sm text-muted-foreground bg-muted/30 rounded-lg p-4">
							{$selectedMessage.body_md ?? ''}
						</pre>
					</div>
				{:else}
					<div data-testid="mail-empty-detail" class="text-center text-sm text-muted-foreground py-12">
						<p class="text-lg">✉️</p>
						<p class="mt-2">Select a message to see details.</p>
						<p class="mt-1">Use j/k to move through the list.</p>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>
