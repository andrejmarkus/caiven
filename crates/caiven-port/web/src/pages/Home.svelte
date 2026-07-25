<script lang="ts">
  import { api, type Cart, type CollectionInfo, type JamInfo, type TagCount } from '../api';
  import CartCard from '../components/CartCard.svelte';
  import ScreenshotImg from '../components/ScreenshotImg.svelte';
  import { link } from '../router.svelte';
  import { buttonVariants } from '$lib/components/ui/button';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import PlayIcon from '@lucide/svelte/icons/play';
  import StarIcon from '@lucide/svelte/icons/star';
  import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
  import TrophyIcon from '@lucide/svelte/icons/trophy';

  let top = $state<Cart[]>([]);
  let trending = $state<Cart[]>([]);
  let recent = $state<Cart[]>([]);
  let collections = $state<CollectionInfo[]>([]);
  let jams = $state<JamInfo[]>([]);
  let tags = $state<TagCount[]>([]);
  let loading = $state(true);
  let error = $state('');

  const editorial = $derived(collections.filter((c) => c.kind === 'editorial').sort((a, b) => (a.featured_rank ?? 999) - (b.featured_rank ?? 999)));
  const shelf = $derived(editorial[0] ?? null);
  const featured = $derived(shelf?.carts[0] ?? top[0] ?? trending[0] ?? null);
  const openJam = $derived(jams.find((j) => j.status === 'open') ?? null);

  $effect(() => {
    (async () => {
      loading = true;
      error = '';
      try {
        const [a, b, c, d, e, f] = await Promise.all([
          api.listCarts({ per_page: 6, sort: 'top' }),
          api.listCarts({ per_page: 6, sort: 'trending' }),
          api.listCarts({ per_page: 6, sort: 'new' }),
          api.listCollections({ kind: 'editorial', per_page: 10 }),
          api.listJams(),
          api.listTags(),
        ]);
        top = a.carts;
        trending = b.carts;
        recent = c.carts;
        collections = d;
        jams = e;
        tags = f;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      } finally {
        loading = false;
      }
    })();
  });
</script>

<div class="container-page space-y-14 py-7 md:py-10">
  {#if error}
    <div class="rounded-lg border border-destructive/50 bg-destructive/10 p-4 text-sm text-destructive">{error}</div>
  {/if}

  {#if loading}
    <Skeleton class="h-[460px] w-full rounded-xl" />
  {:else if featured}
    <section class="surface-panel relative overflow-hidden rounded-xl">
      <div class="pointer-events-none absolute -top-40 -left-28 size-[520px] bg-[radial-gradient(ellipse_at_center,rgba(254,176,93,.16),transparent_68%)]"></div>
      <div class="relative flex flex-wrap items-center gap-9 p-6 md:p-11">
        <div class="min-w-0 flex-1 basis-[420px]">
          <div class="label-mono inline-flex items-center gap-1.5 rounded-full bg-accent px-3 py-1.5 text-[10px] font-semibold text-accent-foreground">
            <StarIcon class="size-3 fill-current" />
            Cart of the week
          </div>
          <h1 class="mt-5 text-4xl leading-tight font-bold tracking-tight md:text-5xl">{featured.title}</h1>
          <p class="mt-4 max-w-[52ch] text-base leading-relaxed text-muted-foreground md:text-lg">
            {featured.description || 'A tiny world built for the Caiven fantasy console.'}
          </p>
          <div class="mt-6 flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
            <a href="/author/{featured.owner ?? featured.author}" use:link class="font-semibold text-foreground">
              {featured.owner ?? featured.author}
            </a>
            <span class="text-border">|</span>
            <span class="text-primary">{featured.rating_count ? `${featured.rating_avg.toFixed(1)} ★` : 'New'}</span>
            <span class="text-border">|</span>
            <span class="font-mono">{featured.plays.toLocaleString()} plays</span>
          </div>
          <div class="mt-7 flex flex-wrap gap-3">
            <a href="/play/{featured.id}" use:link class={buttonVariants({ size: 'lg', class: 'ember-glow h-12' })}>
              <PlayIcon data-icon="inline-start" fill="currentColor" />
              Play in browser
            </a>
            <a href="/cart/{featured.id}" use:link class={buttonVariants({ variant: 'secondary', size: 'lg', class: 'h-12' })}>Cart details</a>
          </div>
          <div class="mt-5 flex flex-wrap gap-2">
            {#each featured.tags as tag}
              <a href="/browse?tag={encodeURIComponent(tag)}" use:link class="rounded-full border border-border px-3 py-1 text-sm text-muted-foreground hover:border-primary hover:text-primary">{tag}</a>
            {/each}
          </div>
        </div>
        <a href="/cart/{featured.id}" use:link class="cart-notch relative mx-auto aspect-square w-full max-w-[420px] flex-1 basis-[340px] overflow-hidden border border-border bg-secondary">
          <ScreenshotImg id={featured.id} hasScreenshot={featured.has_screenshot} alt={featured.title} />
          <div class="scanline-overlay pointer-events-none absolute inset-0 opacity-30"></div>
          <span class="label-mono absolute top-3 left-3 rounded bg-black/60 px-2 py-1 text-[10px] text-white/60">128 × 128 · 16 col</span>
        </a>
      </div>
    </section>
  {/if}

  {#if shelf}
    <section>
      <div class="mb-5 flex items-end justify-between gap-4">
        <div>
          <div class="label-mono mb-1.5 text-[10px] text-accent-foreground">Editor’s pick</div>
          <h2 class="text-xl font-semibold">{shelf.title}</h2>
          <p class="mt-1 text-sm text-muted-foreground">{shelf.description}</p>
        </div>
        <a href="/collections/{shelf.slug}" use:link class="flex items-center gap-1 text-sm text-muted-foreground hover:text-primary">
          Open shelf <ArrowRightIcon class="size-4" />
        </a>
      </div>
      <div class="cart-grid">
        {#each shelf.carts.slice(0, 6) as cart (cart.id)}<CartCard {cart} compact />{/each}
      </div>
    </section>
  {/if}

  {#if openJam}
    <section class="surface-panel relative overflow-hidden rounded-xl p-6 md:p-8">
      <div class="pointer-events-none absolute -top-28 -right-20 size-96 bg-[radial-gradient(ellipse_at_center,rgba(254,176,93,.14),transparent_70%)]"></div>
      <div class="relative flex flex-wrap items-center gap-7">
        <div class="min-w-0 flex-1 basis-[460px]">
          <div class="label-mono flex items-center gap-2 text-[10px] text-primary"><TrophyIcon class="size-4" />Submissions open</div>
          <h2 class="mt-2 text-2xl font-bold">{openJam.title}</h2>
          <p class="mt-2 max-w-2xl text-muted-foreground">{openJam.description}</p>
        </div>
        <div class="flex items-center gap-5">
          <div><strong class="block font-mono text-xl">{openJam.entry_count}</strong><span class="label-mono text-[9px] text-muted-foreground">entries</span></div>
          <div><strong class="block font-mono text-xl">{openJam.creator_count}</strong><span class="label-mono text-[9px] text-muted-foreground">creators</span></div>
          <a href="/jams/{openJam.slug}" use:link class={buttonVariants({ size: 'lg' })}>Enter jam</a>
        </div>
      </div>
    </section>
  {/if}

  {#each [
    { title: 'Trending this week', sub: 'Most played in the last seven days', carts: trending, href: '/browse?sort=trending' },
    { title: 'Fresh off Studio', sub: 'New and recently updated carts', carts: recent, href: '/browse?sort=new' },
  ] as section}
    {#if section.carts.length}
      <section>
        <div class="mb-5 flex items-end justify-between">
          <div><h2 class="text-xl font-semibold">{section.title}</h2><p class="mt-1 text-sm text-muted-foreground">{section.sub}</p></div>
          <a href={section.href} use:link class="flex items-center gap-1 text-sm text-muted-foreground hover:text-primary">See all <ArrowRightIcon class="size-4" /></a>
        </div>
        <div class="cart-grid">{#each section.carts as cart (cart.id)}<CartCard {cart} compact />{/each}</div>
      </section>
    {/if}
  {/each}

  {#if tags.length}
    <section>
      <h2 class="text-xl font-semibold">Find your kind of tiny</h2>
      <div class="mt-4 flex flex-wrap gap-2">
        {#each tags.slice(0, 16) as tag}
          <a href="/browse?tag={encodeURIComponent(tag.tag)}" use:link class="rounded-full border border-border px-3 py-1.5 text-sm text-muted-foreground hover:border-primary hover:text-primary">{tag.tag} <span class="font-mono text-xs text-foreground">{tag.count}</span></a>
        {/each}
      </div>
    </section>
  {/if}
</div>
