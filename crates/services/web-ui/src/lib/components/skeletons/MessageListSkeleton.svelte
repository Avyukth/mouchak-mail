<script lang="ts">
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';

	let { rows = 5 }: { rows?: number } = $props();
</script>

<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
	<div class="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
		<Skeleton class="h-4 w-24 skeleton-shimmer" />
	</div>
	<ul class="divide-y divide-gray-200 dark:divide-gray-700">
		{#each Array(rows) as _, i}
			<li data-testid="message-skeleton" class="p-4">
				<div class="flex items-start justify-between gap-4">
					<div class="flex-1 min-w-0 space-y-2">
						<div class="flex items-center gap-2">
							<Skeleton class="h-5 w-48 skeleton-shimmer" />
							<Skeleton class="h-5 w-12 rounded-full skeleton-shimmer" />
						</div>
						<Skeleton class="h-4 w-full max-w-md skeleton-shimmer" />
					</div>
					<Skeleton class="h-4 w-16 skeleton-shimmer" />
				</div>
			</li>
		{/each}
	</ul>
</div>
