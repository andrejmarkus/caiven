<script lang="ts">
  import { api, type Cart, type CollectionInfo } from '../api';
  import { currentUser } from '../stores.svelte';
  import CartCard from '../components/CartCard.svelte';
  import { Button } from '@caiven/ui/button';
  import { navigate } from '../router.svelte';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';

  let { slug }: { slug: string } = $props();
  let collection = $state<CollectionInfo | null>(null);
  let available = $state<Cart[]>([]);
  let error = $state('');
  let adding = $state(false);
  let editing = $state(false);
  let title = $state('');
  let description = $state('');
  const canEdit = $derived(!!collection && !!currentUser.value && (collection.owner === currentUser.value.username || currentUser.value.is_admin));

  async function load() {
    try { collection = await api.getCollection(slug); }
    catch (e) { error = e instanceof Error ? e.message : String(e); }
  }
  $effect(() => { slug; load(); });
  async function toggleFollow() {
    if (!collection) return;
    if (!currentUser.value) { navigate(`/login?next=/collections/${slug}`); return; }
    collection.followed_by_me ? await api.unfollowCollection(slug) : await api.followCollection(slug);
    await load();
  }
  async function openPicker() {
    adding = !adding;
    if (adding && !available.length) available = (await api.listCarts({ per_page: 100, sort: 'new' })).carts;
  }
  async function add(id: string) { collection = await api.addCollectionCart(slug, id); }
  async function remove(id: string) { collection = await api.removeCollectionCart(slug, id); }
  function openEditor() {
    if (!collection) return;
    title = collection.title;
    description = collection.description;
    editing = true;
  }
  async function save(e: Event) {
    e.preventDefault();
    collection = await api.updateCollection(slug, { title, description });
    editing = false;
  }
  async function move(index: number, direction: -1 | 1) {
    if (!collection) return;
    const next = collection.carts.map((cart) => cart.id);
    const target = index + direction;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    collection = await api.reorderCollection(slug, next);
  }
  async function destroy() {
    await api.deleteCollection(slug);
    navigate('/collections');
  }
</script>

<div class="container-page py-8 md:py-10">
  <button onclick={() => history.back()} class="mb-5 flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"><ArrowLeftIcon class="size-4" />Collections</button>
  {#if error}<div class="rounded-lg border border-destructive/50 p-4 text-destructive">{error}</div>{/if}
  {#if collection}
    <header class="surface-panel rounded-xl p-6 md:p-8">
      <div class="label-mono text-[10px] text-accent-foreground">{collection.kind === 'editorial' ? 'Editor’s pick' : `Curated by ${collection.owner}`}</div>
      <div class="mt-2 flex flex-wrap items-start justify-between gap-4">
        <div><h1 class="text-3xl font-bold">{collection.title}</h1><p class="mt-2 max-w-2xl text-muted-foreground">{collection.description}</p><p class="mt-4 font-mono text-xs text-muted-foreground">{collection.cart_count} carts · {collection.follower_count} followers</p></div>
        <div class="flex gap-2">
          <Button variant={collection.followed_by_me ? 'secondary' : 'default'} onclick={toggleFollow}>{collection.followed_by_me ? 'Following' : 'Follow shelf'}</Button>
          {#if canEdit}<Button variant="secondary" onclick={openEditor}><PencilIcon />Edit</Button><Button variant="secondary" onclick={openPicker}><PlusIcon />Add cart</Button><Button variant="destructive" size="icon" onclick={destroy} aria-label="Delete collection"><TrashIcon /></Button>{/if}
        </div>
      </div>
    </header>
    {#if adding}
      <div class="surface-panel mt-5 max-h-80 overflow-y-auto rounded-lg p-4">
        <h2 class="mb-3 text-sm font-semibold">Add any public cart</h2>
        <div class="grid gap-2 sm:grid-cols-2">
          {#each available.filter((c) => !collection?.carts.some((x) => x.id === c.id)) as cart}
            <button onclick={() => add(cart.id)} class="flex items-center justify-between rounded-md border border-border p-3 text-left hover:border-primary"><span><strong class="block text-sm">{cart.title}</strong><span class="text-xs text-muted-foreground">{cart.owner ?? cart.author}</span></span><PlusIcon class="size-4" /></button>
          {/each}
        </div>
      </div>
    {/if}
    <div class="cart-grid mt-7">
      {#each collection.carts as cart, index (cart.id)}
        <div class="relative"><CartCard {cart} />{#if canEdit}<div class="absolute top-2 left-2 z-10 flex overflow-hidden rounded-full bg-black/80 text-white"><button onclick={() => move(index, -1)} disabled={index === 0} aria-label="Move {cart.title} up" class="flex size-8 items-center justify-center disabled:opacity-30"><ArrowUpIcon class="size-3.5" /></button><button onclick={() => move(index, 1)} disabled={index === collection.carts.length - 1} aria-label="Move {cart.title} down" class="flex size-8 items-center justify-center disabled:opacity-30"><ArrowDownIcon class="size-3.5" /></button><button onclick={() => remove(cart.id)} aria-label="Remove {cart.title}" class="flex size-8 items-center justify-center hover:bg-destructive"><TrashIcon class="size-3.5" /></button></div>{/if}</div>
      {:else}
        <p class="col-span-full py-16 text-center text-muted-foreground">Empty shelf. Add first cart.</p>
      {/each}
    </div>
  {/if}
</div>
{#if editing}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4">
    <form onsubmit={save} class="surface-panel w-full max-w-lg space-y-4 rounded-xl p-6">
      <h2 class="text-xl font-semibold">Edit collection</h2>
      <input bind:value={title} maxlength={80} required class="h-10 w-full rounded-md border border-border bg-background px-3" />
      <textarea bind:value={description} maxlength={500} rows={4} class="w-full rounded-md border border-border bg-background p-3"></textarea>
      <div class="flex gap-2"><Button type="submit">Save</Button><Button type="button" variant="ghost" onclick={() => (editing = false)}>Cancel</Button></div>
    </form>
  </div>
{/if}
