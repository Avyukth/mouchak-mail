<script lang="ts">
	import type { WithElementRef } from "bits-ui";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "$lib/utils.js";

	type CardProps = WithElementRef<HTMLAttributes<HTMLDivElement>> & {
		interactive?: boolean;
		elevated?: boolean;
	};

	let {
		ref = $bindable(null),
		class: className,
		interactive = false,
		elevated = false,
		children,
		...restProps
	}: CardProps = $props();
</script>

<div
	bind:this={ref}
	class={cn(
		"bg-card text-card-foreground rounded-xl border border-border/60 shadow-material transition-all duration-300",
		elevated && "shadow-lg bg-gradient-to-b from-card to-card/95",
		interactive && [
			"cursor-pointer",
			"hover:shadow-material-hover hover:-translate-y-1",
			"hover:border-primary/30",
			"active:scale-[0.99] active:shadow-material-active"
		],
		className
	)}
	{...restProps}
>
	{@render children?.()}
</div>
