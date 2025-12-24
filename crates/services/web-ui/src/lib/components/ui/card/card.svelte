<script lang="ts">
	import type { WithElementRef } from "bits-ui";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "$lib/utils.js";

	type CardProps = WithElementRef<HTMLAttributes<HTMLDivElement>> & {
		interactive?: boolean;
	};

	let {
		ref = $bindable(null),
		class: className,
		interactive = false,
		children,
		...restProps
	}: CardProps = $props();
</script>

<div
	bind:this={ref}
	class={cn(
		"bg-card text-card-foreground rounded-lg border shadow-sm",
		interactive && "stripe-hover stripe-hover--dark cursor-pointer transition-all duration-150 hover:border-primary/50 hover:shadow-lg [&>*]:relative [&>*]:z-10",
		className
	)}
	{...restProps}
>
	{@render children?.()}
</div>
