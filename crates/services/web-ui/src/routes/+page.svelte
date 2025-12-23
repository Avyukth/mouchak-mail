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
	import Inbox from 'lucide-svelte/icons/inbox';
	import { DashboardSkeleton } from '$lib/components/skeletons';
	import { NumberTicker, BlurFade, GridPattern, ShimmerButton, StatusIndicator } from '$lib/components/magic';

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

	// Status indicator mapping
	const statusMap = $derived(
		healthStatus === 'ok' ? 'online' :
		healthStatus === 'checking...' ? 'away' : 'offline'
	);

	const statusLabel = $derived(
		healthStatus === 'ok' ? 'Online' :
		healthStatus === 'checking...' ? 'Checking...' : 'Offline'
	);
</script>

<GridPattern pattern="dots" opacity={0.05} masked class="min-h-screen -m-4 md:-m-6 p-4 md:p-6">
	<div class="space-y-6 max-w-6xl mx-auto">
		<!-- Page Header with proper hierarchy -->
		<BlurFade delay={0}>
			<header>
				<h1 class="text-2xl md:text-3xl font-bold text-foreground">Dashboard</h1>
				<p class="text-muted-foreground mt-1">Welcome to MCP Agent Mail</p>
			</header>
		</BlurFade>

		{#if loading}
			<DashboardSkeleton />
		{:else}
			<!-- Error Alert with recovery actions -->
			{#if error && !errorDismissed}
				<BlurFade delay={100}>
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
				</BlurFade>
			{/if}

			<!-- Status Cards with consistent spacing -->
			<div class="grid grid-cols-1 md:grid-cols-3 gap-4 md:gap-6">
				<!-- Backend Status -->
				<BlurFade delay={100}>
					<Card.Root class="h-full">
						<Card.Header class="pb-2">
							<Card.Title class="text-base flex items-center gap-3">
								<StatusIndicator status={statusMap} size="md" />
								Backend Status
							</Card.Title>
						</Card.Header>
						<Card.Content>
							<p class="text-2xl font-bold text-foreground capitalize">
								{statusLabel}
							</p>
						</Card.Content>
					</Card.Root>
				</BlurFade>

				<!-- Projects Count -->
				<BlurFade delay={150}>
					<Card.Root class="h-full">
						<Card.Header class="pb-2">
							<div class="flex items-center gap-3">
								<FolderKanban class="h-5 w-5 text-primary shrink-0" aria-hidden="true" />
								<Card.Title class="text-base">Projects</Card.Title>
							</div>
						</Card.Header>
						<Card.Content>
							<p class="text-2xl font-bold text-primary">
								<NumberTicker value={projects.length} delay={200} />
								<span class="sr-only">{projects.length} projects</span>
							</p>
						</Card.Content>
					</Card.Root>
				</BlurFade>

				<!-- Quick Actions -->
				<BlurFade delay={200}>
					<Card.Root class="h-full">
						<Card.Header class="pb-2">
							<Card.Title class="text-base">Quick Actions</Card.Title>
						</Card.Header>
						<Card.Content class="space-y-3">
							<a href="/projects" class="block">
								<ShimmerButton class="w-full justify-between">
									<span class="flex items-center gap-2">
										<FolderKanban class="h-4 w-4" />
										View Projects
									</span>
									<ArrowRight class="h-4 w-4" />
								</ShimmerButton>
							</a>
							<a
								href="/inbox"
								class="flex items-center justify-between min-h-[44px] px-4 py-2 bg-muted text-muted-foreground rounded-lg hover:bg-muted/80 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
							>
								<span class="flex items-center gap-2 font-medium">
									<Inbox class="h-4 w-4" />
									Check Inbox
								</span>
								<ArrowRight class="h-4 w-4" aria-hidden="true" />
							</a>
						</Card.Content>
					</Card.Root>
				</BlurFade>
			</div>

			<!-- Recent Projects -->
			{#if projects.length > 0}
				<BlurFade delay={250}>
					<Card.Root>
						<Card.Header>
							<Card.Title>Recent Projects</Card.Title>
						</Card.Header>
						<Card.Content class="p-0">
							<ul class="divide-y divide-border" role="list">
								{#each projects.slice(0, 5) as project, index}
									<li
										class="animate-in fade-in slide-in-from-bottom-2"
										style="animation-delay: {index * 50}ms; animation-fill-mode: both;"
									>
										<a
											href="/projects/{project.slug}"
											class="block min-h-[60px] px-6 py-4 hover:bg-muted/50 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
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
				</BlurFade>
			{/if}
		{/if}
	</div>
</GridPattern>

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
