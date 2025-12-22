<!--
  @component
  AnimatedGradient - Smoothly transitioning gradient background.
  
  @example
  ```svelte
  <AnimatedGradient fromColor="from-indigo-500" viaColor="via-purple-500" toColor="to-pink-500">
    <h1 class="text-white">Welcome</h1>
  </AnimatedGradient>
  ```
-->
<script lang="ts">
    import { browser } from "$app/environment";

    /** Starting gradient color class (e.g., "from-indigo-500") */
    export let fromColor: string;
    /** Middle gradient color class (optional) */
    export let viaColor: string = "";
    /** Ending gradient color class (e.g., "to-pink-500") */
    export let toColor: string;
    /** Animation direction */
    export let direction: "horizontal" | "vertical" | "diagonal" = "horizontal";
    /** Animation duration in seconds */
    export let duration: number = 3;
    /** Additional CSS classes */
    let className: string = "";
    export { className as class };

    // Check for reduced motion preference
    const prefersReducedMotion = browser
        ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
        : false;

    const animationClass = {
        horizontal: "animate-gradient-x",
        vertical: "animate-gradient-y",
        diagonal: "animate-gradient-xy",
    }[direction];
</script>

<div
    class="bg-gradient-to-r {fromColor} {viaColor} {toColor} bg-[length:200%_200%] {prefersReducedMotion
        ? ''
        : animationClass} {className}"
    style="animation-duration: {duration}s;"
    data-testid="animated-gradient"
>
    <slot />
</div>

<style>
    @keyframes gradient-x {
        0%,
        100% {
            background-position: 0% 50%;
        }
        50% {
            background-position: 100% 50%;
        }
    }
    @keyframes gradient-y {
        0%,
        100% {
            background-position: 50% 0%;
        }
        50% {
            background-position: 50% 100%;
        }
    }
    @keyframes gradient-xy {
        0%,
        100% {
            background-position: 0% 0%;
        }
        50% {
            background-position: 100% 100%;
        }
    }
    :global(.animate-gradient-x) {
        animation: gradient-x var(--tw-animate-duration, 3s) ease infinite;
    }
    :global(.animate-gradient-y) {
        animation: gradient-y var(--tw-animate-duration, 3s) ease infinite;
    }
    :global(.animate-gradient-xy) {
        animation: gradient-xy var(--tw-animate-duration, 3s) ease infinite;
    }
</style>
