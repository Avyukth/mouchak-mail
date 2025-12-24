<!--
  @component
  StatCard - Dashboard statistics card with animated number display.

  @example
  ```svelte
  <StatCard
    icon={FolderKanban}
    value={299}
    label="Projects"
    href="/projects"
    trend="+12"
    trendDirection="up"
  />
  ```
-->
<script lang="ts">
	import * as Card from '$lib/components/ui/card/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { NumberTicker } from '$lib/components/magic';
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';

	interface Props {
		/** Lucide icon component - use as slot */
		icon: typeof import('lucide-svelte/icons/folder-kanban').default;
		/** Numeric value to display with animation */
		value: number;
		/** Label text below the value */
		label: string;
		/** Optional link URL */
		href?: string;
		/** Optional trend text (e.g., "+12") */
		trend?: string;
		/** Trend direction for coloring */
		trendDirection?: 'up' | 'down';
		/** Highlight the card (e.g., for unread items) */
		highlight?: boolean;
		/** Animation delay in milliseconds */
		delay?: number;
		/** Additional CSS classes */
		class?: string;
	}

	let {
		icon: Icon,
		value,
		label,
		href,
		trend,
		trendDirection,
		highlight = false,
		delay = 0,
		class: className
	}: Props = $props();
</script>

<svelte:element
	this={href ? 'a' : 'div'}
	{href}
	class={cn(
		'block group focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 rounded-lg',
		href && 'cursor-pointer',
		className
	)}
	data-testid="stat-card"
>
	<Card.Root
		class={cn(
			'p-4 transition-colors h-full',
			href && 'hover:bg-muted/50',
			highlight && 'ring-2 ring-primary/50 border-primary/30'
		)}
	>
		<div class="flex items-center justify-between">
			<Icon class="h-4 w-4 text-muted-foreground shrink-0" />
			<div class="flex items-center gap-2">
				{#if trend}
					<Badge
						variant="secondary"
						class={cn(
							'text-xs',
							trendDirection === 'up' && 'text-success',
							trendDirection === 'down' && 'text-destructive'
						)}
					>
						{trend}
					</Badge>
				{/if}
				{#if highlight}
					<span
						class="h-2 w-2 rounded-full bg-primary animate-pulse"
						aria-label="Needs attention"
					></span>
				{/if}
			</div>
		</div>
		<div class="mt-3">
			<p class="text-2xl md:text-3xl font-bold tracking-tight text-foreground">
				<NumberTicker {value} {delay} />
			</p>
			<p class="text-xs text-muted-foreground">{label}</p>
		</div>
	</Card.Root>
</svelte:element>
