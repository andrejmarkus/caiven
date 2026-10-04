<script lang="ts">
  import { api, type Cart, type CollectionInfo, type JamInfo, type TagCount } from '../api';
  import CartCard from '../components/CartCard.svelte';
  import AttractScreen from '../components/AttractScreen.svelte';
  import { currentUser } from '../stores.svelte';
  import { link } from '../router.svelte';
  import { buttonVariants } from '@caiven/ui/button';
  import { Skeleton } from '@caiven/ui/skeleton';
  import MaximizeIcon from '@lucide/svelte/icons/maximize-2';
  import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
  import CodeIcon from '@lucide/svelte/icons/code';

  let top = $state<Cart[]>([]);
  let trending = $state<Cart[]>([]);
  let recent = $state<Cart[]>([]);
  let newRemixes = $state<Cart[]>([]);
  let mostRemixed = $state<Cart[]>([]);
  let collections = $state<CollectionInfo[]>([]);
  let jams = $state<JamInfo[]>([]);
  let tags = $state<TagCount[]>([]);
  let loading = $state(true);
  let error = $state('');

  // Admin-curated, so a user tag can't claim this row; only opted-in carts show.
  const START_HERE = 'start-here';
  const editorial = $derived(collections.filter((c) => c.kind === 'editorial').sort((a, b) => (a.featured_rank ?? 999) - (b.featured_rank ?? 999)));
  const starters = $derived((editorial.find((c) => c.slug === START_HERE)?.carts ?? []).filter((c) => c.remixable).slice(0, 6));
  const shelf = $derived(editorial.find((c) => c.slug !== START_HERE) ?? null);
  const featured = $derived(shelf?.carts[0] ?? top[0] ?? trending[0] ?? null);
  const openJam = $derived(jams.find((j) => j.status === 'open') ?? null);

  $effect(() => {
    (async () => {
      loading = true;
      error = '';
      try {
        const [a, b, c, d, e, f, g, h] = await Promise.all([
          api.listCarts({ per_page: 6, sort: 'top' }),
          api.listCarts({ per_page: 6, sort: 'trending' }),
          api.listCarts({ per_page: 6, sort: 'new' }),
          api.listCollections({ kind: 'editorial', per_page: 10 }),
          api.listJams(),
          api.listTags(),
          api.listCarts({ per_page: 6, sort: 'remixes' }),
          api.listCarts({ per_page: 6, sort: 'remixed' }),
        ]);
        top = a.carts;
        trending = b.carts;
        recent = c.carts;
        collections = d;
        jams = e;
        tags = f;
        newRemixes = g.carts;
        mostRemixed = h.carts;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      } finally {
        loading = false;
      }
    })();
  });
</script>

<div class="container-page space-y-14 py-7 md:py-12">
  {#if error}
    <div class="rounded-lg border border-destructive/50 bg-destructive/10 p-4 text-sm text-destructive">{error}</div>
  {/if}

  {#if loading}
    <Skeleton class="h-[420px] w-full rounded-lg" />
  {:else if featured}
    <div class="space-y-6">
      {#if !currentUser.value}
        <p class="max-w-[60ch] text-lg text-muted-foreground md:text-xl">
          <strong class="text-foreground">Caiven is a fantasy console.</strong> Every game here is a small Lua program you can play, open, change and publish again.
        </p>
      {/if}
      <section class="-mx-6 grid items-center gap-8 bg-black/25 px-6 py-8 md:grid-cols-[minmax(0,2fr)_minmax(0,3fr)] md:gap-12 md:rounded-lg md:p-10 lg:mx-0">
        <div class="min-w-0 md:order-2">
          <AttractScreen cart={featured} />
        </div>
        <div class="min-w-0 md:order-1">
          <p class="text-sm text-muted-foreground">Cart of the week</p>
          <h1 class="mt-2 text-4xl leading-tight md:text-5xl">{featured.title}</h1>
          {#if featured.description}
            <p class="mt-4 max-w-[46ch] text-base leading-relaxed text-muted-foreground md:text-lg">{featured.description}</p>
          {/if}
          <p class="mt-5 text-sm text-muted-foreground">
            by <a href={featured.owner ? `/author/${featured.owner}` : undefined} use:link class="font-semibold text-foreground">{featured.owner ?? featured.author}</a>
            <span class="ml-3 font-mono">{featured.plays.toLocaleString()} plays</span>
            {#if featured.rating_count}<span class="ml-3 font-mono">{featured.rating_avg.toFixed(1)} rating</span>{/if}
          </p>
          <div class="mt-7 flex flex-wrap gap-3">
            {#if featured.remixable}
              <a href="/remix/{featured.id}" use:link class={buttonVariants({ size: 'lg', class: 'h-12 px-6' })}>
                <CodeIcon data-icon="inline-start" />Remix this game
              </a>
            {/if}
            <a href="/play/{featured.id}" use:link class={buttonVariants({ variant: featured.remixable ? 'secondary' : 'default', size: 'lg', class: 'h-12 px-6' })}>
              <MaximizeIcon data-icon="inline-start" />Full screen
            </a>
          </div>
        </div>
      </section>
    </div>
  {/if}

  {#if starters.length}
    <section>
      <div class="mb-5">
        <h2 class="text-xl">Remix a starter</h2>
        <p class="mt-1 text-sm text-muted-foreground">Open the code in your browser and change it. You only need an account to publish.</p>
      </div>
      <div class="cart-grid">
        {#each starters as cart (cart.id)}
          <div class="flex flex-col gap-2">
            <CartCard {cart} compact />
            <a href="/remix/{cart.id}" use:link aria-label="Remix {cart.title}" class={buttonVariants({ variant: 'secondary', size: 'sm' })}>
              <CodeIcon data-icon="inline-start" />Remix
            </a>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if shelf}
    <section>
      <div class="mb-5 flex items-end justify-between gap-4">
        <div>
          <h2 class="text-xl">{shelf.title}</h2>
          {#if shelf.description}<p class="mt-1 text-sm text-muted-foreground">{shelf.description}</p>{/if}
        </div>
        <a href="/collections/{shelf.slug}" use:link class="flex shrink-0 items-center gap-1 text-sm text-muted-foreground hover:text-primary">
          See all <ArrowRightIcon class="size-4" />
        </a>
      </div>
      <div class="cart-grid">
        {#each shelf.carts.slice(0, 6) as cart (cart.id)}<CartCard {cart} compact />{/each}
      </div>
    </section>
  {/if}

  {#if openJam}
    <section class="flex flex-wrap items-center gap-x-8 gap-y-4 border-y border-border py-6">
      <div class="min-w-0 flex-1 basis-[420px]">
        <h2 class="text-2xl">{openJam.title}</h2>
        <p class="mt-1 max-w-2xl text-muted-foreground">{openJam.description}</p>
        <p class="mt-2 font-mono text-sm text-muted-foreground">Jam open: {openJam.entry_count} {openJam.entry_count === 1 ? 'entry' : 'entries'} from {openJam.creator_count} {openJam.creator_count === 1 ? 'creator' : 'creators'}</p>
      </div>
      <a href="/jams/{openJam.slug}" use:link class={buttonVariants({ size: 'lg' })}>Enter jam</a>
    </section>
  {/if}

  {#each [
    { title: 'Trending this week', carts: trending, href: '/browse?sort=trending' },
    { title: 'New remixes', carts: newRemixes, href: '/browse?sort=remixes' },
    { title: 'Most remixed', carts: mostRemixed, href: '/browse?sort=remixed' },
    { title: 'New carts', carts: recent, href: '/browse?sort=new' },
  ] as section}
    {#if section.carts.length}
      <section>
        <div class="mb-5 flex items-end justify-between">
          <h2 class="text-xl">{section.title}</h2>
          <a href={section.href} use:link class="flex items-center gap-1 text-sm text-muted-foreground hover:text-primary">See all <ArrowRightIcon class="size-4" /></a>
        </div>
        <div class="cart-grid">{#each section.carts as cart (cart.id)}<CartCard {cart} compact />{/each}</div>
      </section>
    {/if}
  {/each}

  {#if tags.length}
    <section>
      <h2 class="text-xl">Tags</h2>
      <div class="mt-4 flex flex-wrap gap-2">
        {#each tags.slice(0, 16) as tag}
          <a href="/browse?tag={encodeURIComponent(tag.tag)}" use:link class="rounded-md border border-border px-3 py-1.5 text-sm text-muted-foreground hover:border-primary hover:text-primary">{tag.tag} <span class="font-mono text-xs text-foreground">{tag.count}</span></a>
        {/each}
      </div>
    </section>
  {/if}
</div>
