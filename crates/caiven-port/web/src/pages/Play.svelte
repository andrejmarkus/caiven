<script lang="ts">
  import { api, type CartDetail } from '../api';
  import { CartPlayer } from '../player';
  import { playSessionId, rememberCart } from '../history';
  import { link } from '../router.svelte';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import MaximizeIcon from '@lucide/svelte/icons/maximize-2';
  import MinimizeIcon from '@lucide/svelte/icons/minimize-2';
  import VolumeIcon from '@lucide/svelte/icons/volume-2';
  import VolumeOffIcon from '@lucide/svelte/icons/volume-x';
  import RotateIcon from '@lucide/svelte/icons/rotate-ccw';

  let { id }: { id: string } = $props();
  let cart = $state<CartDetail | null>(null);
  let canvas = $state<HTMLCanvasElement | undefined>();
  let stage = $state<HTMLDivElement | undefined>();
  let touchContainer = $state<HTMLDivElement | undefined>();
  let loading = $state(true);
  let error = $state('');
  let fault = $state('');
  let fullscreen = $state(false);
  let muted = $state(false);
  let fps = $state(60);
  let player: CartPlayer | null = null;

  async function boot() {
    player?.stop(); player = null; loading = true; error = ''; fault = '';
    try {
      cart = await api.getCart(id);
      const res = await fetch(api.cartUrl(id));
      if (!res.ok) throw new Error(`failed to fetch cart (${res.status})`);
      const bytes = new Uint8Array(await res.arrayBuffer());
      loading = false;
      await new Promise((resolve) => setTimeout(resolve, 0));
      if (!canvas) throw new Error('canvas did not mount');
      player = await CartPlayer.load(canvas, bytes);
      player.setMuted(muted);
      if (touchContainer) player.mountTouchControls(touchContainer);
      player.start((message) => (fault = message), (value) => (fps = value));
      rememberCart(cart);
      void api.recordPlay(id, playSessionId());
    } catch (e) { error = e instanceof Error ? e.message : String(e); loading = false; }
  }
  function toggleMute() { muted = !muted; player?.setMuted(muted); }
  function toggleFullscreen() { if (!stage) return; document.fullscreenElement ? void document.exitFullscreen() : void stage.requestFullscreen(); }
  $effect(() => {
    id; boot();
    const onFull = () => (fullscreen = document.fullscreenElement === stage);
    document.addEventListener('fullscreenchange', onFull);
    return () => { document.removeEventListener('fullscreenchange', onFull); player?.stop(); player = null; };
  });
</script>

<div class="flex min-h-[calc(100vh-4rem)] flex-col bg-[#0d0d0d]">
  <div class="flex flex-wrap items-center gap-3 border-b border-void-800 px-4 py-3 md:px-7">
    <a href="/cart/{id}" use:link class="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"><ArrowLeftIcon class="size-4" />{cart?.title ?? 'Back to cart'}</a>
    {#if cart}<span class="font-mono text-xs text-muted-foreground">{cart.owner ?? cart.author} · v{cart.latest_version}</span>{/if}
    <div class="ml-auto flex items-center gap-2">
      <span class="label-mono mr-1 flex items-center gap-2 text-[10px] text-muted-foreground"><span class="size-2 rounded-full bg-primary shadow-[0_0_8px_var(--color-ember)]"></span>{fps} fps</span>
      <button onclick={toggleMute} aria-label={muted ? 'Unmute' : 'Mute'} class="flex size-9 items-center justify-center rounded-md border border-void-700 text-muted-foreground hover:bg-void-800">{#if muted}<VolumeOffIcon class="size-4" />{:else}<VolumeIcon class="size-4" />{/if}</button>
      <button onclick={boot} aria-label="Restart cart" class="flex size-9 items-center justify-center rounded-md border border-void-700 text-muted-foreground hover:bg-void-800"><RotateIcon class="size-4" /></button>
      <button onclick={toggleFullscreen} class="flex h-9 items-center gap-2 rounded-md border border-void-700 px-3 text-sm font-semibold text-foreground hover:bg-void-800">{#if fullscreen}<MinimizeIcon class="size-4" />Exit{:else}<MaximizeIcon class="size-4" />Fullscreen{/if}</button>
    </div>
  </div>
  {#if error}<div class="m-5 rounded-lg border border-destructive/50 bg-destructive/10 p-4 text-destructive">{error}</div>{/if}
  {#if loading}<div class="flex flex-1 items-center justify-center text-sm text-muted-foreground">Booting cart…</div>
  {:else}
    <div bind:this={stage} class="stage flex flex-1 items-center justify-center p-4 md:p-8">
      <div class="relative aspect-square w-[min(620px,72vh)] overflow-hidden rounded-lg bg-black shadow-2xl shadow-black/60">
        <canvas bind:this={canvas} width="128" height="128" class="block size-full" style="image-rendering: pixelated;"></canvas>
        <div class="scanline-overlay crt-vignette pointer-events-none absolute inset-0 opacity-65"></div>
        {#if fault}<div class="absolute inset-0 flex flex-col items-center justify-center bg-black/90 p-5 text-center"><strong class="font-mono text-sm text-destructive">Cart crashed</strong><p class="mt-2 font-mono text-xs text-white">{fault}</p></div>{/if}
        <div bind:this={touchContainer} class="touch-overlay pointer-events-none absolute inset-0"></div>
      </div>
    </div>
    <div class="flex flex-wrap justify-center gap-3 px-5 pb-7 text-sm text-muted-foreground">{#each [['← →','move'],['↑ ↓','aim'],['J / Z','A'],['K / X','B'],['Gamepad','supported'],['Touch','mobile']] as control}<span><kbd class="mr-1 rounded border border-void-700 px-2 py-1 font-mono text-xs">{control[0]}</kbd>{control[1]}</span>{/each}</div>
  {/if}
</div>

<style>
  .stage:fullscreen { height: 100vh; background: #000; }
  .touch-overlay { display: none; }
  @media (hover: none) and (pointer: coarse) { .touch-overlay { display: block; } }
  .touch-overlay :global(.touch-dpad), .touch-overlay :global(.touch-face) { position: absolute; bottom: 4%; display: grid; gap: 4px; pointer-events: auto; }
  .touch-overlay :global(.touch-dpad) { left: 3%; grid-template-columns: repeat(3, 48px); grid-template-rows: repeat(3, 48px); }
  .touch-overlay :global(.touch-face) { right: 3%; grid-template-columns: repeat(2, 52px); grid-auto-rows: 52px; }
  .touch-overlay :global(.touch-btn) { display: flex; align-items: center; justify-content: center; border: 1px solid rgb(255 255 255 / .35); border-radius: 8px; background: rgb(255 255 255 / .18); color: white; user-select: none; touch-action: none; }
  .touch-overlay :global(.a), .touch-overlay :global(.b) { border-radius: 50%; }
  .touch-overlay :global(.up) { grid-column: 2; grid-row: 1; } .touch-overlay :global(.left) { grid-column: 1; grid-row: 2; } .touch-overlay :global(.right) { grid-column: 3; grid-row: 2; } .touch-overlay :global(.down) { grid-column: 2; grid-row: 3; }
  .touch-overlay :global(.b) { grid-column: 1; } .touch-overlay :global(.a) { grid-column: 2; }
</style>
