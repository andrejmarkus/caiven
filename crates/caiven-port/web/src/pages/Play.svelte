<script lang="ts">
  import { api, ApiError, type CartDetail } from '../api';
  import { CartPlayer } from '../player';
  import { link } from '../router.svelte';

  let { id }: { id: string } = $props();

  let cart = $state<CartDetail | null>(null);
  let canvas = $state<HTMLCanvasElement | undefined>();
  let loading = $state(true);
  let error = $state('');
  let player: CartPlayer | null = null;

  async function boot() {
    loading = true;
    error = '';
    try {
      cart = await api.getCart(id);
      const res = await fetch(api.cartUrl(id));
      if (!res.ok) throw new Error(`failed to fetch cart (${res.status})`);
      const bytes = new Uint8Array(await res.arrayBuffer());
      loading = false;
      await new Promise((r) => setTimeout(r, 0)); // let canvas mount
      if (!canvas) throw new Error('canvas did not mount');
      player = await CartPlayer.load(canvas, bytes);
      player.start();
    } catch (e) {
      error = e instanceof ApiError ? e.message : e instanceof Error ? e.message : String(e);
      loading = false;
    }
  }

  $effect(() => {
    id;
    boot();
    return () => {
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
  <div class="stage" class:hidden={loading || error}>
    <canvas bind:this={canvas} width="128" height="128"></canvas>
  </div>
  {#if !loading && !error}
    <p class="muted hint">Arrows/WASD to move · J/Z = A · K/X = B · gamepad supported</p>
  {/if}
</div>

<style>
  .stage {
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
  .hint {
    text-align: center;
  }
</style>
