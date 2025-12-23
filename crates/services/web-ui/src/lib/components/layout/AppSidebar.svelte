<script lang="ts">
	import { page } from '$app/stores';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Sheet from '$lib/components/ui/sheet/index.js';
	import type { ComponentType } from 'svelte';
	import LayoutDashboard from 'lucide-svelte/icons/layout-dashboard';
	import FolderKanban from 'lucide-svelte/icons/folder-kanban';
	import Bot from 'lucide-svelte/icons/bot';
	import Inbox from 'lucide-svelte/icons/inbox';
	import Mail from 'lucide-svelte/icons/mail';
	import PanelLeft from 'lucide-svelte/icons/panel-left';
	import Sun from 'lucide-svelte/icons/sun';
	import Moon from 'lucide-svelte/icons/moon';
	import { toggleMode } from 'mode-watcher';

	interface NavItem {
		href: string;
		label: string;
		icon: ComponentType;
		badge?: number;
	}

	interface Props {
		unreadCount?: number;
	}

	let { unreadCount = 0 }: Props = $props();

	const navItems: NavItem[] = [
		{ href: '/', label: 'Dashboard', icon: LayoutDashboard },
		{ href: '/projects', label: 'Projects', icon: FolderKanban },
		{ href: '/agents', label: 'Agents', icon: Bot },
		{ href: '/mail', label: 'Mail', icon: Mail },
		{ href: '/inbox', label: 'Inbox', icon: Inbox }
	];

	// Reactive badge for inbox
	let navItemsWithBadge = $derived(
		navItems.map((item) => ({
			...item,
			badge: item.href === '/inbox' ? unreadCount : undefined
		}))
	);

	let mobileOpen = $state(false);

	function isActive(href: string): boolean {
		if (href === '/') return $page.url.pathname === '/';
		return $page.url.pathname.startsWith(href);
	}

	function closeMobile() {
		mobileOpen = false;
	}
</script>

<!-- Mobile: Header + Sheet sidebar (< 768px) -->
<header
	class="md:hidden fixed top-0 left-0 right-0 z-40 bg-card border-b border-border px-4 py-2 flex items-center gap-3"
>
	<Sheet.Root bind:open={mobileOpen}>
		<Sheet.Trigger
			class="inline-flex items-center justify-center rounded-md min-h-[44px] min-w-[44px] hover:bg-accent transition-colors"
		>
			<PanelLeft class="h-5 w-5" />
			<span class="sr-only">Toggle menu</span>
		</Sheet.Trigger>
		<Sheet.Content side="left" class="w-72 p-0">
			<Sheet.Header class="border-b border-border p-4">
				<Sheet.Title class="flex items-center gap-2 text-lg font-bold text-primary">
					<Mail class="h-5 w-5" />
					Agent Mail
				</Sheet.Title>
				<Sheet.Description class="text-sm text-muted-foreground">
					MCP Communication Hub
				</Sheet.Description>
			</Sheet.Header>
			<nav class="flex flex-col gap-1 p-4">
				{#each navItemsWithBadge as item}
					<a
						href={item.href}
						onclick={closeMobile}
						class="flex items-center gap-3 rounded-lg px-3 min-h-[44px] transition-colors {isActive(
							item.href
						)
							? 'bg-primary/10 text-primary border-l-4 border-primary'
							: 'hover:bg-accent text-foreground'}"
					>
						<item.icon class="h-5 w-5 flex-shrink-0" />
						<span class="font-medium flex-1">{item.label}</span>
						{#if item.badge && item.badge > 0}
							<Badge variant="destructive" class="ml-auto">{item.badge}</Badge>
						{/if}
					</a>
				{/each}
			</nav>
			<Sheet.Footer class="absolute bottom-0 left-0 right-0 border-t border-border p-4">
				<Button onclick={toggleMode} variant="outline" size="icon" class="min-h-[44px] min-w-[44px]">
					<Sun class="h-5 w-5 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
					<Moon
						class="absolute h-5 w-5 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100"
					/>
					<span class="sr-only">Toggle theme</span>
				</Button>
			</Sheet.Footer>
		</Sheet.Content>
	</Sheet.Root>
	<div class="flex items-center gap-2">
		<Mail class="h-5 w-5 text-primary" />
		<span class="font-semibold text-primary">Agent Mail</span>
	</div>
</header>
<!-- Mobile spacer for fixed header -->
<div class="md:hidden h-14 flex-shrink-0"></div>

<!-- Tablet/Desktop: Fixed sidebar (>= 768px) -->
<aside
	class="hidden md:flex flex-col border-r border-border bg-card transition-all duration-300 w-64"
>
	<!-- Header - h-14 matches AppHeader -->
	<div class="flex items-center gap-2 border-b border-border h-14 px-4">
		<Mail class="h-5 w-5 text-primary" />
		<div class="flex flex-col">
			<span class="text-sm font-bold text-primary">Agent Mail</span>
			<span class="text-xs text-muted-foreground">MCP Communication Hub</span>
		</div>
	</div>

	<!-- Navigation -->
	<nav class="flex-1 flex flex-col gap-1 p-2">
		{#each navItemsWithBadge as item}
			<a
				href={item.href}
				class="flex items-center gap-3 rounded-lg px-3 min-h-[44px] transition-colors {isActive(
					item.href
				)
					? 'bg-primary/10 text-primary border-l-4 border-primary -ml-0.5 pl-2.5'
					: 'hover:bg-accent text-foreground'}"
			>
				<item.icon class="h-5 w-5 flex-shrink-0" />
				<span class="font-medium flex-1">{item.label}</span>
				{#if item.badge && item.badge > 0}
					<Badge variant="destructive">{item.badge}</Badge>
				{/if}
			</a>
		{/each}
	</nav>

	<!-- Footer -->
	<div class="border-t border-border p-2 flex items-center justify-end">
		<Button onclick={toggleMode} variant="ghost" size="icon" class="min-h-[44px] min-w-[44px]">
			<Sun class="h-5 w-5 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
			<Moon class="absolute h-5 w-5 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
			<span class="sr-only">Toggle theme</span>
		</Button>
	</div>
</aside>
