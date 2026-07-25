<script lang="ts">
  import type { Cart } from '../api';
  import { link, navigate } from '../router.svelte';
  import ScreenshotImg from './ScreenshotImg.svelte';
  import PlayIcon from '@lucide/svelte/icons/play';
  import StarIcon from '@lucide/svelte/icons/star';

  let { cart, compact = false }: { cart: Cart; compact?: boolean } = $props();
  const creator = $derived(cart.owner ?? cart.author);

  function play(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    navigate(`/play/${cart.id}`);
  }
</script>

<a
  href="/cart/{cart.id}"
  use:link
  class="group block overflow-hidden rounded-lg border border-border bg-card text-foreground shadow-sm transition-[border-color,box-shadow,transform] duration-200 hover:-translate-y-0.5 hover:border-primary hover:text-foreground hover:no-underline hover:shadow-md"
>
  <div class="cart-notch relative aspect-square overflow-hidden bg-secondary">
    <ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} alt={cart.title} />
    <button
      onclick={play}
      aria-label="Play {cart.title}"
      class="ember-glow absolute right-2 bottom-2 flex size-9 items-center justify-center rounded-full bg-primary text-primary-foreground transition-transform hover:scale-105"
    >
      <PlayIcon class="ml-0.5 size-4" fill="currentColor" />
    </button>
  </div>
  <div class={compact ? 'p-3' : 'p-3.5'}>
    <h3 class="truncate font-display text-base font-semibold">{cart.title}</h3>
    <p class="label-mono mt-1 truncate text-[10px] text-muted-foreground">{creator}</p>
    {#if !compact && cart.description}
      <p class="mt-2 line-clamp-2 min-h-10 text-sm leading-snug text-muted-foreground">{cart.description}</p>
    {/if}
    <div class="mt-3 flex items-center gap-1.5 border-t border-[var(--border-subtle)] pt-3">
      <StarIcon class="size-3 fill-primary text-primary" />
      <span class="font-mono text-xs text-muted-foreground">{cart.rating_count ? cart.rating_avg.toFixed(1) : '—'}</span>
      <span class="ml-auto font-mono text-xs text-muted-foreground">{cart.plays.toLocaleString()} plays</span>
    </div>
  </div>
</a>
