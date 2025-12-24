<script lang="ts" module>
	import { type VariantProps, tv } from "tailwind-variants";

	export const badgeVariants = tv({
		base: "focus:ring-ring relative inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-all duration-150 focus:outline-none focus:ring-2 focus:ring-offset-2 overflow-hidden [&>*]:relative [&>*]:z-10",
		variants: {
			variant: {
				default:
					"bg-primary text-primary-foreground hover:bg-primary/80 border-transparent stripe-hover stripe-hover--light",
				secondary:
					"bg-secondary text-secondary-foreground hover:bg-secondary/80 border-transparent stripe-hover stripe-hover--dark",
				destructive:
					"bg-destructive text-destructive-foreground hover:bg-destructive/80 border-transparent stripe-hover stripe-hover--light",
				outline: "text-foreground",
			},
		},
		defaultVariants: {
			variant: "default",
		},
	});

	export type BadgeVariant = VariantProps<typeof badgeVariants>["variant"];
</script>

<script lang="ts">
	import type { WithElementRef } from "bits-ui";
	import type { HTMLAnchorAttributes } from "svelte/elements";
	import { cn } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		href,
		class: className,
		variant = "default",
		children,
		...restProps
	}: WithElementRef<HTMLAnchorAttributes> & {
		variant?: BadgeVariant;
	} = $props();
</script>

<svelte:element
	this={href ? "a" : "span"}
	bind:this={ref}
	{href}
	class={cn(badgeVariants({ variant }), className)}
	{...restProps}
>
	{@render children?.()}
</svelte:element>
