<script lang="ts">
  import type { Cart } from '../api';
  import { link, navigate } from '../router.svelte';
  import ScreenshotImg from './ScreenshotImg.svelte';
  import PlayIcon from '@lucide/svelte/icons/play';
  import StarIcon from '@lucide/svelte/icons/star';

  let { cart }: { cart: Cart } = $props();

  function play(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    navigate(`/play/${cart.id}`);
  }
</script>

<a
  href="/cart/{cart.id}"
  use:link
  class="cart-notch group relative block aspect-square overflow-hidden bg-secondary ring-1 ring-white/5 transition-shadow hover:no-underline hover:ring-primary/30"
>
  <div class="absolute inset-0 transition-transform duration-300 ease-out group-hover:scale-[1.06]">
    <ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} alt={cart.title} />
  </div>

  <div class="absolute inset-0 bg-gradient-to-t from-black/95 via-black/10 to-transparent"></div>
  <div class="scanline-overlay pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-300 group-hover:opacity-50"></div>

  <div class="label-mono absolute top-2 left-2 rounded-sm bg-black/55 px-1.5 py-0.5 text-[10px] text-white/65 backdrop-blur-sm">
    #{cart.id.slice(0, 6)}
  </div>

  {#if cart.rating_count > 0}
    <div class="absolute top-2 right-2 flex items-center gap-1 rounded-full bg-black/55 px-2 py-0.5 text-xs font-medium text-white backdrop-blur-sm">
      <StarIcon class="size-3 fill-primary text-primary" />
      {cart.rating_avg.toFixed(1)}
    </div>
  {/if}

  <div class="pointer-events-none absolute inset-0 flex items-center justify-center opacity-0 transition-opacity group-hover:opacity-100">
    <button
      onclick={play}
      aria-label="Play {cart.title}"
      class="pointer-events-auto flex size-12 items-center justify-center rounded-full bg-white/95 text-black shadow-xl transition-transform hover:scale-105"
    >
      <PlayIcon class="ml-0.5 size-5" fill="currentColor" />
    </button>
  </div>

  <div class="absolute inset-x-0 bottom-0 p-3">
    <h3 class="truncate text-sm font-semibold text-white">{cart.title}</h3>
    <p class="label-mono mt-0.5 truncate text-[10px] text-white/50">{cart.author}</p>
  </div>
</a>
