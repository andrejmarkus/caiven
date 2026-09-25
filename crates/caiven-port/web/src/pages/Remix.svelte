<script lang="ts">
  import { tick } from 'svelte';
  import { api, ApiError, type Cart, type CartDetail, type FunnelEvent } from '../api';
  import { CartPlayer } from '../player';
  import { currentUser, setUser } from '../stores.svelte';
  import { link, navigate, route } from '../router.svelte';
  import { parseCav, luaSource, withLuaSource, type Cav } from '../lib/cav.js';
  import { findConstants, setConstant, parseLuaError, errorHint } from '../lib/remix.js';
  import { draftKey as draftKeyFor, markPendingPublish, clearPendingPublish } from '../lib/pending-publish';
  import { Button, buttonVariants } from '@caiven/ui/button';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import PlayIcon from '@lucide/svelte/icons/play';
  import RotateIcon from '@lucide/svelte/icons/rotate-ccw';
  import UploadIcon from '@lucide/svelte/icons/upload-cloud';
  import CopyIcon from '@lucide/svelte/icons/copy';

  let { id }: { id: string } = $props();

  type RunError = { line: number | null; detail: string; hint: string | null; runtime: boolean };
  type Draft = { source: string; title: string; description: string; remixable: boolean; saved_at: string };

  const LINE_HEIGHT = 20;
  // Long enough to skip half-typed words, short enough to feel live.
  const AUTO_RUN_MS = 700;
  const draftKey = $derived(draftKeyFor(id));

  let cart = $state<CartDetail | null>(null);
  let cav: Cav | null = null;
  let original = $state('');
  let source = $state('');
  let ranSource = $state<string | null>(null);
  let runError = $state<RunError | null>(null);
  let loading = $state(true);
  let error = $state('');
  let restored = $state(false);
  let canvas = $state<HTMLCanvasElement | undefined>();
  let touchContainer = $state<HTMLDivElement | undefined>();
  let editor = $state<HTMLTextAreaElement | undefined>();
  let scrollTop = $state(0);
  let player: CartPlayer | null = null;
  let bootGeneration = 0;
  let reportedRan = false;
  // Back from the account wall: this person's steps were already counted
  // under their anonymous key, so the logged-in key must not count them again.
  let resumed = false;
  // Last source handed to the VM, so a broken edit isn't retried every pause.
  let attempted = '';

  let publishOpen = $state(false);
  let title = $state('');
  let description = $state('');
  let remixable = $state(true);
  let publishing = $state(false);
  let publishError = $state('');
  let published = $state<Cart | null>(null);
  let copied = $state(false);
  let resend = $state<'idle' | 'sending' | 'sent' | 'failed'>('idle');

  const changed = $derived(source !== original);
  const runOk = $derived(ranSource === source && !runError);
  const canPublish = $derived(changed && runOk);
  const constants = $derived(findConstants(source));
  const lineCount = $derived(source.split('\n').length);
  const shareUrl = $derived(published ? `${window.location.origin}/play/${published.id}` : '');
  const needsVerify = $derived(!!currentUser.value?.email && !currentUser.value.email_verified);

  function track(event: FunnelEvent) {
    if (!resumed) void api.recordFunnel(id, event).catch(() => {});
  }

  function readDraft(): Draft | null {
    try {
      const raw = localStorage.getItem(draftKey);
      return raw ? (JSON.parse(raw) as Draft) : null;
    } catch {
      return null;
    }
  }

  function saveDraft() {
    try {
      if (!changed) { localStorage.removeItem(draftKey); return; }
      const draft: Draft = { source, title, description, remixable, saved_at: new Date().toISOString() };
      localStorage.setItem(draftKey, JSON.stringify(draft));
    } catch {
      // Storage blocked: the edit still lives in this tab.
    }
  }

  function clearDraft() {
    try { localStorage.removeItem(draftKey); } catch { /* nothing to clear */ }
  }

  async function boot() {
    const generation = ++bootGeneration;
    player?.stop(); player = null; loading = true; error = ''; runError = null; published = null;
    try {
      const detail = await api.getCart(id);
      if (generation !== bootGeneration) return;
      cart = detail;
      if (!detail.remixable) { loading = false; return; }
      const res = await fetch(api.cartUrl(id));
      if (!res.ok) throw new Error(`failed to fetch cart (${res.status})`);
      const parsed = parseCav(new Uint8Array(await res.arrayBuffer()));
      const text = luaSource(parsed);
      if (text === null) throw new Error('This cart has no Lua source to remix.');
      if (generation !== bootGeneration) return;
      cav = parsed;
      original = text;
      const draft = readDraft();
      restored = !!draft && draft.source !== text;
      source = restored && draft ? draft.source : text;
      title = draft?.title || `${detail.title} remix`.slice(0, 64);
      description = draft?.description ?? '';
      remixable = draft?.remixable ?? true;
      loading = false;
      await tick();
      if (!canvas) throw new Error('canvas did not mount');
      const loaded = await CartPlayer.load(canvas, withLuaSource(parsed, text), null);
      if (generation !== bootGeneration) { loaded.stop(); return; }
      player = loaded;
      ranSource = text;
      if (touchContainer) player.mountTouchControls(touchContainer);
      player.start(onFault);
      resumed = route.search.get('publish') === '1';
      track('remix_opened');
      if (restored) run(false);
      if (resumed) await openPublish();
    } catch (e) {
      if (generation !== bootGeneration) return;
      error = e instanceof Error ? e.message : String(e);
      loading = false;
    }
  }

  function onFault(message: string) {
    const parsed = parseLuaError(message);
    runError = { ...parsed, hint: errorHint(parsed.detail), runtime: true };
  }

  function run(focusGame = true) {
    if (!player || !cav) return;
    attempted = source;
    let bytes: Uint8Array;
    try {
      bytes = withLuaSource(cav, source);
    } catch (e) {
      runError = { line: null, detail: e instanceof Error ? e.message : String(e), hint: null, runtime: false };
      return;
    }
    const failure = player.reload(bytes);
    if (failure) {
      const parsed = parseLuaError(failure);
      runError = { ...parsed, hint: errorHint(parsed.detail), runtime: false };
      if (parsed.line) scrollToLine(parsed.line);
      return;
    }
    runError = null;
    ranSource = source;
    if (changed && !reportedRan) {
      reportedRan = true;
      track('remix_ran');
    }
    if (focusGame) canvas?.focus();
  }

  function scrollToLine(line: number) {
    if (!editor) return;
    editor.scrollTop = Math.max(0, (line - 4) * LINE_HEIGHT);
  }

  function selectConstant(line: number) {
    if (!editor) return;
    const lines = source.split('\n');
    const start = lines.slice(0, line - 1).reduce((n, l) => n + l.length + 1, 0);
    const text = lines[line - 1];
    const at = text.indexOf('=') + 1;
    const value = /-?\d+(?:\.\d+)?/.exec(text.slice(at));
    editor.focus();
    if (value) editor.setSelectionRange(start + at + value.index, start + at + value.index + value[0].length);
    scrollToLine(line);
  }

  function tweak(line: number, value: string) {
    const next = setConstant(source, line, value);
    if (next === source) return;
    source = next;
    run(false);
  }

  function reset() {
    source = original;
    clearDraft();
    restored = false;
    run(false);
  }

  function onEditorKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); run(); }
  }

  async function openPublish() {
    if (!canPublish) return;
    track('publish_started');
    saveDraft();
    markPendingPublish(id);
    if (!currentUser.value) {
      // Most people reaching this wall are new, so it opens on sign-up.
      navigate(`/register?next=${encodeURIComponent(`/remix/${id}?publish=1`)}`);
      return;
    }
    publishOpen = true;
    publishError = '';
  }

  async function resendVerification() {
    resend = 'sending';
    try { await api.resendVerification(); resend = 'sent'; } catch { resend = 'failed'; }
  }

  // Confirming happens in another tab; coming back here should unlock Publish.
  function refreshUser() {
    if (needsVerify) api.me().then(setUser).catch(() => {});
  }

  async function screenshot(): Promise<Blob | null> {
    if (!canvas) return null;
    return new Promise((resolve) => canvas!.toBlob((blob) => resolve(blob), 'image/png'));
  }

  async function publish(e: Event) {
    e.preventDefault();
    if (!cav || !cart || !currentUser.value || !canPublish) return;
    publishing = true; publishError = '';
    try {
      const bytes = withLuaSource(cav, source, { title, author: currentUser.value.username });
      const file = new File([bytes as BlobPart], 'remix.cav', { type: 'application/octet-stream' });
      const shot = await screenshot();
      const created = await api.createCart(file, {
        title, description, tags: cart.tags, remixable, parent_cart_id: cart.id,
      });
      if (shot) await api.uploadScreenshot(created.id, shot).catch(() => {});
      clearDraft();
      clearPendingPublish();
      published = created;
      publishOpen = false;
    } catch (err) {
      if (err instanceof ApiError && err.status === 403 && needsVerify) {
        publishError = "Your email isn't confirmed yet. Click the link in the email, then press Publish again.";
      } else {
        publishError = err instanceof Error ? err.message : 'Publish failed';
      }
    } finally {
      publishing = false;
    }
  }

  async function copyShare() {
    try { await navigator.clipboard.writeText(shareUrl); copied = true; setTimeout(() => (copied = false), 1500); } catch { /* clipboard blocked */ }
  }

  $effect(() => {
    id; boot();
    return () => { ++bootGeneration; player?.stop(); player = null; };
  });

  $effect(() => {
    const text = source;
    if (loading) return;
    const timer = setTimeout(() => {
      if (text !== ranSource && text !== attempted) run(false);
    }, AUTO_RUN_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    source; title; description; remixable;
    if (!loading && cav) saveDraft();
  });

  $effect(() => {
    window.addEventListener('focus', refreshUser);
    return () => window.removeEventListener('focus', refreshUser);
  });
</script>

<div class="flex min-h-[calc(100vh-4rem)] flex-col bg-[#0d0d0d]">
  <div class="flex flex-wrap items-center gap-3 border-b border-void-800 px-4 py-3 md:px-7">
    <a href="/play/{id}" use:link class="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"><ArrowLeftIcon class="size-4" />Back to game</a>
    {#if cart}<span class="text-sm text-foreground">Remixing <strong>{cart.title}</strong> <span class="text-muted-foreground">by @{cart.owner ?? cart.author}</span></span>{/if}
    <div class="ml-auto flex items-center gap-2">
      {#if changed}<Button variant="secondary" size="sm" onclick={reset}><RotateIcon class="size-4" />Start over</Button>{/if}
      <Button size="sm" disabled={!canPublish || publishing || !!published} onclick={openPublish} title={canPublish ? 'Publish your version' : changed ? 'Run your change first' : 'Change something first'}><UploadIcon class="size-4" />Publish my version</Button>
    </div>
  </div>

  {#if error}<div class="m-5 rounded-lg border border-destructive/50 bg-destructive/10 p-4 text-destructive">{error}</div>{/if}
  {#if loading}<div class="flex flex-1 items-center justify-center text-sm text-muted-foreground">Loading the game's code…</div>
  {:else if cart && !cart.remixable}
    <div class="container-page py-16 text-center">
      <h1 class="text-2xl font-semibold">This cart isn't open for remixing</h1>
      <p class="mt-2 text-muted-foreground">Its creator keeps the source to themselves. You can still play it.</p>
      <a href="/play/{id}" use:link class={buttonVariants({ class: 'mt-6' })}><PlayIcon fill="currentColor" />Play</a>
    </div>
  {:else if !error}
    {#if published}
      <div class="border-b border-primary/40 bg-primary/10 px-4 py-4 md:px-7" role="status">
        <p class="font-semibold">Published. It's yours now.</p>
        <div class="mt-2 flex flex-wrap items-center gap-2">
          <code class="rounded bg-black/40 px-2 py-1 text-sm">{shareUrl}</code>
          <Button size="sm" variant="secondary" onclick={copyShare}><CopyIcon class="size-4" />{copied ? 'Copied' : 'Copy link'}</Button>
          <a href="/play/{published.id}" use:link class={buttonVariants({ size: 'sm' })}>Play your version</a>
          <a href="/cart/{published.id}" use:link class={buttonVariants({ size: 'sm', variant: 'secondary' })}>Cart page</a>
        </div>
      </div>
    {/if}
    {#if publishOpen}
      <form onsubmit={publish} class="border-b border-void-800 bg-card px-4 py-4 md:px-7" aria-label="Publish your remix">
        <div class="flex flex-wrap items-end gap-4">
          <label class="block min-w-[220px] flex-1 text-sm font-semibold">Title<input bind:value={title} maxlength={64} required class="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 font-normal" /></label>
          <label class="block min-w-[220px] flex-[2] text-sm font-semibold">What did you change?<input bind:value={description} maxlength={512} placeholder="Doubled the speed" class="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 font-normal" /></label>
          <label class="flex items-center gap-2 text-sm"><input type="checkbox" bind:checked={remixable} />Let others remix my version</label>
          <Button type="submit" disabled={publishing}>{publishing ? 'Publishing…' : 'Publish as @' + currentUser.value?.username}</Button>
          <Button type="button" variant="secondary" onclick={() => (publishOpen = false)}>Cancel</Button>
        </div>
        <p class="mt-2 text-xs text-muted-foreground">Publishes a new cart linked to <strong>{cart?.title}</strong>, credited to @{cart?.owner ?? cart?.author}.</p>
        {#if needsVerify}
          <div class="mt-3 rounded-md border border-primary/40 bg-primary/10 p-3 text-sm" data-testid="verify-notice">
            <p class="font-semibold">One step left: confirm your email.</p>
            <p class="mt-1">We sent a link to <strong>{currentUser.value?.email}</strong>. Click it, then come back to this tab and press Publish. Your remix is saved in this browser.</p>
            <div class="mt-2 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
              <Button type="button" size="sm" variant="secondary" disabled={resend === 'sending' || resend === 'sent'} onclick={resendVerification}>{resend === 'sent' ? 'Sent' : 'Send it again'}</Button>
              {#if resend === 'failed'}<span class="text-destructive">Couldn't send. Try again in a minute.</span>{:else}<span>Check spam if it isn't there in a minute.</span>{/if}
            </div>
          </div>
        {/if}
        {#if publishError}<p class="mt-2 text-sm text-destructive" role="alert">{publishError}</p>{/if}
      </form>
    {/if}

    <div class="grid flex-1 gap-4 p-4 lg:grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)] md:p-6">
      <section class="flex flex-col gap-3" aria-label="Game">
        <div class="relative aspect-3/2 w-full overflow-hidden rounded-lg bg-black shadow-2xl shadow-black/60">
          <canvas bind:this={canvas} width="192" height="128" class="block size-full" style="image-rendering: pixelated;"></canvas>
          <div class="scanline-overlay crt-vignette pointer-events-none absolute inset-0 opacity-65"></div>
          <div bind:this={touchContainer} class="touch-overlay pointer-events-none absolute inset-0"></div>
        </div>
        <p class="text-xs text-muted-foreground">Click the game to play · arrows move · Z or Space = A button · X = B button</p>
        {#if constants.length}
          <div class="surface-panel rounded-lg p-3">
            <p class="mb-2 text-sm font-semibold">Change one thing</p>
            <div class="flex flex-wrap gap-2">
              {#each constants as c (c.line)}
                <div class="flex items-center gap-2 rounded-md border border-border bg-background px-2 py-1 font-mono text-xs">
                  <button type="button" class="text-primary hover:underline" onclick={() => selectConstant(c.line)} title="Show line {c.line}">{c.name}</button>
                  <input type="number" value={c.value} step={c.value.includes('.') ? 0.1 : 1} aria-label={c.name} class="w-16 rounded border border-border bg-transparent px-1 py-0.5" onchange={(e) => tweak(c.line, e.currentTarget.value)} />
                  {#if c.suggestion && c.suggestion !== c.value}<button type="button" class="rounded bg-primary/15 px-1.5 py-0.5 text-primary hover:bg-primary/25" onclick={() => tweak(c.line, c.suggestion!)} aria-label="Try {c.name} = {c.suggestion}">try {c.suggestion}</button>{/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </section>

      <section class="flex min-h-[420px] flex-col gap-2" aria-label="Code">
        <div class="flex items-center gap-2">
          <span class="text-sm font-semibold">Lua</span>
          {#if restored}<span class="text-xs text-muted-foreground">Restored your saved edit</span>{/if}
          <span class="ml-auto text-xs text-muted-foreground">Reruns as you type · Ctrl+Enter</span>
          <Button size="sm" onclick={() => run()} class={changed && ranSource !== source ? 'ember-glow' : ''}><PlayIcon class="size-4" fill="currentColor" />Run</Button>
        </div>
        {#if runError}
          <div class="rounded-md border border-destructive/50 bg-destructive/10 p-3 text-sm" role="alert">
            <p class="font-semibold text-destructive">{runError.line ? `Line ${runError.line}: ` : ''}{runError.detail}</p>
            {#if runError.hint}<p class="mt-1 text-foreground">{runError.hint}</p>{/if}
            <p class="mt-1 text-xs text-muted-foreground">{runError.runtime ? 'The game stopped here. Fix the line and it reruns.' : 'Your last working version is still running. Your edit is kept. Fix it and it reruns.'}</p>
          </div>
        {:else if changed && runOk}
          <p class="text-sm text-primary" role="status">It runs. That's your change in the game.</p>
        {/if}
        <div class="relative flex min-h-0 flex-1 overflow-hidden rounded-md border border-border bg-black/50 font-mono text-[13px]">
          <div class="pointer-events-none select-none overflow-hidden border-r border-border px-2 text-right text-muted-foreground" style="line-height: {LINE_HEIGHT}px; padding-top: 8px;" aria-hidden="true">
            <div style="transform: translateY({-scrollTop}px)">
              {#each { length: lineCount } as _, i}<div class:text-destructive={runError?.line === i + 1} class:font-bold={runError?.line === i + 1}>{i + 1}</div>{/each}
            </div>
          </div>
          <div class="relative min-w-0 flex-1">
            {#if runError?.line}<div class="pointer-events-none absolute inset-x-0 bg-destructive/25" style="top: {8 + (runError.line - 1) * LINE_HEIGHT - scrollTop}px; height: {LINE_HEIGHT}px;" data-testid="error-line"></div>{/if}
            <textarea
              bind:this={editor}
              bind:value={source}
              onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
              onkeydown={onEditorKey}
              spellcheck="false"
              autocapitalize="off"
              wrap="off"
              aria-label="Lua source"
              class="absolute inset-0 size-full resize-none bg-transparent px-3 text-foreground outline-none"
              style="line-height: {LINE_HEIGHT}px; padding-top: 8px; padding-bottom: 8px; white-space: pre; tab-size: 2;"
            ></textarea>
          </div>
        </div>
      </section>
    </div>
  {/if}
</div>
