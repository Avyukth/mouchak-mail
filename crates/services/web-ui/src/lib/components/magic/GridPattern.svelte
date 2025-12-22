<!--
  @component
  GridPattern - Animated dot or line grid pattern background.
  
  @example
  ```svelte
  <GridPattern pattern="dots" opacity={0.1}>
    <div class="relative z-10">Content</div>
  </GridPattern>
  ```
-->
<script lang="ts">
    import { browser } from "$app/environment";

    /** Grid pattern type */
    export let pattern: "dots" | "lines" | "small-dots" = "dots";
    /** Grid color class (e.g., "text-muted-foreground") */
    export let color: string = "text-muted-foreground";
    /** Grid opacity (0.0 to 1.0) */
    export let opacity: number = 0.1;
    /** Whether to animate the pattern */
    export let animated: boolean = false;
    /** Use gradient mask (fades at edges) */
    export let masked: boolean = false;
    /** Additional CSS classes */
    let className: string = "";
    export { className as class };

    // Check for reduced motion preference
    const prefersReducedMotion = browser
        ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
        : false;

    const patternClasses: Record<string, string> = {
        dots: "bg-[radial-gradient(circle,currentColor_1px,transparent_1px)] bg-[length:24px_24px]",
        lines: "bg-[linear-gradient(to_right,currentColor_1px,transparent_1px),linear-gradient(to_bottom,currentColor_1px,transparent_1px)] bg-[length:24px_24px]",
        "small-dots":
            "bg-[radial-gradient(circle,currentColor_0.5px,transparent_0.5px)] bg-[length:16px_16px]",
    };

    const patternClass = patternClasses[pattern] || patternClasses["dots"];
    const animationClass =
        animated && !prefersReducedMotion ? "animate-pulse" : "";
    const maskClass = masked
        ? "[mask-image:radial-gradient(ellipse_at_center,black_30%,transparent_70%)]"
        : "";
</script>

<div class="relative {className}" data-testid="grid-pattern">
    <div
        class="absolute inset-0 {patternClass} {color} {animationClass} {maskClass}"
        style="opacity: {opacity};"
        aria-hidden="true"
    ></div>
    <div class="relative z-10">
        <slot />
    </div>
</div>
