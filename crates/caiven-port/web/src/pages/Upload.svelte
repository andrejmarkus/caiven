<script lang="ts">
  import { api, type JamInfo } from '../api';
  import { currentUser } from '../stores.svelte';
  import { route, navigate } from '../router.svelte';
  import { Button } from '@caiven/ui/button';
  import UploadIcon from '@lucide/svelte/icons/upload-cloud';

  const cartId = $derived(route.search.get('cart') ?? '');
  let title = $state('');
  let description = $state('');
  let tags = $state('');
  let changelog = $state('');
  let cartFile = $state<File | null>(null);
  let dragOver = $state(false);
  let error = $state('');
  let busy = $state(false);
  let jams = $state<JamInfo[]>([]);
  let jamSlug = $state('');

  $effect(() => { api.listJams().then((rows) => (jams = rows.filter((j) => j.status === 'open'))).catch(() => {}); });
  function pick(file?: File) { if (file) cartFile = file; }
  async function submit(e: Event) {
    e.preventDefault();
    if (!cartFile) { error = 'Select a .cav file'; return; }
    busy = true; error = '';
    try {
      if (cartId) {
        await api.createVersion(cartId, cartFile, changelog);
        navigate(`/cart/${cartId}`);
      } else {
        const cart = await api.createCart(cartFile, { title, description, tags: tags.split(',').map((x) => x.trim()).filter(Boolean) });
        if (jamSlug) await api.enterJam(jamSlug, cart.id);
        navigate(`/cart/${cart.id}`);
      }
    } catch (e) { error = e instanceof Error ? e.message : 'Upload failed'; }
    finally { busy = false; }
  }
</script>

<div class="container-page max-w-[820px] py-9 md:py-12">
  <h1 class="page-title">{cartId ? 'Publish new version' : 'Publish a cart'}</h1>
  <p class="mt-1 text-sm text-muted-foreground">Drop public <code class="text-foreground">.cav</code> built in Studio. Owner account becomes creator identity.</p>
  <div class="mt-7 flex">
    {#each ['Cart file', 'Details', 'Publish'] as label, i}
      <div class="flex flex-1 items-center gap-2"><span class="flex size-7 items-center justify-center rounded-full font-mono text-xs font-semibold" class:bg-primary={i === 0} class:text-primary-foreground={i === 0} class:bg-secondary={i > 0} class:text-muted-foreground={i > 0}>{i + 1}</span><span class="text-sm" class:font-semibold={i === 0} class:text-muted-foreground={i > 0}>{label}</span>{#if i < 2}<span class="h-px flex-1 bg-border"></span>{/if}</div>
    {/each}
  </div>
  {#if error}<div class="mt-5 rounded-lg border border-destructive/50 bg-destructive/10 p-4 text-sm text-destructive">{error}</div>{/if}
  <form onsubmit={submit} class="mt-7 space-y-5">
    <div
      role="button" tabindex="0"
      class="surface-panel flex flex-col items-center rounded-xl border-dashed p-10 text-center transition-colors"
      class:border-primary={dragOver}
      ondragover={(e) => { e.preventDefault(); dragOver = true; }}
      ondragleave={() => (dragOver = false)}
      ondrop={(e) => { e.preventDefault(); dragOver = false; pick(e.dataTransfer?.files?.[0]); }}
    >
      <span class="flex size-13 items-center justify-center rounded-lg border border-border bg-background text-primary"><UploadIcon class="size-6" /></span>
      <h2 class="mt-4 font-semibold">{cartFile ? cartFile.name : 'Drop your .cav here'}</h2>
      <p class="mt-2 text-sm text-muted-foreground">{cartFile ? `${(cartFile.size / 1024).toFixed(1)} KB` : 'Or publish from terminal: caiven-studio publish game.cav'}</p>
      <label class="mt-4 cursor-pointer rounded-md bg-secondary px-4 py-2 text-sm font-semibold">Browse files<input type="file" accept=".cav" class="sr-only" onchange={(e) => pick(e.currentTarget.files?.[0])} /></label>
    </div>
    <div class="surface-panel space-y-5 rounded-xl p-6">
      {#if cartId}
        <label class="block text-sm font-semibold">Changelog<textarea bind:value={changelog} rows={4} placeholder="What changed in this version?" class="mt-2 w-full rounded-md border border-border bg-background p-3 font-normal"></textarea></label>
      {:else}
        <label class="block text-sm font-semibold">Title<input bind:value={title} maxlength={64} required placeholder="Read from cart header" class="mt-2 h-10 w-full rounded-md border border-border bg-background px-3 font-normal" /></label>
        <p class="text-sm font-semibold">Author<span class="mt-1 block text-sm font-normal">Publishing as <strong>@{currentUser.value?.username}</strong></span><span class="mt-1 block text-xs font-normal text-muted-foreground">Creator identity is your account and can't be changed here.</span></p>
        <label class="block text-sm font-semibold">Short description<textarea bind:value={description} maxlength={512} rows={3} class="mt-2 w-full rounded-md border border-border bg-background p-3 font-normal"></textarea><span class="mt-1 block text-xs font-normal text-muted-foreground">Say what player does, not what game is about.</span></label>
        <label class="block text-sm font-semibold">Tags<input bind:value={tags} placeholder="platformer, dark" class="mt-2 h-10 w-full rounded-md border border-border bg-background px-3 font-normal" /><span class="mt-1 block text-xs font-normal text-muted-foreground">Comma separated.</span></label>
        {#if jams.length}
          <label class="block border-t border-[var(--border-subtle)] pt-5 text-sm font-semibold">Enter open jam<select bind:value={jamSlug} class="mt-2 h-10 w-full rounded-md border border-border bg-background px-3 font-normal"><option value="">No jam</option>{#each jams as jam}<option value={jam.slug}>{jam.title} · closes {new Date(jam.submissions_close_at).toLocaleDateString()}</option>{/each}</select></label>
        {/if}
      {/if}
      <Button type="submit" disabled={busy || !cartFile}>{busy ? 'Publishing…' : cartId ? 'Publish version' : 'Publish cart'}</Button>
    </div>
  </form>
</div>
