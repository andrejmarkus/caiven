<script lang="ts">
  import type { Cart } from '../api';
  import { link, navigate } from '../router.svelte';
  import ScreenshotImg from './ScreenshotImg.svelte';
  import { Button } from '@caiven/ui/button';
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
  class="group block overflow-hidden rounded-lg border border-border bg-card text-foreground transition-colors hover:border-primary hover:text-foreground hover:no-underline"
>
  <div class="relative aspect-3/2 overflow-hidden bg-secondary">
    <ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} alt={cart.title} />
    <Button
      type="button"
      size="icon"
      onclick={play}
      aria-label="Play {cart.title}"
      class="absolute right-2 bottom-2"
    >
      <PlayIcon class="ml-0.5 size-4" fill="currentColor" />
    </Button>
  </div>
  <div class={compact ? 'p-3' : 'p-3.5'}>
    <h3 class="truncate text-base">{cart.title}</h3>
    <p class="mt-0.5 truncate text-sm text-muted-foreground">{creator}</p>
    {#if !compact && cart.description}
      <p class="mt-2 line-clamp-2 min-h-10 text-sm leading-snug text-muted-foreground">{cart.description}</p>
    {/if}
    <div class="mt-2 flex items-center gap-1.5 font-mono text-xs text-muted-foreground">
      {#if cart.rating_count}<StarIcon class="size-3 fill-primary text-primary" /><span>{cart.rating_avg.toFixed(1)}</span>{/if}
      <span class="ml-auto">{cart.plays.toLocaleString()} plays</span>
    </div>
  </div>
</a>
