<script lang="ts">
  import { api, type CollectionInfo } from '../api';
  import { currentUser } from '../stores.svelte';
  import { link, navigate } from '../router.svelte';
  import ScreenshotImg from '../components/ScreenshotImg.svelte';
  import { Button } from '@caiven/ui/button';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import UsersIcon from '@lucide/svelte/icons/users';

  let collections = $state<CollectionInfo[]>([]);
  let loading = $state(true);
  let error = $state('');
  let creating = $state(false);
  let title = $state('');
  let description = $state('');
  let editorial = $state(false);

  const picks = $derived(collections.filter((c) => c.kind === 'editorial'));
  const community = $derived(collections.filter((c) => c.kind === 'player' && c.owner !== currentUser.value?.username));
  const mine = $derived(collections.filter((c) => c.owner === currentUser.value?.username));

  async function load() {
    loading = true;
    try { collections = await api.listCollections({ per_page: 100 }); }
    catch (e) { error = e instanceof Error ? e.message : String(e); }
    finally { loading = false; }
  }
  $effect(() => { load(); });

  async function create(e: Event) {
    e.preventDefault();
    try {
      const collection = editorial && currentUser.value?.is_admin
        ? await api.createEditorialCollection({ title, description, featured_rank: picks.length + 1 })
        : await api.createCollection({ title, description });
      navigate(`/collections/${collection.slug}`);
    } catch (e) { error = e instanceof Error ? e.message : String(e); }
  }
</script>

<div class="container-page py-8 md:py-10">
  <div class="flex flex-wrap items-end justify-between gap-4">
    <div><h1 class="page-title">Collections</h1><p class="mt-1 text-sm text-muted-foreground">Editor picks and player-made shelves. Every cart stays public.</p></div>
    {#if currentUser.value}<Button onclick={() => (creating = !creating)}><PlusIcon />New collection</Button>{/if}
  </div>

  {#if creating}
    <form onsubmit={create} class="surface-panel mt-6 max-w-2xl space-y-4 rounded-lg p-5">
      <div><label for="collection-title" class="text-sm font-semibold">Title</label><input id="collection-title" bind:value={title} maxlength={80} required class="mt-2 h-10 w-full rounded-md border border-border bg-background px-3" /></div>
      <div><label for="collection-description" class="text-sm font-semibold">Description</label><textarea id="collection-description" bind:value={description} maxlength={500} rows={3} class="mt-2 w-full rounded-md border border-border bg-background p-3"></textarea></div>
      {#if currentUser.value?.is_admin}
        <label class="flex items-center gap-2 text-sm"><input type="checkbox" bind:checked={editorial} /> Publish as Editor’s Pick</label>
      {/if}
      <div class="flex gap-2"><Button type="submit">Create shelf</Button><Button type="button" variant="ghost" onclick={() => (creating = false)}>Cancel</Button></div>
    </form>
  {/if}

  {#if error}<div class="mt-6 rounded-lg border border-destructive/50 p-4 text-destructive">{error}</div>{/if}
  {#if loading}<div class="mt-7 space-y-5">{#each Array(3) as _}<div class="h-48 animate-pulse rounded-lg bg-card"></div>{/each}</div>{/if}

  {#each [
    { title: 'Editor’s Picks', sub: 'Shelves assembled by Caiven editors.', rows: picks },
    { title: 'Community Collections', sub: 'Public shelves from players.', rows: community },
    { title: 'Your Collections', sub: 'Shelves you curate.', rows: mine },
  ] as section}
    {#if section.rows.length}
      <section class="mt-9">
        <h2 class="text-xl font-semibold">{section.title}</h2><p class="mt-1 mb-4 text-sm text-muted-foreground">{section.sub}</p>
        <div class="space-y-5">
          {#each section.rows as collection}
            <a href="/collections/{collection.slug}" use:link class="surface-panel flex flex-wrap gap-6 rounded-lg p-5 text-foreground hover:border-primary hover:text-foreground">
              <div class="min-w-0 flex-1 basis-[260px]">
                <div class="label-mono text-[10px] text-accent-foreground">{collection.kind === 'editorial' ? 'Editor’s pick' : `By ${collection.owner}`}</div>
                <h3 class="mt-2 text-lg font-semibold">{collection.title}</h3>
                <p class="mt-2 text-sm text-muted-foreground">{collection.description}</p>
                <div class="mt-4 flex items-center gap-4 font-mono text-xs text-muted-foreground"><span>{collection.cart_count} carts</span><span class="flex items-center gap-1"><UsersIcon class="size-3.5" />{collection.follower_count}</span></div>
              </div>
              <div class="grid min-w-0 flex-1 basis-[420px] grid-cols-5 gap-2">
                {#each collection.carts.slice(0, 5) as cart}<div class="cart-notch aspect-square overflow-hidden bg-secondary"><ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} alt="" /></div>{/each}
              </div>
            </a>
          {/each}
        </div>
      </section>
    {/if}
  {/each}
</div>
