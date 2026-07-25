<script lang="ts">
  import { api, type Cart, type Sort, type TagCount } from '../api';
  import CartCard from '../components/CartCard.svelte';
  import Pagination from '../components/Pagination.svelte';
  import { route, navigate } from '../router.svelte';
  import SearchIcon from '@lucide/svelte/icons/search';
  import XIcon from '@lucide/svelte/icons/x';
  import DiscIcon from '@lucide/svelte/icons/disc';

  const PER_PAGE = 24;
  const sorts: Array<{ value: Sort; label: string }> = [
    { value: 'trending', label: 'Trending' },
    { value: 'top', label: 'Top rated' },
    { value: 'new', label: 'Newest' },
  ];
  let carts = $state<Cart[]>([]);
  let tags = $state<TagCount[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let error = $state('');
  let q = $state('');
  let tag = $state('');
  let sort = $state<Sort>('trending');
  let page = $state(0);

  function readUrl() {
    q = route.search.get('q') ?? '';
    tag = route.search.get('tag') ?? '';
    sort = (route.search.get('sort') as Sort) ?? 'trending';
    page = Number(route.search.get('page') ?? 0) || 0;
  }
  function push() {
    const p = new URLSearchParams();
    if (q.trim()) p.set('q', q.trim());
    if (tag) p.set('tag', tag);
    if (sort !== 'trending') p.set('sort', sort);
    if (page) p.set('page', String(page));
    navigate(`/browse${p.size ? `?${p}` : ''}`);
  }
  function applySort(value: Sort) { sort = value; page = 0; push(); }
  function applyTag(value: string) { tag = value; page = 0; push(); }
  function submit(e: Event) { e.preventDefault(); page = 0; push(); }
  function clearAll() { q = ''; tag = ''; page = 0; push(); }

  $effect(() => {
    route.path;
    route.search.toString();
    readUrl();
    (async () => {
      loading = true;
      error = '';
      try {
        const [res, tagRes] = await Promise.all([
          api.listCarts({ page, per_page: PER_PAGE, q: q || undefined, tag: tag || undefined, sort }),
          api.listTags(),
        ]);
        carts = res.carts;
        total = res.total;
        tags = tagRes;
      } catch (e) { error = e instanceof Error ? e.message : String(e); }
      finally { loading = false; }
    })();
  });
</script>

<div class="container-page py-8 md:py-10">
  <div class="mb-6 flex flex-wrap items-end justify-between gap-5">
    <div>
      <h1 class="page-title">Browse carts</h1>
      <p class="mt-1 text-sm text-muted-foreground">{loading ? 'Searching the Port…' : `${total} ${total === 1 ? 'cart' : 'carts'}${tag ? ` tagged ${tag}` : ''}`}</p>
    </div>
    <div class="flex items-center gap-2">
      <span class="label-mono text-[10px] text-muted-foreground">Sort</span>
      <div class="flex rounded-md border border-border bg-card p-1">
        {#each sorts as item}
          <button onclick={() => applySort(item.value)} class="rounded px-3 py-1.5 text-sm" class:bg-primary={sort === item.value} class:text-primary-foreground={sort === item.value} class:text-muted-foreground={sort !== item.value}>{item.label}</button>
        {/each}
      </div>
    </div>
  </div>

  <form onsubmit={submit} class="mb-5 flex max-w-xl items-center gap-2 rounded-md border border-border bg-card px-3 focus-within:border-primary">
    <SearchIcon class="size-4 text-muted-foreground" />
    <input bind:value={q} placeholder="Search title, creator, or tag…" class="h-11 min-w-0 flex-1 border-0 bg-transparent p-0 text-sm outline-none ring-0" />
    <button class="rounded bg-secondary px-3 py-1.5 text-xs font-semibold">Search</button>
  </form>

  <div class="mb-7 flex flex-wrap items-center gap-2 border-b border-border pb-5">
    {#if tag}
      <button onclick={() => applyTag('')} class="flex items-center gap-2 rounded-full bg-accent px-3 py-1.5 text-sm text-accent-foreground">tag: {tag}<XIcon class="size-3" /></button>
    {/if}
    {#if q}
      <button onclick={() => { q = ''; push(); }} class="flex items-center gap-2 rounded-full border border-border px-3 py-1.5 text-sm text-muted-foreground">“{q}”<XIcon class="size-3" /></button>
    {/if}
    <div class="flex flex-wrap gap-2 md:ml-auto">
      {#each tags.slice(0, 8) as item}
        <button onclick={() => applyTag(item.tag)} class="rounded-full border border-border px-3 py-1 text-sm text-muted-foreground hover:border-primary hover:text-primary">{item.tag}</button>
      {/each}
    </div>
  </div>

  {#if error}<div class="mb-6 rounded-lg border border-destructive/50 bg-destructive/10 p-4 text-sm text-destructive">{error}</div>{/if}
  {#if loading}
    <div class="cart-grid">{#each Array(12) as _}<div class="aspect-[.72] animate-pulse rounded-lg bg-card"></div>{/each}</div>
  {:else if carts.length === 0}
    <div class="py-20 text-center">
      <DiscIcon class="mx-auto size-12 rounded-lg border border-border bg-card p-3 text-muted-foreground" />
      <h2 class="mt-4 text-lg font-semibold">Nothing matches that</h2>
      <p class="mx-auto mt-2 max-w-md text-sm text-muted-foreground">Drop tag filter or try shorter search. Every public cart is searchable.</p>
      <button onclick={clearAll} class="mt-5 rounded-md bg-secondary px-4 py-2 text-sm font-semibold">Clear filters</button>
    </div>
  {:else}
    <div class="cart-grid">{#each carts as cart (cart.id)}<CartCard {cart} />{/each}</div>
    <Pagination {page} perPage={PER_PAGE} {total} onchange={(p) => { page = p; push(); }} />
  {/if}
</div>
