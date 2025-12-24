<!--
  @component
  ShimmerButton - A button with a shimmering light traveling around the perimeter.

  @example
  ```svelte
  <ShimmerButton>Get Started</ShimmerButton>
  <ShimmerButton variant="secondary" size="lg">Learn More</ShimmerButton>
  ```
-->
<script lang="ts">
    import { browser } from "$app/environment";

    /** Button variant */
    export let variant: "primary" | "secondary" | "ghost" = "primary";
    /** Button size */
    export let size: "sm" | "md" | "lg" = "md";
    /** Shimmer color - uses theme variable by default */
    export let shimmerColor: string = "hsl(var(--shimmer-color) / 0.4)";
    /** Background color */
    export let background: string = "";
    /** Animation duration in seconds */
    export let duration: number = 2;
    /** Disabled state */
    export let disabled: boolean = false;
    /** Button type */
    export let type: "button" | "submit" | "reset" = "button";
    /** Additional CSS classes */
    let className: string = "";
    export { className as class };

    // Check for reduced motion preference
    const prefersReducedMotion = browser
        ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
        : false;

    const sizeClasses = {
        sm: "px-3 py-1.5 text-sm",
        md: "px-4 py-2 text-base",
        lg: "px-6 py-3 text-lg",
    };

    const variantClasses = {
        primary: "bg-primary text-primary-foreground hover:bg-primary/90",
        secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ghost: "bg-transparent hover:bg-accent hover:text-accent-foreground",
    };
</script>

<button
    {type}
    {disabled}
    class="shimmer-button relative overflow-hidden rounded-lg font-medium transition-colors
           inline-flex items-center justify-center
           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
           disabled:pointer-events-none disabled:opacity-50
           {sizeClasses[size]} {variantClasses[variant]} {className}"
    style="--shimmer-color: {shimmerColor}; --shimmer-duration: {duration}s; {background ? `background: ${background};` : ''}"
    data-testid="shimmer-button"
    on:click
    on:keydown
>
    {#if !prefersReducedMotion && !disabled}
        <span class="shimmer-effect absolute inset-0 pointer-events-none" aria-hidden="true"></span>
    {/if}
    <span class="relative z-10 inline-flex items-center">
        <slot />
    </span>
</button>

<style>
    .shimmer-effect {
        background: linear-gradient(
            90deg,
            transparent 0%,
            var(--shimmer-color, hsl(var(--shimmer-color) / 0.4)) 50%,
            transparent 100%
        );
        background-size: 200% 100%;
        animation: shimmer-move var(--shimmer-duration, 2s) ease-in-out infinite;
    }

    @keyframes shimmer-move {
        0% {
            background-position: 200% 0;
        }
        100% {
            background-position: -200% 0;
        }
    }

    /* Border shimmer effect */
    .shimmer-button::before {
        content: "";
        position: absolute;
        inset: 0;
        border-radius: inherit;
        padding: 2px;
        background: linear-gradient(
            90deg,
            transparent 0%,
            var(--shimmer-color, hsl(var(--shimmer-color) / 0.4)) 50%,
            transparent 100%
        );
        background-size: 200% 100%;
        -webkit-mask: linear-gradient(white 0 0) content-box, linear-gradient(white 0 0);
        mask: linear-gradient(white 0 0) content-box, linear-gradient(white 0 0);
        -webkit-mask-composite: xor;
        mask-composite: exclude;
        animation: shimmer-move var(--shimmer-duration, 2s) ease-in-out infinite;
        pointer-events: none;
    }

    /* Respect reduced motion preference */
    @media (prefers-reduced-motion: reduce) {
        .shimmer-effect,
        .shimmer-button::before {
            animation: none;
            background: transparent;
        }
    }
</style>
