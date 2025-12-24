<script lang="ts">
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
</script>

<div data-testid="agent-skeleton" class="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
	<div class="flex items-start justify-between mb-4">
		<div class="flex items-center gap-3">
			<Skeleton class="w-10 h-10 rounded-full skeleton-shimmer" />
			<div class="space-y-2">
				<Skeleton class="h-5 w-24 skeleton-shimmer" />
				<Skeleton class="h-4 w-16 skeleton-shimmer" />
			</div>
		</div>
	</div>

	<div class="space-y-3">
		<div class="flex justify-between items-center">
			<Skeleton class="h-4 w-12 skeleton-shimmer" />
			<Skeleton class="h-4 w-20 skeleton-shimmer" />
		</div>
		<div class="flex justify-between items-center">
			<Skeleton class="h-4 w-16 skeleton-shimmer" />
			<Skeleton class="h-4 w-32 skeleton-shimmer" />
		</div>
		<div class="pt-2 border-t border-gray-200 dark:border-gray-700">
			<div class="flex justify-between items-center">
				<Skeleton class="h-4 w-20 skeleton-shimmer" />
				<Skeleton class="h-4 w-28 skeleton-shimmer" />
			</div>
		</div>
	</div>

	<div class="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
		<Skeleton class="h-4 w-24 skeleton-shimmer" />
	</div>
</div>
