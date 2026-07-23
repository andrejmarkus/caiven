<script lang="ts">
  import { api, ApiError, type CartDetail } from '../api';
  import { CartPlayer } from '../player';
  import { link } from '../router.svelte';

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

<div class="container">
  <p><a href="/cart/{id}" use:link>&larr; back to {cart?.title ?? 'cart'}</a></p>
  {#if error}<p class="error">{error}</p>{/if}
  {#if loading}
    <p class="muted">loading…</p>
  {/if}
  <div class="stage" class:hidden={loading || error} bind:this={stage}>
    <canvas bind:this={canvas} width="128" height="128"></canvas>
    {#if fault}
      <div class="fault-overlay">
        <p class="fault-title">Cart crashed</p>
        <p class="fault-message">{fault}</p>
      </div>
    {/if}
    <div class="touch-overlay" bind:this={touchContainer}></div>
  </div>
  {#if !loading && !error}
    <p class="muted hint">
      Arrows/WASD to move · J/Z = A · K/X = B · gamepad supported
      <button class="fullscreen-btn" onclick={toggleFullscreen}>
        {isFullscreen ? 'exit fullscreen' : 'fullscreen'}
      </button>
    </p>
  {/if}
</div>

<style>
  .stage {
    position: relative;
    display: flex;
    justify-content: center;
    margin: 1rem 0;
  }
  .stage.hidden {
    display: none;
  }
  canvas {
    image-rendering: pixelated;
    width: min(90vw, 640px);
    height: min(90vw, 640px);
    border: 1px solid var(--border);
    background: #000;
  }
  .stage:fullscreen {
    align-items: center;
    height: 100vh;
    background: #000;
  }
  .stage:fullscreen canvas {
    width: min(90vw, 90vh);
    height: min(90vw, 90vh);
  }
  .hint {
    text-align: center;
  }
  .fullscreen-btn {
    margin-left: 0.5rem;
    padding: 0.15rem 0.6rem;
    cursor: pointer;
  }
  .fault-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1rem;
    background: rgba(0, 0, 0, 0.85);
    color: #fff;
    text-align: center;
  }
  .fault-title {
    color: #f66;
    font-weight: bold;
    margin: 0;
  }
  .fault-message {
    font-family: monospace;
    font-size: 0.85rem;
    margin: 0;
    word-break: break-word;
  }
  .touch-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
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
