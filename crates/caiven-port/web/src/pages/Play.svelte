<script lang="ts">
  import { api, ApiError, type CartDetail } from '../api';
  import { CartPlayer } from '../player';
  import { link } from '../router.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as Alert from '$lib/components/ui/alert';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import Maximize2Icon from '@lucide/svelte/icons/maximize-2';
  import Minimize2Icon from '@lucide/svelte/icons/minimize-2';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';

  let { id }: { id: string } = $props();

  let cart = $state<CartDetail | null>(null);
  let canvas = $state<HTMLCanvasElement | undefined>();
  let stage = $state<HTMLDivElement | undefined>();
  let touchContainer = $state<HTMLDivElement | undefined>();
  let loading = $state(true);
  let error = $state('');
  let fault = $state('');
  let isFullscreen = $state(false);
  let player: CartPlayer | null = null;

  async function boot() {
    loading = true;
    error = '';
    fault = '';
    try {
      cart = await api.getCart(id);
      const res = await fetch(api.cartUrl(id));
      if (!res.ok) throw new Error(`failed to fetch cart (${res.status})`);
      const bytes = new Uint8Array(await res.arrayBuffer());
      loading = false;
      await new Promise((r) => setTimeout(r, 0)); // let canvas mount
      if (!canvas) throw new Error('canvas did not mount');
      player = await CartPlayer.load(canvas, bytes);
      if (touchContainer) player.mountTouchControls(touchContainer);
      player.start((message) => {
        fault = message;
      });
    } catch (e) {
      error = e instanceof ApiError ? e.message : e instanceof Error ? e.message : String(e);
      loading = false;
    }
  }

  function toggleFullscreen(): void {
    if (!stage) return;
    if (document.fullscreenElement) {
      void document.exitFullscreen();
    } else {
      void stage.requestFullscreen();
    }
  }

  function onFullscreenChange(): void {
    isFullscreen = document.fullscreenElement === stage;
  }

  $effect(() => {
    id;
    boot();
    document.addEventListener('fullscreenchange', onFullscreenChange);
    return () => {
      document.removeEventListener('fullscreenchange', onFullscreenChange);
      player?.stop();
      player = null;
    };
  });
</script>

<div class="container-page py-8">
  <a href="/cart/{id}" use:link class="mb-4 inline-flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground">
    <ArrowLeftIcon class="size-3.5" />
    Back to {cart?.title ?? 'cart'}
  </a>

  {#if error}
    <Alert.Root variant="destructive" class="mb-4">
      <CircleAlertIcon />
      <Alert.Description>{error}</Alert.Description>
    </Alert.Root>
  {/if}

  {#if loading}
    <p class="text-sm text-muted-foreground">Loading…</p>
  {/if}

  <div class="stage flex justify-center" class:hidden={loading || error} bind:this={stage}>
    <div class="relative">
      <canvas
        bind:this={canvas}
        width="128"
        height="128"
        class="animate-power-on block rounded-xl bg-black shadow-2xl shadow-black/50"
        style="image-rendering: pixelated; width: min(82vw, 560px); height: min(82vw, 560px);"
      ></canvas>
      <div class="crt-vignette scanline-overlay pointer-events-none absolute inset-0 rounded-xl opacity-70"></div>
      {#if fault}
        <div class="fault-overlay absolute inset-0 flex flex-col items-center justify-center gap-2 rounded-xl bg-black/90 p-4 text-center text-white">
          <p class="m-0 font-mono text-sm font-bold text-destructive">Cart crashed</p>
          <p class="m-0 max-w-full font-mono text-xs break-words">{fault}</p>
        </div>
      {/if}
      <div class="touch-overlay pointer-events-none absolute inset-0" bind:this={touchContainer}></div>
    </div>
  </div>

  {#if !loading && !error}
    <div class="mt-4 flex flex-wrap items-center justify-center gap-3 text-sm text-muted-foreground">
      <span>Arrows/WASD move · J/Z = A · K/X = B · gamepad supported</span>
      <Button variant="secondary" size="sm" onclick={toggleFullscreen}>
        {#if isFullscreen}
          <Minimize2Icon data-icon="inline-start" />
          Exit fullscreen
        {:else}
          <Maximize2Icon data-icon="inline-start" />
          Fullscreen
        {/if}
      </Button>
    </div>
  {/if}
</div>

<style>
  .stage.hidden {
    display: none;
  }
  .stage:fullscreen {
    align-items: center;
    height: 100vh;
    background: #000;
  }
  .stage:fullscreen :global(canvas) {
    width: min(90vw, 90vh) !important;
    height: min(90vw, 90vh) !important;
  }
  .touch-overlay {
    display: none;
  }
  @media (hover: none) and (pointer: coarse) {
    .touch-overlay {
      display: block;
    }
  }
  .touch-overlay :global(.touch-dpad),
  .touch-overlay :global(.touch-face) {
    position: absolute;
    bottom: 5%;
    display: grid;
    gap: 4px;
    pointer-events: auto;
  }
  .touch-overlay :global(.touch-dpad) {
    left: 4%;
    grid-template-columns: repeat(3, 44px);
    grid-template-rows: repeat(3, 44px);
  }
  .touch-overlay :global(.touch-face) {
    right: 4%;
    grid-template-columns: repeat(2, 52px);
    grid-auto-rows: 52px;
    align-items: center;
  }
  .touch-overlay :global(.touch-btn) {
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.15);
    border: 1px solid rgba(255, 255, 255, 0.35);
    border-radius: 8px;
    color: #fff;
    font-size: 1.1rem;
    user-select: none;
    touch-action: none;
  }
  .touch-overlay :global(.touch-btn.a),
  .touch-overlay :global(.touch-btn.b) {
    border-radius: 50%;
  }
  .touch-overlay :global(.up) {
    grid-column: 2;
    grid-row: 1;
  }
  .touch-overlay :global(.left) {
    grid-column: 1;
    grid-row: 2;
  }
  .touch-overlay :global(.right) {
    grid-column: 3;
    grid-row: 2;
  }
  .touch-overlay :global(.down) {
    grid-column: 2;
    grid-row: 3;
  }
  .touch-overlay :global(.b) {
    grid-column: 1;
    grid-row: 1;
  }
  .touch-overlay :global(.a) {
    grid-column: 2;
    grid-row: 1;
  }
</style>
