<script lang="ts">
  import StarIcon from '@lucide/svelte/icons/star';

  let {
    value,
    interactive = false,
    size = 14,
    onrate,
  }: {
    value: number;
    interactive?: boolean;
    size?: number;
    onrate?: (score: number) => void;
  } = $props();

  const stars = [1, 2, 3, 4, 5];
</script>

<span class="inline-flex items-center gap-0.5">
  {#each stars as n (n)}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <span
      class={interactive ? 'cursor-pointer' : ''}
      role={interactive ? 'button' : undefined}
      tabindex={interactive ? 0 : undefined}
      onclick={() => interactive && onrate?.(n)}
      onkeydown={(e) => interactive && e.key === 'Enter' && onrate?.(n)}
    >
      <StarIcon {size} class={n <= Math.round(value) ? 'fill-primary text-primary' : 'text-border'} />
    </span>
  {/each}
</span>
