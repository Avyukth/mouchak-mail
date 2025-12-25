<script lang="ts">
	import { browser } from '$app/environment';
	import {
		checkHealth,
		getDashboardStats,
		type DashboardStats,
		type Project
	} from '$lib/api/client';
	import * as Alert from '$lib/components/ui/alert/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import AlertCircle from 'lucide-svelte/icons/alert-circle';
	import RefreshCw from 'lucide-svelte/icons/refresh-cw';
	import X from 'lucide-svelte/icons/x';
	import FolderKanban from 'lucide-svelte/icons/folder-kanban';
	import Bot from 'lucide-svelte/icons/bot';
	import Inbox from 'lucide-svelte/icons/inbox';
	import Mail from 'lucide-svelte/icons/mail';
	import { DashboardSkeleton } from '$lib/components/skeletons';
	import { BlurFade, GridPattern } from '$lib/components/magic';
	import { StatCard, ProjectList, QuickActions } from '$lib/components/dashboard';

	let healthStatus = $state<string>('checking...');
	let stats = $state<DashboardStats | null>(null);
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

			const dashboardStats = await getDashboardStats();
			stats = dashboardStats;
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
		healthStatus === 'ok' ? 'online' : healthStatus === 'checking...' ? 'away' : 'offline'
	);

	const statusLabel = $derived(
		healthStatus === 'ok' ? 'Online' : healthStatus === 'checking...' ? 'Checking...' : 'Offline'
	);

	// Attention count for header subtitle
	const attentionCount = $derived(stats?.inboxCount ?? 0);

	// Recent projects (top 5)
	const recentProjects = $derived(stats?.projects.slice(0, 5) ?? []);
</script>

<GridPattern pattern="dots" opacity={0.05} masked class="min-h-screen -mx-4 md:-mx-6 p-4 md:p-6">
	<div class="space-y-6 max-w-6xl mx-auto">
		<!-- Page Header with status badge -->
		<BlurFade delay={0}>
			<header class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
				<div>
					<h1 class="text-xl md:text-2xl font-bold tracking-tight text-foreground">
						Welcome back!
					</h1>
					<p class="text-sm text-muted-foreground mt-1">
						{#if attentionCount > 0}
							{attentionCount}
							{attentionCount === 1 ? 'item needs' : 'items need'} your attention
						{:else}
							Everything's looking good
						{/if}
					</p>
				</div>
				<!-- Compact Status Badge -->
				<span
					class="inline-flex items-center gap-2 px-3 py-1.5 rounded-full text-sm font-medium
					{statusMap === 'online'
						? 'bg-green-500/10 text-green-500'
						: statusMap === 'away'
							? 'bg-yellow-500/10 text-yellow-500'
							: 'bg-red-500/10 text-red-500'}"
				>
					<span
						class="h-2 w-2 rounded-full {statusMap === 'online'
							? 'bg-green-500'
							: statusMap === 'away'
								? 'bg-yellow-500 animate-pulse'
								: 'bg-red-500'}"
					></span>
					{statusLabel}
				</span>
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

			<!-- Stats Grid: 2 cols mobile, 4 cols desktop -->
			<BlurFade delay={100}>
				<section class="grid grid-cols-2 md:grid-cols-4 gap-3 md:gap-4">
					<StatCard
						icon={FolderKanban}
						value={stats?.projectCount ?? 0}
						label="Projects"
						href="/projects"
						delay={0}
					/>
					<StatCard
						icon={Bot}
						value={stats?.agentCount ?? 0}
						label="Agents"
						href="/agents"
						delay={50}
					/>
					<StatCard
						icon={Inbox}
						value={stats?.inboxCount ?? 0}
						label="Inbox"
						href="/inbox"
						highlight={attentionCount > 0}
						delay={100}
					/>
					<StatCard
						icon={Mail}
						value={stats?.messageCount ?? 0}
						label="Messages"
						href="/mail"
						delay={150}
					/>
				</section>
			</BlurFade>

			<!-- Main Content Grid: Projects (2 cols) + Quick Actions (1 col) -->
			<BlurFade delay={200}>
				<section class="grid grid-cols-1 md:grid-cols-3 gap-4 md:gap-6">
					<!-- Recent Projects (2 cols on desktop) -->
					<div class="md:col-span-2">
						<ProjectList projects={recentProjects} />
					</div>

					<!-- Quick Actions (1 col) -->
					<div>
						<QuickActions inboxCount={attentionCount} />
					</div>
				</section>
			</BlurFade>
		{/if}
	</div>
</GridPattern>
