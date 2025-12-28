<script lang="ts">
	import '../app.css';
	import { browser } from '$app/environment';
	import { ModeWatcher } from 'mode-watcher';
	import { Toaster } from '$lib/components/ui/sonner/index.js';
	import { AppSidebar, AppHeader } from '$lib/components/layout/index.js';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import TutorialModal from '$lib/components/TutorialModal.svelte';
	import { InstallPrompt, UpdatePrompt } from '$lib/components/pwa/index.js';
	import DemoModeBanner from '$lib/components/DemoModeBanner.svelte';
	import { dataProvider } from '$lib/data';
	import type { Snippet } from 'svelte';

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();

	// Fetch actual unread count from unified inbox API
	let unreadCount = $state(0);

	$effect(() => {
		if (browser) {
			fetchUnreadCount();
			// Refresh every 30 seconds
			const interval = setInterval(fetchUnreadCount, 30000);
			return () => clearInterval(interval);
		}
	});

	async function fetchUnreadCount() {
		try {
			const response = await dataProvider.fetchUnifiedInbox(1000);
			unreadCount = response.messages.filter((m) => !m.is_read).length;
		} catch {
			// Silently fail - keep showing 0
		}
	}
</script>

<ModeWatcher />
<Toaster />
<CommandPalette />
<TutorialModal />
<InstallPrompt />
<UpdatePrompt />

<div class="h-screen flex flex-col overflow-hidden">
	<!-- Demo mode banner (only visible in static builds) -->
	<DemoModeBanner />

	<div class="flex-1 flex overflow-hidden">
		<!-- Sidebar (handles both mobile sheet trigger and desktop sidebar) -->
		<AppSidebar {unreadCount} />

		<!-- Main content -->
		<div class="flex-1 flex flex-col min-w-0 overflow-hidden">
			<!-- Header with breadcrumbs (desktop only, mobile has header in sidebar) -->
			<AppHeader />
			<main class="flex-1 px-4 md:px-6 pt-[61px] md:pt-0 bg-background overflow-y-auto">
				{@render children()}
			</main>
		</div>
	</div>
</div>
