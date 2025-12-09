<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import type { Snippet } from 'svelte';

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();

	interface NavItem {
		href: string;
		label: string;
		icon: string;
	}

	const navItems: NavItem[] = [
		{ href: '/', label: 'Dashboard', icon: '🏠' },
		{ href: '/projects', label: 'Projects', icon: '📁' },
		{ href: '/agents', label: 'Agents', icon: '🤖' },
		{ href: '/inbox', label: 'Inbox', icon: '📬' }
	];

	let sidebarOpen = $state(true);

	function isActive(href: string): boolean {
		if (href === '/') return $page.url.pathname === '/';
		return $page.url.pathname.startsWith(href);
	}
</script>

<div class="min-h-screen flex">
	<!-- Sidebar -->
	<aside
		class="w-64 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 flex-shrink-0 transition-all duration-300"
		class:hidden={!sidebarOpen}
	>
		<div class="p-4 border-b border-gray-200 dark:border-gray-700">
			<h1 class="text-xl font-bold text-primary-600 dark:text-primary-400">
				📧 Agent Mail
			</h1>
			<p class="text-sm text-gray-500 dark:text-gray-400">MCP Communication Hub</p>
		</div>

		<nav class="p-4 space-y-1">
			{#each navItems as item}
				<a
					href={item.href}
					class="flex items-center gap-3 px-3 py-2 rounded-lg transition-colors"
					class:bg-primary-100={isActive(item.href)}
					class:dark:bg-primary-900={isActive(item.href)}
					class:text-primary-700={isActive(item.href)}
					class:dark:text-primary-300={isActive(item.href)}
					class:hover:bg-gray-100={!isActive(item.href)}
					class:dark:hover:bg-gray-700={!isActive(item.href)}
				>
					<span class="text-lg">{item.icon}</span>
					<span class="font-medium">{item.label}</span>
				</a>
			{/each}
		</nav>
	</aside>

	<!-- Main content -->
	<div class="flex-1 flex flex-col">
		<!-- Top bar -->
		<header class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
			<div class="flex items-center justify-between">
				<button
					onclick={() => sidebarOpen = !sidebarOpen}
					class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700"
				>
					<span class="text-xl">☰</span>
				</button>

				<div class="flex items-center gap-4">
					<span class="text-sm text-gray-500 dark:text-gray-400">
						MCP Agent Mail
					</span>
				</div>
			</div>
		</header>

		<!-- Page content -->
		<main class="flex-1 p-6 bg-gray-50 dark:bg-gray-900 overflow-auto">
			{@render children()}
		</main>
	</div>
</div>
