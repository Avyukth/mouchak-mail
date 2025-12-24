<script lang="ts">
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import * as Card from '$lib/components/ui/card';
</script>

<div data-testid="project-skeleton">
	<Card.Root class="h-full">
		<Card.Content class="p-5 md:p-6">
			<div class="flex items-start justify-between mb-4">
				<div class="flex items-center gap-3">
					<!-- Icon placeholder -->
					<Skeleton class="w-10 h-10 rounded-lg" />
					<div class="min-w-0">
						<!-- Title placeholder -->
						<Skeleton class="h-5 w-32" />
					</div>
				</div>
				<!-- Menu button placeholder -->
				<Skeleton class="h-8 w-8 rounded-md" />
			</div>

			<!-- Date placeholder -->
			<div class="flex items-center gap-2">
				<Skeleton class="h-4 w-4 rounded" />
				<Skeleton class="h-4 w-28" />
			</div>
		</Card.Content>
	</Card.Root>
</div>
