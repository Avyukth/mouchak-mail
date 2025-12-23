<script lang="ts">
	import { browser } from '$app/environment';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import Command from 'lucide-svelte/icons/command';
	import Filter from 'lucide-svelte/icons/filter';
	import RefreshCw from 'lucide-svelte/icons/refresh-cw';
	import Mail from 'lucide-svelte/icons/mail';
	import ChevronLeft from 'lucide-svelte/icons/chevron-left';
	import ChevronRight from 'lucide-svelte/icons/chevron-right';
	import type { ComponentType } from 'svelte';

	interface TutorialStep {
		title: string;
		description: string;
		icon: ComponentType;
		tips: string[];
	}

	const STORAGE_KEY = 'agent-mail-tutorial-seen';

	const steps: TutorialStep[] = [
		{
			title: 'Welcome to Agent Mail',
			description: 'Your multi-agent messaging hub for AI coding assistants.',
			icon: Mail,
			tips: [
				'Manage messages between AI agents and projects',
				'Track conversations with full threading support',
				'Archive and search through your message history'
			]
		},
		{
			title: 'Keyboard Shortcuts',
			description: 'Navigate faster with these shortcuts.',
			icon: Command,
			tips: [
				'⌘K or Ctrl+K - Open command palette',
				'/ - Quick search (when not in an input)',
				'Esc - Close dialogs and modals'
			]
		},
		{
			title: 'Filtering Messages',
			description: 'Find exactly what you need.',
			icon: Filter,
			tips: [
				'Filter by project, sender, or recipient',
				'Sort by importance level',
				'Use thread view for related messages'
			]
		},
		{
			title: 'Stay Updated',
			description: 'Never miss important messages.',
			icon: RefreshCw,
			tips: [
				'Messages auto-refresh in the background',
				'Unread count shown in sidebar',
				'Install as PWA for notifications'
			]
		}
	];

	let open = $state(false);
	let currentStep = $state(0);

	$effect(() => {
		if (!browser) return;

		// Check if tutorial has been seen
		const seen = localStorage.getItem(STORAGE_KEY);
		if (!seen) {
			// Show tutorial on first visit (with slight delay for page load)
			const timer = setTimeout(() => {
				open = true;
			}, 1000);
			return () => clearTimeout(timer);
		}
	});

	// Listen for manual trigger (from help menu)
	$effect(() => {
		if (!browser) return;

		function handleShowTutorial() {
			currentStep = 0;
			open = true;
		}

		window.addEventListener('show-tutorial', handleShowTutorial);
		return () => window.removeEventListener('show-tutorial', handleShowTutorial);
	});

	function nextStep() {
		if (currentStep < steps.length - 1) {
			currentStep++;
		} else {
			completeTutorial();
		}
	}

	function prevStep() {
		if (currentStep > 0) {
			currentStep--;
		}
	}

	function completeTutorial() {
		if (browser) {
			localStorage.setItem(STORAGE_KEY, Date.now().toString());
		}
		open = false;
		currentStep = 0;
	}

	function skipTutorial() {
		completeTutorial();
	}

	const step = $derived(steps[currentStep]);
	const isFirstStep = $derived(currentStep === 0);
	const isLastStep = $derived(currentStep === steps.length - 1);
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="max-w-md p-0 overflow-hidden">
		<!-- Gradient Header -->
		<div
			class="bg-gradient-to-r from-purple-600 via-indigo-500 to-pink-500 p-6 text-white"
		>
			<div class="flex items-center gap-3 mb-3">
				<div class="p-2 bg-white/20 rounded-lg">
					<step.icon class="h-6 w-6" />
				</div>
				<Dialog.Title class="text-xl font-bold text-white">{step.title}</Dialog.Title>
			</div>
			<Dialog.Description class="text-white/90">{step.description}</Dialog.Description>
		</div>

		<!-- Content -->
		<div class="p-6 space-y-4">
			<ul class="space-y-3">
				{#each step.tips as tip}
					<li class="flex items-start gap-3">
						<div class="mt-1 h-2 w-2 rounded-full bg-primary shrink-0"></div>
						<span class="text-sm text-muted-foreground">{tip}</span>
					</li>
				{/each}
			</ul>
		</div>

		<!-- Footer with step indicators and navigation -->
		<div class="border-t border-border p-4">
			<!-- Step Indicators -->
			<div class="flex justify-center gap-2 mb-4">
				{#each steps as _, i}
					<button
						type="button"
						class="h-2 w-2 rounded-full transition-colors {i === currentStep
							? 'bg-primary'
							: 'bg-muted-foreground/30'}"
						onclick={() => (currentStep = i)}
						aria-label="Go to step {i + 1}"
					></button>
				{/each}
			</div>

			<!-- Navigation Buttons -->
			<div class="flex items-center justify-between">
				<Button variant="ghost" size="sm" onclick={skipTutorial}>
					Skip
				</Button>

				<div class="flex items-center gap-2">
					{#if !isFirstStep}
						<Button variant="outline" size="sm" onclick={prevStep} class="gap-1">
							<ChevronLeft class="h-4 w-4" />
							Back
						</Button>
					{/if}

					<Button size="sm" onclick={nextStep} class="gap-1">
						{isLastStep ? 'Get Started' : 'Next'}
						{#if !isLastStep}
							<ChevronRight class="h-4 w-4" />
						{/if}
					</Button>
				</div>
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>
