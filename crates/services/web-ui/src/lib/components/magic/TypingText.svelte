<!--
  @component
  TypingText - Typewriter effect with optional blinking cursor.
  
  @example
  ```svelte
  <TypingText text="Hello, World!" cursor={true} />
  ```
-->
<script lang="ts">
    import { onMount } from 'svelte';
    import { browser } from '$app/environment';

    /** Text to type out */
    export let text: string;
    /** Typing speed in milliseconds per character */
    export let speed: number = 50;
    /** Initial delay in milliseconds before typing starts */
    export let delay: number = 0;
    /** Whether to show blinking cursor */
    export let cursor: boolean = true;
    /** Additional CSS classes */
    let className: string = '';
    export { className as class };

    let displayedText = '';
    let showCursor = cursor;

    // Check for reduced motion preference
    const prefersReducedMotion = browser 
        ? window.matchMedia('(prefers-reduced-motion: reduce)').matches 
        : false;

    onMount(() => {
        if (prefersReducedMotion) {
            // Skip animation, show full text immediately
            displayedText = text;
            return;
        }

        let currentIndex = 0;
        
        const startTyping = () => {
            const interval = setInterval(() => {
                if (currentIndex < text.length) {
                    displayedText = text.slice(0, currentIndex + 1);
                    currentIndex++;
                } else {
                    clearInterval(interval);
                }
            }, speed);

            return () => clearInterval(interval);
        };

        const timeoutId = setTimeout(startTyping, delay);
        
        return () => {
            clearTimeout(timeoutId);
        };
    });
</script>

<span class="inline-flex items-center {className}" data-testid="typing-text">
    <span>{displayedText}</span>
    {#if showCursor}
        <span 
            class="ml-0.5 inline-block w-[2px] h-[1em] bg-current animate-[blink_1s_step-end_infinite]"
            aria-hidden="true"
        ></span>
    {/if}
</span>

<style>
    @keyframes blink {
        0%, 100% { opacity: 1; }
        50% { opacity: 0; }
    }
</style>
