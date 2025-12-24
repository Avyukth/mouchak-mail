<script lang="ts">
	import { page } from '$app/stores';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Sheet from '$lib/components/ui/sheet/index.js';
	import { ThemeToggle } from '$lib/components/ui/theme-toggle/index.js';
	import type { ComponentType } from 'svelte';
	import LayoutDashboard from 'lucide-svelte/icons/layout-dashboard';
	import FolderKanban from 'lucide-svelte/icons/folder-kanban';
	import Bot from 'lucide-svelte/icons/bot';
	import Inbox from 'lucide-svelte/icons/inbox';
	import Mail from 'lucide-svelte/icons/mail';
	import PanelLeft from 'lucide-svelte/icons/panel-left';

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
		{ href: '/mail', label: 'Mail', icon: Mail },
		{ href: '/projects', label: 'Projects', icon: FolderKanban },
		{ href: '/agents', label: 'Agents', icon: Bot },
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

	// Spotlight state for Factory.ai hover effect
	let spotlightStyle = $state('opacity: 0;');
	let navContainer: HTMLElement;

	function handleNavHover(e: MouseEvent) {
		const target = e.currentTarget as HTMLElement;
		if (!navContainer) return;

		const rect = target.getBoundingClientRect();
		const containerRect = navContainer.getBoundingClientRect();

		spotlightStyle = `
			opacity: 1;
			transform: translateY(${rect.top - containerRect.top}px);
			width: ${rect.width}px;
			height: ${rect.height}px;
		`;
	}

	function handleNavLeave() {
		spotlightStyle = 'opacity: 0;';
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
				<ThemeToggle />
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
	class="hidden md:flex flex-col h-screen border-r border-border bg-card transition-all duration-300 w-64"
>
	<!-- Header - h-14 matches AppHeader -->
	<div class="flex items-center gap-2 border-b border-border h-14 px-4 flex-shrink-0">
		<Mail class="h-5 w-5 text-primary" />
		<div class="flex flex-col">
			<span class="text-sm font-bold text-primary">Agent Mail</span>
			<span class="text-xs text-muted-foreground">MCP Communication Hub</span>
		</div>
	</div>

	<!-- Navigation with Factory.ai Spotlight -->
	<nav
		bind:this={navContainer}
		class="nav-container flex-1 flex flex-col gap-1 p-2 overflow-y-auto relative"
		onmouseleave={handleNavLeave}
	>
		<div class="nav-spotlight" style={spotlightStyle}></div>
		{#each navItemsWithBadge as item}
			<a
				href={item.href}
				onmouseenter={handleNavHover}
				class="nav-link flex items-center gap-3 rounded-lg px-3 min-h-[44px] transition-colors relative z-10 {isActive(
					item.href
				)
					? 'bg-primary/10 text-primary border-l-4 border-primary -ml-0.5 pl-2.5'
					: 'text-foreground'}"
			>
				<item.icon class="h-5 w-5 flex-shrink-0" />
				<span class="font-medium flex-1">{item.label}</span>
				{#if item.badge && item.badge > 0}
					<Badge variant="destructive">{item.badge}</Badge>
				{/if}
			</a>
		{/each}
	</nav>

	<!-- Footer with Factory.ai Theme Toggle -->
	<div class="border-t border-border p-2 flex items-center justify-center flex-shrink-0 bg-card">
		<ThemeToggle />
	</div>
</aside>

<style>
	/* Factory.ai Travelling Spotlight Effect */
	.nav-container {
		position: relative;
	}

	.nav-spotlight {
		position: absolute;
		left: 8px;
		top: 0;
		background: hsl(var(--primary) / 0.1);
		border-radius: 8px;
		pointer-events: none;
		z-index: 0;
		width: 0;
		height: 44px;
		overflow: hidden;
		transition:
			transform 0.2s cubic-bezier(0.4, 0, 0.2, 1),
			width 0.2s cubic-bezier(0.4, 0, 0.2, 1),
			height 0.2s cubic-bezier(0.4, 0, 0.2, 1),
			opacity 0.15s ease;
	}

	.nav-spotlight::before {
		content: '';
		position: absolute;
		inset: 0;
		background: repeating-linear-gradient(
			45deg,
			transparent 0px,
			transparent 2px,
			hsl(var(--primary) / 0.15) 2px,
			hsl(var(--primary) / 0.15) 3px,
			transparent 3px,
			transparent 5px
		);
		background-size: 7.07px 7.07px;
		opacity: 0;
		animation: stripe-slide 2000ms linear infinite paused;
	}

	.nav-container:hover .nav-spotlight::before {
		opacity: 1;
		animation: stripe-fade-in 100ms ease-out forwards, stripe-slide 2000ms linear infinite;
	}

	.nav-link {
		position: relative;
		z-index: 1;
	}

	/* Respect reduced motion preference */
	@media (prefers-reduced-motion: reduce) {
		.nav-spotlight {
			transition: opacity 0.1s ease;
		}
	}
</style>
