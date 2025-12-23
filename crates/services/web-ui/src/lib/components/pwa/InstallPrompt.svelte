<script lang="ts">
	import { browser } from '$app/environment';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import Download from 'lucide-svelte/icons/download';
	import X from 'lucide-svelte/icons/x';
	import Share from 'lucide-svelte/icons/share';
	import MoreVertical from 'lucide-svelte/icons/more-vertical';

	// BeforeInstallPromptEvent is not in standard types
	interface BeforeInstallPromptEvent extends Event {
		prompt(): Promise<void>;
		userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
	}

	let deferredPrompt = $state<BeforeInstallPromptEvent | null>(null);
	let showPrompt = $state(false);
	let isIOS = $state(false);
	let isStandalone = $state(false);

	$effect(() => {
		if (!browser) return;

		// Detect iOS
		isIOS = /iPad|iPhone|iPod/.test(navigator.userAgent);

		// Check if already installed (standalone mode)
		isStandalone = window.matchMedia('(display-mode: standalone)').matches;

		// Don't show if already installed
		if (isStandalone) return;

		// Listen for beforeinstallprompt event (Chrome/Edge/Android)
		const handleBeforeInstall = (e: Event) => {
			e.preventDefault();
			deferredPrompt = e as BeforeInstallPromptEvent;
			showPrompt = true;
		};

		window.addEventListener('beforeinstallprompt', handleBeforeInstall);

		// For iOS, show manual instructions after a delay
		if (isIOS && !isStandalone) {
			const timer = setTimeout(() => {
				showPrompt = true;
			}, 3000);
			return () => clearTimeout(timer);
		}

		return () => {
			window.removeEventListener('beforeinstallprompt', handleBeforeInstall);
		};
	});

	async function handleInstall() {
		if (!deferredPrompt) return;

		await deferredPrompt.prompt();
		const { outcome } = await deferredPrompt.userChoice;

		if (outcome === 'accepted') {
			showPrompt = false;
		}
		deferredPrompt = null;
	}

	function dismiss() {
		showPrompt = false;
		// Store dismissal in localStorage to not show again for a while
		if (browser) {
			localStorage.setItem('pwa-install-dismissed', Date.now().toString());
		}
	}
</script>

{#if showPrompt && !isStandalone}
	<div
		class="fixed bottom-4 left-4 right-4 z-50 md:left-auto md:right-4 md:w-96"
		role="dialog"
		aria-labelledby="install-title"
	>
		<Card.Root class="shadow-lg border-primary/20">
			<Card.Header class="pb-2">
				<div class="flex items-start justify-between">
					<Card.Title id="install-title" class="text-base">Install Agent Mail</Card.Title>
					<Button
						variant="ghost"
						size="icon"
						class="h-8 w-8 -mr-2 -mt-2"
						onclick={dismiss}
						aria-label="Dismiss"
					>
						<X class="h-4 w-4" />
					</Button>
				</div>
			</Card.Header>
			<Card.Content class="space-y-3">
				{#if isIOS}
					<!-- iOS installation instructions -->
					<p class="text-sm text-muted-foreground">
						Install this app on your device for quick access:
					</p>
					<ol class="text-sm text-muted-foreground space-y-2 list-decimal list-inside">
						<li class="flex items-center gap-2">
							Tap the Share button <Share class="h-4 w-4 inline" />
						</li>
						<li>Scroll down and tap "Add to Home Screen"</li>
						<li>Tap "Add" to confirm</li>
					</ol>
				{:else}
					<!-- Android/Desktop installation -->
					<p class="text-sm text-muted-foreground">
						Install Agent Mail for quick access and offline support.
					</p>
					<Button onclick={handleInstall} class="w-full gap-2">
						<Download class="h-4 w-4" />
						Install App
					</Button>
				{/if}
			</Card.Content>
		</Card.Root>
	</div>
{/if}
