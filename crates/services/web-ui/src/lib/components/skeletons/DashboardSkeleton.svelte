<script lang="ts">
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
</script>

<!-- Stats Grid: 4 cards -->
<div class="grid grid-cols-2 md:grid-cols-4 gap-3 md:gap-4 mb-6">
	{#each Array(4) as _, i}
		<Card.Root class="p-4">
			<div class="flex items-center justify-between mb-3">
				<Skeleton class="h-4 w-4 skeleton-shimmer" />
				<Skeleton class="h-5 w-10 rounded-full skeleton-shimmer" />
			</div>
			<Skeleton class="h-8 w-16 mb-1 skeleton-shimmer" />
			<Skeleton class="h-3 w-12 skeleton-shimmer" />
		</Card.Root>
	{/each}
</div>

<!-- Main Content Grid: 2 cols + 1 col -->
<div class="grid grid-cols-1 md:grid-cols-3 gap-4 md:gap-6">
	<!-- Recent Projects (2 cols) -->
	<div class="md:col-span-2">
		<Card.Root>
			<Card.Header class="pb-2">
				<div class="flex items-center justify-between">
					<Skeleton class="h-5 w-32 skeleton-shimmer" />
					<Skeleton class="h-4 w-16 skeleton-shimmer" />
				</div>
			</Card.Header>
			<Card.Content class="space-y-1 pt-0">
				{#each Array(5) as _, i}
					<div class="flex items-center gap-3 p-3 -mx-3">
						<Skeleton class="h-9 w-9 rounded-md skeleton-shimmer" />
						<div class="flex-1 space-y-1">
							<Skeleton class="h-4 w-32 skeleton-shimmer" />
							<Skeleton class="h-3 w-20 skeleton-shimmer" />
						</div>
						<Skeleton class="h-4 w-4 skeleton-shimmer" />
					</div>
				{/each}
			</Card.Content>
		</Card.Root>
	</div>

	<!-- Quick Actions (1 col) -->
	<div>
		<Card.Root>
			<Card.Header class="pb-2">
				<Skeleton class="h-5 w-24 skeleton-shimmer" />
			</Card.Header>
			<Card.Content class="space-y-3">
				<Skeleton class="h-11 w-full rounded-lg skeleton-shimmer" />
				<Skeleton class="h-11 w-full rounded-lg skeleton-shimmer" />
				<Skeleton class="h-11 w-full rounded-lg skeleton-shimmer" />
			</Card.Content>
		</Card.Root>
	</div>
</div>
