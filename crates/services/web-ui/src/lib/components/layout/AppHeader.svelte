<script lang="ts">
	import { page } from '$app/stores';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Breadcrumb from '$lib/components/ui/breadcrumb/index.js';
	import Command from 'lucide-svelte/icons/command';
	import Home from 'lucide-svelte/icons/home';
	import ChevronRight from 'lucide-svelte/icons/chevron-right';

	// Route label mappings
	const routeLabels: Record<string, string> = {
		'': 'Dashboard',
		projects: 'Projects',
		agents: 'Agents',
		mail: 'Mail',
		inbox: 'Inbox'
	};

	// Generate breadcrumbs from current path
	let breadcrumbs = $derived.by(() => {
		const pathname = $page.url.pathname;
		const segments = pathname.split('/').filter(Boolean);

		if (segments.length === 0) {
			return [{ href: '/', label: 'Dashboard', isLast: true }];
		}

		const crumbs: Array<{ href: string; label: string; isLast: boolean }> = [
			{ href: '/', label: 'Home', isLast: false }
		];

		let currentPath = '';
		segments.forEach((segment, index) => {
			currentPath += `/${segment}`;
			const isLast = index === segments.length - 1;
			const prevSegment = index > 0 ? segments[index - 1] : '';

			// Check if this is a dynamic segment (like a UUID or numeric ID)
			// UUIDs follow 8-4-4-4-12 pattern of hex chars
			const containsUUID = segment.match(/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}/i);
			const isNumericId = segment.match(/^\d+$/);
			// Also treat long hex-looking strings as dynamic (like project slugs)
			const isLongSlug = segment.length > 20 && segment.includes('-');
			const isDynamic = containsUUID || isNumericId || isLongSlug;

			// For dynamic segments, show a human-readable label based on context
			let label: string;
			if (isDynamic) {
				// Use context from previous segment to determine label
				if (prevSegment === 'projects') {
					label = 'Project Details';
				} else if (prevSegment === 'agents') {
					label = 'Agent Details';
				} else if (prevSegment === 'inbox' || prevSegment === 'messages') {
					label = 'Message';
				} else {
					label = 'Details';
				}
			} else {
				label = routeLabels[segment] || capitalize(segment);
			}

			crumbs.push({
				href: currentPath,
				label,
				isLast
			});
		});

		return crumbs;
	});

	function capitalize(str: string): string {
		return str.charAt(0).toUpperCase() + str.slice(1).replace(/-/g, ' ');
	}

	function handleCommandPalette() {
		// Dispatch custom event for command palette
		window.dispatchEvent(new CustomEvent('open-command-palette'));
	}
</script>

<header class="hidden md:flex items-center justify-between border-b border-border bg-card h-14 px-6">
	<!-- Breadcrumbs -->
	<Breadcrumb.Root>
		<Breadcrumb.List>
			{#each breadcrumbs as crumb, i}
				{#if i > 0}
					<Breadcrumb.Separator>
						<ChevronRight class="h-4 w-4" />
					</Breadcrumb.Separator>
				{/if}
				<Breadcrumb.Item>
					{#if crumb.isLast}
						<Breadcrumb.Page class="font-medium">{crumb.label}</Breadcrumb.Page>
					{:else}
						<Breadcrumb.Link href={crumb.href} class="text-muted-foreground hover:text-foreground transition-colors">
							{#if crumb.href === '/'}
								<Home class="h-4 w-4" />
							{:else}
								{crumb.label}
							{/if}
						</Breadcrumb.Link>
					{/if}
				</Breadcrumb.Item>
			{/each}
		</Breadcrumb.List>
	</Breadcrumb.Root>

	<!-- Right side actions -->
	<div class="flex items-center gap-2">
		<!-- Command Palette Trigger -->
		<Button
			variant="outline"
			size="sm"
			class="hidden lg:flex items-center gap-2 text-muted-foreground"
			onclick={handleCommandPalette}
		>
			<span class="text-sm">Search...</span>
			<kbd class="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border border-border bg-muted px-1.5 font-mono text-2xs font-medium text-muted-foreground">
				<Command class="h-3 w-3" />
				<span>K</span>
			</kbd>
		</Button>
		<!-- Mobile command button -->
		<Button
			variant="ghost"
			size="icon"
			class="lg:hidden min-h-[44px] min-w-[44px]"
			onclick={handleCommandPalette}
		>
			<Command class="h-5 w-5" />
			<span class="sr-only">Open command palette</span>
		</Button>
	</div>
</header>
