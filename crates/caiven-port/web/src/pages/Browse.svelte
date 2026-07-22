<script lang="ts">
  import { api, type Cart, type Sort } from '../api';
  import CartCard from '../components/CartCard.svelte';
  import Pagination from '../components/Pagination.svelte';
  import { route, navigate } from '../router.svelte';

  const PER_PAGE = 24;

  let carts = $state<Cart[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let error = $state('');

  let q = $state('');
  let tag = $state('');
  let sort = $state<Sort>('new');
  let page = $state(0);

  function syncFromUrl() {
    const s = route.search;
    q = s.get('q') ?? '';
    tag = s.get('tag') ?? '';
    sort = (s.get('sort') as Sort) ?? 'new';
    page = Number(s.get('page') ?? 0) || 0;
  }

  function pushUrl() {
    const p = new URLSearchParams();
    if (q) p.set('q', q);
    if (tag) p.set('tag', tag);
    if (sort !== 'new') p.set('sort', sort);
    if (page) p.set('page', String(page));
    const s = p.toString();
    navigate(`/browse${s ? `?${s}` : ''}`);
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const res = await api.listCarts({ page, per_page: PER_PAGE, q: q || undefined, tag: tag || undefined, sort });
      carts = res.carts;
      total = res.total;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    route.path;
    route.search.toString();
    syncFromUrl();
    load();
  });

  function onSubmit(e: Event) {
    e.preventDefault();
    page = 0;
    pushUrl();
  }

  function onSortChange() {
    page = 0;
    pushUrl();
  }

  function onPageChange(p: number) {
    page = p;
    pushUrl();
  }

  function clearTag() {
    tag = '';
    page = 0;
    pushUrl();
  }
</script>

<div class="container">
  <h1>Browse</h1>
  <form class="panel row filters" onsubmit={onSubmit}>
    <input placeholder="Search…" bind:value={q} />
    <select bind:value={sort} onchange={onSortChange}>
      <option value="new">Newest</option>
      <option value="popular">Most downloaded</option>
      <option value="top">Top rated</option>
    </select>
    <button type="submit">Search</button>
    {#if tag}
      <span class="row">
        <span class="muted">tag: {tag}</span>
        <button type="button" class="secondary" onclick={clearTag}>×</button>
      </span>
    {/if}
  </form>

  {#if error}<p class="error">{error}</p>{/if}
  {#if loading}
    <p class="muted">loading…</p>
  {:else if carts.length === 0}
    <p class="muted">No carts found.</p>
  {:else}
    <div class="grid">
      {#each carts as cart (cart.id)}
        <CartCard {cart} />
      {/each}
    </div>
    <Pagination {page} perPage={PER_PAGE} {total} onchange={onPageChange} />
  {/if}
</div>

<style>
  .filters {
    margin: 1rem 0 1.5rem;
    flex-wrap: wrap;
  }
  .filters input {
    flex: 1;
    min-width: 160px;
  }
</style>
