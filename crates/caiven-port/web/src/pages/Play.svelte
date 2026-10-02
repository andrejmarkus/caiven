<script lang="ts">
  import { tick } from 'svelte';
  import { api, type CartDetail } from '../api';
  import { CartPlayer } from '../player';
  import { playSessionId, rememberCart } from '../history';
  import { link, setTitle } from '../router.svelte';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import MaximizeIcon from '@lucide/svelte/icons/maximize-2';
  import MinimizeIcon from '@lucide/svelte/icons/minimize-2';
  import AudioControls from '../components/AudioControls.svelte';
  import { applyAudio } from '../lib/audio-prefs.svelte';
  import RotateIcon from '@lucide/svelte/icons/rotate-ccw';
  import CodeIcon from '@lucide/svelte/icons/code-xml';
  import GitForkIcon from '@lucide/svelte/icons/git-fork';
  import { buttonVariants } from '@caiven/ui/button';

  let { id }: { id: string } = $props();
  let cart = $state<CartDetail | null>(null);
  let canvas = $state<HTMLCanvasElement | undefined>();
  let stage = $state<HTMLDivElement | undefined>();
  let touchContainer = $state<HTMLDivElement | undefined>();
  let loading = $state(true);
  let error = $state('');
  let fault = $state('');
  let fullscreen = $state(false);
  let fps = $state(60);
  let player: CartPlayer | null = null;
  let bootGeneration = 0;
  let qualified = false;
  // Seconds of play after the first button press: an idle or background tab
  // never counts. Long enough to have actually played a tiny game.
  const QUALIFIED_PLAY_SECONDS = 20;

  async function boot() {
    const generation = ++bootGeneration;
    const cartId = id;
    player?.stop(); player = null; loading = true; error = ''; fault = ''; qualified = false;
    try {
      const loadedCart = await api.getCart(cartId);
      if (generation !== bootGeneration) return;
      const res = await fetch(api.cartUrl(cartId));
      if (!res.ok) throw new Error(`failed to fetch cart (${res.status})`);
      const bytes = new Uint8Array(await res.arrayBuffer());
      if (generation !== bootGeneration) return;
      cart = loadedCart;
      loading = false;
      await tick();
      if (generation !== bootGeneration) return;
      if (!canvas) throw new Error('canvas did not mount');
      const loadedPlayer = await CartPlayer.load(canvas, bytes, cartId);
      if (generation !== bootGeneration) { loadedPlayer.stop(); return; }
      player = loadedPlayer;
      applyAudio(player);
      if (touchContainer) player.mountTouchControls(touchContainer);
      const running = loadedPlayer;
      player.start((message) => (fault = message), (value) => {
        fps = value;
        if (!qualified && !fault && generation === bootGeneration && running.engagedSeconds >= QUALIFIED_PLAY_SECONDS) {
          qualified = true;
          void api.recordFunnel(cartId, 'qualified_play').catch(() => {});
        }
      });
      rememberCart(cart);
      // A metrics request must not interrupt an already running game.
      void api.recordPlay(cartId, playSessionId()).catch(() => {});
    } catch (e) {
      if (generation !== bootGeneration) return;
      error = e instanceof Error ? e.message : String(e); loading = false;
    }
  }
  $effect(() => { if (cart) setTitle(`Play ${cart.title}`); });
  let fakeFullscreen = $state(false);
  function toggleFullscreen() {
    if (!stage) return;
    // iPhone Safari has no element fullscreen API; cover the viewport instead.
    if (!stage.requestFullscreen) { fullscreen = fakeFullscreen = !fakeFullscreen; return; }
    document.fullscreenElement ? void document.exitFullscreen() : void stage.requestFullscreen();
  }
  $effect(() => {
    id; boot();
    const onFull = () => (fullscreen = document.fullscreenElement === stage);
    document.addEventListener('fullscreenchange', onFull);
    return () => { ++bootGeneration; document.removeEventListener('fullscreenchange', onFull); player?.stop(); player = null; };
  });
</script>

<div class="flex min-h-[calc(100dvh-8rem-2px-env(safe-area-inset-top)-env(safe-area-inset-bottom))] md:min-h-[calc(100dvh-4rem)] flex-col bg-[#0d0d0d]">
  <div class="flex flex-wrap items-center gap-3 border-b border-void-800 px-4 py-3 md:px-7">
    <a href="/cart/{id}" use:link class="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"><ArrowLeftIcon class="size-4" />{cart?.title ?? 'Back to cart'}</a>
    {#if cart}<span class="font-mono text-xs text-muted-foreground">{cart.owner ?? cart.author} · v{cart.latest_version}</span>{/if}
    <div class="ml-auto flex items-center gap-2">
      {#if cart?.remixable}<a href="/remix/{id}" use:link class={buttonVariants({ size: 'sm', class: 'ember-glow' })}><CodeIcon class="size-4" />Remix this</a>{/if}
      <span class="label-mono mr-1 flex items-center gap-2 text-[10px] text-muted-foreground"><span class="size-2 rounded-full bg-primary shadow-[0_0_8px_var(--color-ember)]"></span>{fps} fps</span>
      <AudioControls onchange={() => applyAudio(player)} />
      <button onclick={boot} aria-label="Restart cart" class="flex size-9 items-center justify-center rounded-md border border-void-700 text-muted-foreground hover:bg-void-800"><RotateIcon class="size-4" /></button>
      <button onclick={toggleFullscreen} aria-label={fullscreen ? 'Exit' : 'Fullscreen'} class="flex h-9 items-center gap-2 rounded-md border border-void-700 px-3 text-sm font-semibold text-foreground hover:bg-void-800">{#if fullscreen}<MinimizeIcon class="size-4" /><span class="hidden sm:inline">Exit</span>{:else}<MaximizeIcon class="size-4" /><span class="hidden sm:inline">Fullscreen</span>{/if}</button>
    </div>
  </div>
  {#if cart?.parent_cart_id}
    <div class="flex flex-wrap items-center gap-2 border-b border-void-800 px-4 py-2 text-sm text-muted-foreground md:px-7" data-testid="remix-lineage">
      <GitForkIcon class="size-4 text-primary" />
      {#if cart.parent}<span>@{cart.owner ?? cart.author} remixed <a href="/play/{cart.parent.id}" use:link class="text-foreground hover:text-primary">{cart.parent.title}</a>{#if cart.parent.owner}{' by '}<a href="/author/{cart.parent.owner}" use:link class="text-foreground hover:text-primary">@{cart.parent.owner}</a>{/if}</span>
      {:else}<span>Remixed from a cart that was removed</span>{/if}
      {#if cart.description}<span class="truncate italic">“{cart.description}”</span>{/if}
    </div>
  {/if}
  {#if error}<div class="m-5 rounded-lg border border-destructive/50 bg-destructive/10 p-4 text-destructive">{error}</div>{/if}
  {#if loading}<div class="flex flex-1 items-center justify-center text-sm text-muted-foreground">Booting cart…</div>
  {:else}
    <div bind:this={stage} class:fake-fullscreen={fakeFullscreen} class="stage flex flex-1 items-center justify-center p-4 md:p-8">
      {#if fakeFullscreen}<button onclick={toggleFullscreen} aria-label="Exit fullscreen" class="absolute top-[max(0.75rem,env(safe-area-inset-top))] right-[max(0.75rem,env(safe-area-inset-right))] z-10 flex size-9 items-center justify-center rounded-md border border-void-700 bg-black/60 text-muted-foreground"><MinimizeIcon class="size-4" /></button>{/if}
      <div class="relative aspect-3/2 w-[min(100%,1152px,120dvh)] overflow-hidden rounded-lg bg-black shadow-2xl shadow-black/60">
        <canvas bind:this={canvas} width="192" height="128" class="block size-full touch-manipulation" style="image-rendering: pixelated;"></canvas>
        <div class="screen-filter pointer-events-none absolute inset-0"></div>
        {#if fault}<div class="absolute inset-0 flex flex-col items-center justify-center bg-black/90 p-5 text-center"><strong class="font-mono text-sm text-destructive">Cart crashed</strong><p class="mt-2 font-mono text-xs text-white">{fault}</p></div>{/if}
        <div bind:this={touchContainer} class="touch-overlay pointer-events-none absolute inset-0"></div>
      </div>
    </div>
    <div class="flex flex-wrap justify-center gap-3 px-5 pb-5 text-sm text-muted-foreground" data-testid="controls">{#each [['← →','move'],['↑ ↓','aim'],['Z / Space','A button'],['X','B button'],['Gamepad','supported'],['Touch','mobile']] as control}<span><kbd class="mr-1 rounded border border-void-700 px-2 py-1 font-mono text-xs">{control[0]}</kbd>{control[1]}</span>{/each}</div>
    {#if cart?.remixable}
      <div class="mx-auto mb-7 flex w-[min(620px,calc(100%-2rem))] flex-wrap items-center gap-3 rounded-lg border border-primary/30 bg-primary/5 px-4 py-3" data-testid="remix-invite">
        <div class="min-w-0 flex-1">
          <p class="font-semibold text-foreground">Your turn. Change one number and make it yours.</p>
          <p class="text-xs text-muted-foreground">Edit the real code right here. No install{cart.remix_count ? ` · ${cart.remix_count} ${cart.remix_count === 1 ? 'remix' : 'remixes'} so far` : ''}.</p>
        </div>
        <a href="/remix/{id}" use:link class={buttonVariants({ size: 'sm' })}><CodeIcon class="size-4" />Remix it</a>
      </div>
    {/if}
  {/if}
</div>

<style>
  .stage:fullscreen { height: 100vh; background: #000; }
  .stage.fake-fullscreen { position: fixed; inset: 0; z-index: 100; background: #000; }
  .stage:fullscreen > div, .stage.fake-fullscreen > div { width: min(100%, (100dvh - 4rem) * 1.5); }
</style>
