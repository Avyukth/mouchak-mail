<script lang="ts">
	import { browser } from '$app/environment';
	import { checkHealth, getProjects, type Project } from '$lib/api/client';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import FolderKanban from 'lucide-svelte/icons/folder-kanban';
	import ArrowRight from 'lucide-svelte/icons/arrow-right';
	import AlertCircle from 'lucide-svelte/icons/alert-circle';
	import RefreshCw from 'lucide-svelte/icons/refresh-cw';
	import X from 'lucide-svelte/icons/x';
	import { DashboardSkeleton } from '$lib/components/skeletons';

	let healthStatus = $state<string>('checking...');
	let projects = $state<Project[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let errorDismissed = $state(false);
	let retrying = $state(false);

	// Use $effect for client-side data loading in Svelte 5
	$effect(() => {
		if (browser) {
			loadData();
		}
	});

	async function loadData() {
		loading = true;
		errorDismissed = false;
		try {
			const health = await checkHealth();
			healthStatus = health.status;
			error = null;

			const projectList = await getProjects();
			projects = projectList;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to connect to backend';
			healthStatus = 'offline';
		} finally {
			loading = false;
		}
	}

	async function handleRetry() {
		retrying = true;
		errorDismissed = false;
		await loadData();
		retrying = false;
	}

	function dismissError() {
		errorDismissed = true;
	}

	// Status text for accessibility
	const statusText = $derived(
		healthStatus === 'ok' ? 'Online' : healthStatus === 'checking...' ? 'Checking' : 'Offline'
	);
</script>

<div class="space-y-6">
	<!-- Page Header with proper hierarchy -->
	<header>
		<h1 class="text-2xl font-bold text-foreground">Dashboard</h1>
		<p class="text-muted-foreground mt-1">Welcome to MCP Agent Mail</p>
	</header>

	{#if loading}
		<DashboardSkeleton />
	{:else}
		<!-- Error Alert with recovery actions -->
		{#if error && !errorDismissed}
			<Alert.Root variant="destructive" class="relative" aria-live="polite">
				<AlertCircle class="h-4 w-4" />
				<Alert.Title class="font-semibold">Connection Error</Alert.Title>
				<Alert.Description class="mt-2">
					<p>{error}</p>
					<p class="text-sm mt-1 opacity-80">
						Make sure the backend is running on port 8000
					</p>
					<div class="flex items-center gap-3 mt-4">
						<Button
							variant="outline"
							size="sm"
							onclick={handleRetry}
							disabled={retrying}
							class="gap-2"
						>
							<RefreshCw class="h-4 w-4 {retrying ? 'animate-spin' : ''}" />
							{retrying ? 'Retrying...' : 'Retry Connection'}
						</Button>
						<a
							href="https://github.com/Avyukth/mcp-agent-mail-rs#quick-start"
							target="_blank"
							rel="noopener noreferrer"
							class="text-sm underline underline-offset-4 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 rounded"
						>
							View Setup Guide
						</a>
					</div>
				</Alert.Description>
				<Button
					variant="ghost"
					size="icon"
					class="absolute top-2 right-2 h-8 w-8"
					onclick={dismissError}
					aria-label="Dismiss error"
				>
					<X class="h-4 w-4" />
				</Button>
			</Alert.Root>
		{/if}

		<!-- Status Cards with consistent spacing -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
			<!-- Backend Status -->
			<Card.Root>
				<Card.Header class="pb-2">
					<div class="flex items-center gap-3">
						<div
							class="w-3 h-3 rounded-full shrink-0"
							class:bg-green-500={healthStatus === 'ok'}
							class:bg-amber-500={healthStatus === 'checking...'}
							class:bg-red-500={healthStatus === 'offline'}
							role="img"
							aria-label="{statusText} status indicator"
						></div>
						<Card.Title class="text-base">Backend Status</Card.Title>
					</div>
				</Card.Header>
				<Card.Content>
					<p class="text-2xl font-bold text-foreground capitalize flex items-center gap-2">
						{healthStatus}
						<span class="sr-only">({statusText})</span>
					</p>
				</Card.Content>
			</Card.Root>

			<!-- Projects Count -->
			<Card.Root>
				<Card.Header class="pb-2">
					<div class="flex items-center gap-3">
						<FolderKanban class="h-5 w-5 text-primary shrink-0" aria-hidden="true" />
						<Card.Title class="text-base">Projects</Card.Title>
					</div>
				</Card.Header>
				<Card.Content>
					<p class="text-2xl font-bold text-primary">
						{projects.length}
						<span class="sr-only">projects</span>
					</p>
				</Card.Content>
			</Card.Root>

			<!-- Quick Actions -->
			<Card.Root>
				<Card.Header class="pb-2">
					<Card.Title class="text-base">Quick Actions</Card.Title>
				</Card.Header>
				<Card.Content class="space-y-2">
					<a
						href="/projects"
						class="flex items-center justify-between px-4 py-3 bg-primary/10 text-primary rounded-lg hover:bg-primary/20 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
					>
						<span class="font-medium">View Projects</span>
						<ArrowRight class="h-4 w-4" aria-hidden="true" />
					</a>
					<a
						href="/inbox"
						class="flex items-center justify-between px-4 py-3 bg-muted text-muted-foreground rounded-lg hover:bg-muted/80 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
					>
						<span class="font-medium">Check Inbox</span>
						<ArrowRight class="h-4 w-4" aria-hidden="true" />
					</a>
				</Card.Content>
			</Card.Root>
		</div>

		<!-- Recent Projects -->
		{#if projects.length > 0}
			<Card.Root>
				<Card.Header>
					<Card.Title>Recent Projects</Card.Title>
				</Card.Header>
				<Card.Content class="p-0">
					<ul class="divide-y divide-border" role="list">
						{#each projects.slice(0, 5) as project}
							<li>
								<a
									href="/projects/{project.slug}"
									class="block px-6 py-4 hover:bg-muted/50 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
								>
									<p class="font-medium text-foreground">{project.human_key}</p>
									<p class="text-xs text-muted-foreground mt-1">
										<span class="font-mono bg-muted px-1.5 py-0.5 rounded">{project.slug}</span>
									</p>
								</a>
							</li>
						{/each}
					</ul>
				</Card.Content>
			</Card.Root>
		{/if}
	{/if}
</div>
