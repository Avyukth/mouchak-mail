<script lang="ts">
	import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import { cn } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		inset,
		children,
		...restProps
	}: DropdownMenuPrimitive.SubTriggerProps & {
		inset?: boolean;
	} = $props();
</script>

<DropdownMenuPrimitive.SubTrigger
	bind:ref
	class={cn(
		"data-[highlighted]:bg-accent data-[state=open]:bg-accent relative flex cursor-default select-none items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none transition-all duration-150 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:relative [&_svg]:z-10 [&>*]:relative [&>*]:z-10 stripe-hover stripe-hover--dark overflow-hidden",
		inset && "pl-8",
		className
	)}
	{...restProps}
>
	{@render children?.()}
	<ChevronRight class="ml-auto" />
</DropdownMenuPrimitive.SubTrigger>
