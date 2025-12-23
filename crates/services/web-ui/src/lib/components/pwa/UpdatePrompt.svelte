<script lang="ts">
	import { browser } from '$app/environment';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import RefreshCw from 'lucide-svelte/icons/refresh-cw';
	import X from 'lucide-svelte/icons/x';

	let showUpdate = $state(false);
	let registration = $state<ServiceWorkerRegistration | null>(null);

	$effect(() => {
		if (!browser) return;

		// @vite-pwa/sveltekit with registerType: 'autoUpdate' handles most updates automatically
		// This component shows a prompt when a new SW is waiting

		const checkForUpdates = async () => {
			if (!('serviceWorker' in navigator)) return;

			try {
				const reg = await navigator.serviceWorker.ready;
				registration = reg;

				// Check if there's a waiting service worker
				if (reg.waiting) {
					showUpdate = true;
				}

				// Listen for new service workers
				reg.addEventListener('updatefound', () => {
					const newWorker = reg.installing;
					if (!newWorker) return;

					newWorker.addEventListener('statechange', () => {
						if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {
							// New update available
							showUpdate = true;
						}
					});
				});
			} catch (error) {
				console.error('SW registration check failed:', error);
			}
		};

		checkForUpdates();

		// Listen for controller change (another tab updated the SW)
		let refreshing = false;
		navigator.serviceWorker.addEventListener('controllerchange', () => {
			if (refreshing) return;
			refreshing = true;
			window.location.reload();
		});
	});

	function handleUpdate() {
		if (!registration?.waiting) return;

		// Tell the waiting service worker to skip waiting
		registration.waiting.postMessage({ type: 'SKIP_WAITING' });
		showUpdate = false;
	}

	function dismiss() {
		showUpdate = false;
	}
</script>

{#if showUpdate}
	<div
		class="fixed top-4 left-4 right-4 z-50 md:left-auto md:right-4 md:w-96"
		role="alert"
		aria-live="polite"
	>
		<Card.Root class="shadow-lg border-primary/20 bg-primary text-primary-foreground">
			<Card.Header class="pb-2">
				<div class="flex items-start justify-between">
					<Card.Title class="text-base text-primary-foreground">Update Available</Card.Title>
					<Button
						variant="ghost"
						size="icon"
						class="h-8 w-8 -mr-2 -mt-2 hover:bg-primary-foreground/10"
						onclick={dismiss}
						aria-label="Dismiss"
					>
						<X class="h-4 w-4" />
					</Button>
				</div>
			</Card.Header>
			<Card.Content class="space-y-3">
				<p class="text-sm opacity-90">
					A new version of Agent Mail is available. Refresh to update.
				</p>
				<Button
					onclick={handleUpdate}
					variant="secondary"
					class="w-full gap-2"
				>
					<RefreshCw class="h-4 w-4" />
					Refresh Now
				</Button>
			</Card.Content>
		</Card.Root>
	</div>
{/if}
