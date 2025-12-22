<!--
  @component
  NumberCounter - Animated counting from 0 to target value using Svelte's tweened store.
  
  @example
  ```svelte
  <NumberCounter value={1000} duration={2000} prefix="$" />
  ```
-->
<script lang="ts">
    import { tweened } from "svelte/motion";
    import { cubicOut } from "svelte/easing";
    import { browser } from "$app/environment";

    /** Target value to count to */
    export let value: number;
    /** Animation duration in milliseconds */
    export let duration: number = 1000;
    /** Number of decimal places */
    export let decimals: number = 0;
    /** Prefix string (e.g., "$") */
    export let prefix: string = "";
    /** Suffix string (e.g., "%", "+") */
    export let suffix: string = "";
    /** Additional CSS classes */
    let className: string = "";
    export { className as class };

    // Check for reduced motion preference
    const prefersReducedMotion = browser
        ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
        : false;

    const displayed = tweened(0, {
        duration: prefersReducedMotion ? 0 : duration,
        easing: cubicOut,
    });

    // React to value changes
    $: displayed.set(value);

    // Format number with locale (thousands separators)
    function formatNumber(n: number): string {
        if (decimals > 0) {
            return n.toLocaleString(undefined, {
                minimumFractionDigits: decimals,
                maximumFractionDigits: decimals,
            });
        }
        return Math.round(n).toLocaleString();
    }
</script>

<span class="tabular-nums {className}" data-testid="number-counter">
    {prefix}{formatNumber($displayed)}{suffix}
</span>
