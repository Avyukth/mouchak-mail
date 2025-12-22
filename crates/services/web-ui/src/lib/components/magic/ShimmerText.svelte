<!--
  @component
  ShimmerText - Shimmering shine effect across text.
  
  @example
  ```svelte
  <ShimmerText text="Premium Feature" />
  ```
-->
<script lang="ts">
    import { browser } from "$app/environment";

    /** Text to display */
    export let text: string;
    /** Shimmer highlight color */
    export let shimmerColor: string = "white";
    /** Animation duration in seconds */
    export let duration: number = 2;
    /** Additional CSS classes */
    let className: string = "";
    export { className as class };

    // Check for reduced motion preference
    const prefersReducedMotion = browser
        ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
        : false;
</script>

<span
    class="relative inline-block {prefersReducedMotion
        ? ''
        : 'shimmer-container'} {className}"
    style="--shimmer-duration: {duration}s; --shimmer-color: {shimmerColor};"
    data-testid="shimmer-text"
>
    {text}
</span>

<style>
    .shimmer-container {
        background: linear-gradient(
            90deg,
            currentColor 0%,
            var(--shimmer-color, white) 50%,
            currentColor 100%
        );
        background-size: 200% 100%;
        -webkit-background-clip: text;
        background-clip: text;
        color: transparent;
        animation: shimmer var(--shimmer-duration, 2s) ease-in-out infinite;
    }

    @keyframes shimmer {
        0% {
            background-position: 200% 0;
        }
        100% {
            background-position: -200% 0;
        }
    }
</style>
