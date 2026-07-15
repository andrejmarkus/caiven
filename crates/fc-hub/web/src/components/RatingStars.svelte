<script lang="ts">
  let {
    value,
    interactive = false,
    onrate,
  }: {
    value: number;
    interactive?: boolean;
    onrate?: (score: number) => void;
  } = $props();

  const stars = [1, 2, 3, 4, 5];
</script>

<span class="stars" class:interactive>
  {#each stars as n (n)}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <span
      class="star"
      class:filled={n <= Math.round(value)}
      role={interactive ? 'button' : undefined}
      tabindex={interactive ? 0 : undefined}
      onclick={() => interactive && onrate?.(n)}
      onkeydown={(e) => interactive && e.key === 'Enter' && onrate?.(n)}
    >★</span>
  {/each}
</span>

<style>
  .stars {
    display: inline-flex;
    gap: 0.1em;
  }
  .star {
    color: var(--border);
    font-size: 1.1em;
  }
  .star.filled {
    color: var(--accent);
  }
  .interactive .star {
    cursor: pointer;
  }
</style>
