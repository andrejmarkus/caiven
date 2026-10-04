<script lang="ts">
  import { api, type Cart } from '../api';
  import { CartPlayer } from '../player';
  import { applyAudio } from '../lib/audio-prefs.svelte';
  import ScreenshotImg from './ScreenshotImg.svelte';
  import { navigate } from '../router.svelte';

  let { cart }: { cart: Cart } = $props();
  let canvas = $state<HTMLCanvasElement | undefined>();
  let live = $state(false);
  let focused = $state(false);
  // The home screen mounts no d-pad, so a phone plays in the full player.
  const touch = matchMedia('(hover: none) and (pointer: coarse)').matches;

  // Boots the cart muted behind its screenshot; a click on the screen hands it the keyboard and sound.
  $effect(() => {
    const id = cart.id;
    const el = canvas;
    if (!el || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    let player: CartPlayer | null = null;
    let observer: IntersectionObserver | null = null;
    let gone = false;
    (async () => {
      try {
        const res = await fetch(api.cartUrl(id));
        if (!res.ok || gone) return;
        const bytes = new Uint8Array(await res.arrayBuffer());
        if (gone) return;
        // No save key: a demo run must not overwrite the player's real save.
        const loaded = await CartPlayer.load(el, bytes);
        if (gone) { loaded.stop(); return; }
        player = loaded;
        applyAudio(player);
        // A crashed demo hands the screen back to the screenshot and lets go of the keyboard.
        const crashed = () => { live = false; player?.stop(); el.tabIndex = -1; el.blur(); };
        player.start(crashed, undefined, false);
        live = true;
        observer = new IntersectionObserver(([entry]) => player?.setPaused(!entry.isIntersecting));
        observer.observe(el);
      } catch {
        // The screenshot stays up; the Play button still works.
      }
    })();
    return () => {
      gone = true;
      observer?.disconnect();
      player?.stop();
      live = false;
    };
  });
</script>

<div class="relative aspect-3/2 w-full overflow-hidden rounded-lg border bg-black transition-colors {focused ? 'border-primary' : 'border-border'}">
  <ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} alt="" />
  <canvas
    bind:this={canvas}
    width="192"
    height="128"
    aria-label={live ? `${cart.title}, running. ${touch ? 'Tap' : 'Click'} to play.` : `Play ${cart.title}`}
    onclick={() => (touch || !live) && navigate(`/play/${cart.id}`)}
    onfocus={() => (focused = true)}
    onblur={() => (focused = false)}
    class="attract-canvas absolute inset-0 block size-full cursor-pointer outline-none [image-rendering:pixelated]"
    class:live
  ></canvas>
  <div class="screen-filter pointer-events-none absolute inset-0"></div>
</div>
<p class="mt-3 text-sm text-muted-foreground" aria-live="polite">
  {#if focused}Arrows move. Z and X are the A and B buttons.{:else if live}{touch ? 'Tap the screen to play.' : 'Click the screen to play it here.'}{:else}&nbsp;{/if}
</p>

<style>
  .attract-canvas { opacity: 0; }
  /* Power-on: the live picture opens from a bright line, like a CRT warming up. */
  .attract-canvas.live { opacity: 1; animation: power-on 420ms cubic-bezier(0.16, 1, 0.3, 1); }
  @keyframes power-on {
    0% { opacity: 1; transform: scaleY(0.01); filter: brightness(3); }
    60% { transform: scaleY(1); filter: brightness(1.4); }
    100% { transform: scaleY(1); filter: brightness(1); }
  }
</style>
