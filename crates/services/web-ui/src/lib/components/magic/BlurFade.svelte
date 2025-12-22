<!--
  @component
  BlurFade - Blur + fade entrance animation.
  
  @example
  ```svelte
  <BlurFade direction="up" delay={200}>
    <h1>Welcome</h1>
  </BlurFade>
  ```
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { browser } from "$app/environment";

    /** Direction to fade from */
    export let direction: "up" | "down" | "left" | "right" | "center" = "down";
    /** Delay before animation in milliseconds */
    export let delay: number = 0;
    /** Animation duration in milliseconds */
    export let duration: number = 500;
    /** Initial blur amount in pixels */
    export let blur: number = 6;
    /** Additional CSS classes */
    let className: string = "";
    export { className as class };

    let visible = false;

    // Check for reduced motion preference
    const prefersReducedMotion = browser
        ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
        : false;

    // Transform values for each direction
    const transforms: Record<string, string> = {
        up: "translateY(20px)",
        down: "translateY(-20px)",
        left: "translateX(20px)",
        right: "translateX(-20px)",
        center: "scale(0.95)",
    };

    const initialTransform = transforms[direction] || transforms.down;

    onMount(() => {
        if (prefersReducedMotion) {
            visible = true;
            return;
        }

        const timeoutId = setTimeout(() => {
            visible = true;
        }, delay);

        return () => clearTimeout(timeoutId);
    });
</script>

<div
    class="blur-fade {className}"
    class:visible
    style="
        --blur-amount: {blur}px;
        --duration: {duration}ms;
        --initial-transform: {initialTransform};
    "
    data-testid="blur-fade"
>
    <slot />
</div>

<style>
    .blur-fade {
        opacity: 0;
        filter: blur(var(--blur-amount, 6px));
        transform: var(--initial-transform, translateY(-20px));
        transition:
            opacity var(--duration, 500ms) ease-out,
            filter var(--duration, 500ms) ease-out,
            transform var(--duration, 500ms) ease-out;
    }

    .blur-fade.visible {
        opacity: 1;
        filter: blur(0);
        transform: translateY(0) translateX(0) scale(1);
    }

    /* Respect reduced motion preference */
    @media (prefers-reduced-motion: reduce) {
        .blur-fade {
            opacity: 1;
            filter: none;
            transform: none;
            transition: none;
        }
    }
</style>
