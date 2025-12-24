<!--
  @component
  QuickActions - Dashboard quick action buttons with proper visual hierarchy.

  @example
  ```svelte
  <QuickActions inboxCount={3} />
  ```
-->
<script lang="ts">
	import * as Card from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { ShimmerButton } from '$lib/components/magic';
	import Plus from 'lucide-svelte/icons/plus';
	import Inbox from 'lucide-svelte/icons/inbox';
	import Bot from 'lucide-svelte/icons/bot';
	import ArrowRight from 'lucide-svelte/icons/arrow-right';

	interface Props {
		/** Number of unread inbox items */
		inboxCount?: number;
	}

	let { inboxCount = 0 }: Props = $props();
</script>

<Card.Root data-testid="quick-actions">
	<Card.Header class="pb-2">
		<Card.Title class="text-base">Quick Actions</Card.Title>
	</Card.Header>
	<Card.Content class="space-y-3">
		<!-- Primary action: New Project -->
		<a href="/projects" class="block">
			<ShimmerButton class="w-full justify-between">
				<span class="flex items-center gap-2">
					<Plus class="h-4 w-4" />
					New Project
				</span>
				<ArrowRight class="h-4 w-4" />
			</ShimmerButton>
		</a>

		<!-- Secondary action: Inbox with badge -->
		<a
			href="/inbox"
			class="flex items-center justify-between min-h-[44px] px-4 py-2 bg-muted text-foreground rounded-lg hover:bg-muted/80 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
		>
			<span class="flex items-center gap-2 font-medium">
				<Inbox class="h-4 w-4" />
				Check Inbox
			</span>
			<span class="flex items-center gap-2">
				{#if inboxCount > 0}
					<Badge variant="destructive" class="text-xs">{inboxCount}</Badge>
				{/if}
				<ArrowRight class="h-4 w-4 text-muted-foreground" />
			</span>
		</a>

		<!-- Tertiary action: View Agents -->
		<Button href="/agents" variant="ghost" class="w-full justify-start">
			<Bot class="h-4 w-4 mr-2" />
			View Agents
		</Button>
	</Card.Content>
</Card.Root>
