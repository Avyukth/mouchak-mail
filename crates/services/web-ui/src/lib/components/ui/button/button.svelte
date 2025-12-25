<script lang="ts" module>
	import type { WithElementRef } from "bits-ui";
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from "svelte/elements";
	import { type VariantProps, tv } from "tailwind-variants";

	export const buttonVariants = tv({
		base: "ring-offset-background focus-visible:ring-ring relative inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm font-medium transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:relative [&_svg]:z-10 [&>*:not(.sr-only)]:relative [&>*:not(.sr-only)]:z-10 active:scale-[0.98]",
		variants: {
			variant: {
				default: "bg-primary text-primary-foreground hover:bg-primary/90 shadow-material hover:shadow-material-hover rounded-lg stripe-hover stripe-hover--light overflow-hidden",
				destructive: "bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-material hover:shadow-material-hover rounded-lg stripe-hover stripe-hover--light overflow-hidden",
				outline: "border border-input bg-background hover:bg-accent/50 hover:text-accent-foreground hover:border-primary/40 rounded-lg",
				secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80 shadow-sm hover:shadow-material rounded-lg",
				ghost: "hover:bg-accent/60 hover:text-accent-foreground rounded-lg hover:scale-[1.02]",
				link: "text-primary underline-offset-4 hover:underline hover:text-primary/80",
			},
			size: {
				default: "h-10 px-5 py-2",
				sm: "h-8 px-3 text-xs rounded-md",
				lg: "h-12 px-8 text-base rounded-xl",
				icon: "h-10 w-10 rounded-lg",
			},
		},
		defaultVariants: {
			variant: "default",
			size: "default",
		},
	});

	export type ButtonVariant = VariantProps<typeof buttonVariants>["variant"];
	export type ButtonSize = VariantProps<typeof buttonVariants>["size"];

	export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
		WithElementRef<HTMLAnchorAttributes> & {
			variant?: ButtonVariant;
			size?: ButtonSize;
		};
</script>

<script lang="ts">
	import { cn } from "$lib/utils.js";

	let {
		class: className,
		variant = "default",
		size = "default",
		ref = $bindable(null),
		href = undefined,
		type = "button",
		children,
		...restProps
	}: ButtonProps = $props();
</script>

{#if href}
	<a
		bind:this={ref}
		class={cn(buttonVariants({ variant, size }), className)}
		{href}
		{...restProps}
	>
		{@render children?.()}
	</a>
{:else}
	<button
		bind:this={ref}
		class={cn(buttonVariants({ variant, size }), className)}
		{type}
		{...restProps}
	>
		{@render children?.()}
	</button>
{/if}
