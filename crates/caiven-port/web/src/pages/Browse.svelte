<script lang="ts">
  import { api, type Cart, type Sort } from '../api';
  import CartCard from '../components/CartCard.svelte';
  import Pagination from '../components/Pagination.svelte';
  import { route, navigate } from '../router.svelte';
  import * as InputGroup from '$lib/components/ui/input-group';
  import * as Select from '$lib/components/ui/select';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import * as Alert from '$lib/components/ui/alert';
  import * as Empty from '$lib/components/ui/empty';
  import SearchIcon from '@lucide/svelte/icons/search';
  import XIcon from '@lucide/svelte/icons/x';
  import DiscIcon from '@lucide/svelte/icons/disc';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';

  const PER_PAGE = 24;

  const SORTS: Record<Sort, string> = { new: 'Newest', popular: 'Most downloaded', top: 'Top rated' };

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

  function onSortChange(v: string) {
    sort = v as Sort;
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

<div class="container-page py-10">
  <h1 class="mb-6 text-2xl font-semibold">Browse carts</h1>

  <form onsubmit={onSubmit} class="mb-8 flex flex-wrap items-center gap-3">
    <InputGroup.Root class="w-full max-w-xs">
      <InputGroup.Addon>
        <SearchIcon />
      </InputGroup.Addon>
      <InputGroup.Input placeholder="Search carts…" bind:value={q} />
    </InputGroup.Root>

    <Select.Root type="single" value={sort} onValueChange={onSortChange}>
      <Select.Trigger class="w-48">
        {SORTS[sort]}
      </Select.Trigger>
      <Select.Content>
        <Select.Group>
          {#each Object.entries(SORTS) as [value, label] (value)}
            <Select.Item {value} {label} />
          {/each}
        </Select.Group>
      </Select.Content>
    </Select.Root>

    <Button type="submit" variant="secondary">Search</Button>

    {#if tag}
      <Badge variant="secondary" class="h-9 gap-1.5 px-3">
        tag: {tag}
        <button type="button" onclick={clearTag} aria-label="Clear tag filter" class="ml-0.5">
          <XIcon class="size-3" />
        </button>
      </Badge>
    {/if}
  </form>

  {#if error}
    <Alert.Root variant="destructive" class="mb-6">
      <CircleAlertIcon />
      <Alert.Description>{error}</Alert.Description>
    </Alert.Root>
  {/if}

  {#if loading}
    <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6">
      {#each Array(12) as _}
        <Skeleton class="aspect-square w-full rounded-lg" />
      {/each}
    </div>
  {:else if carts.length === 0}
    <Empty.Root>
      <Empty.Header>
        <Empty.Media variant="icon"><DiscIcon /></Empty.Media>
        <Empty.Title>No carts found</Empty.Title>
        <Empty.Description>Try a different search term, or clear the tag filter.</Empty.Description>
      </Empty.Header>
    </Empty.Root>
  {:else}
    <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6">
      {#each carts as cart (cart.id)}
        <CartCard {cart} />
      {/each}
    </div>
    <Pagination {page} perPage={PER_PAGE} {total} onchange={onPageChange} />
  {/if}
</div>
