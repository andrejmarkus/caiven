<script lang="ts">
  import { api, type CartDetail, type CollectionInfo, type UserProfile } from '../api';
  import RatingStars from '../components/RatingStars.svelte';
  import ScreenshotImg from '../components/ScreenshotImg.svelte';
  import CommentList from '../components/CommentList.svelte';
  import CartCard from '../components/CartCard.svelte';
  import { currentUser } from '../stores.svelte';
  import { navigate, link } from '../router.svelte';
  import { Button, buttonVariants } from '$lib/components/ui/button';
  import PlayIcon from '@lucide/svelte/icons/play';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';

  let { id }: { id: string } = $props();
  let cart = $state<CartDetail | null>(null);
  let creator = $state<UserProfile | null>(null);
  let more = $state<CartDetail['versions']>([]);
  let creatorCarts = $state<CartDetail[]>([]);
  let collections = $state<CollectionInfo[]>([]);
  let loading = $state(true);
  let error = $state('');
  let tab = $state<'overview' | 'comments' | 'versions'>('overview');
  let adding = $state(false);
  const isOwner = $derived(!!cart && !!currentUser.value && (cart.owner === currentUser.value.username || currentUser.value.is_admin));

  async function load() {
    loading = true; error = '';
    try {
      cart = await api.getCart(id);
      more = cart.versions;
      if (cart.owner) {
        creator = await api.userProfile(cart.owner, 0, 20);
        creatorCarts = creator.carts.filter((c) => c.id !== id) as CartDetail[];
      }
    } catch (e) { error = e instanceof Error ? e.message : String(e); }
    finally { loading = false; }
  }
  $effect(() => { id; load(); });
  async function rate(score: number) { if (!cart) return; await api.rateCart(cart.id, score); await load(); }
  async function follow() {
    if (!cart?.owner) return;
    if (!currentUser.value) { navigate(`/login?next=/cart/${id}`); return; }
    creator?.followed_by_me ? await api.unfollowUser(cart.owner) : await api.followUser(cart.owner);
    creator = await api.userProfile(cart.owner, 0, 20);
  }
  async function openCollections() {
    if (!currentUser.value) { navigate(`/login?next=/cart/${id}`); return; }
    adding = !adding;
    if (adding) collections = await api.listCollections({ owner: currentUser.value.username, per_page: 100 });
  }
  async function add(slug: string) { await api.addCollectionCart(slug, id); adding = false; }
</script>

<div>
  {#if error}<div class="container-page py-6"><div class="rounded-lg border border-destructive/50 p-4 text-destructive">{error}</div></div>{/if}
  {#if loading}<div class="h-[520px] animate-pulse bg-card"></div>
  {:else if cart}
    <header class="border-b border-border bg-card">
      <div class="container-page flex flex-wrap gap-8 py-8 md:gap-10">
        <div class="min-w-0 flex-1 basis-[460px]">
          <div class="mb-4 flex gap-2 text-sm text-muted-foreground"><a href="/browse" use:link>Browse</a><span>/</span><span>{cart.tags[0] ?? 'cart'}</span><span>/</span><span>{cart.title}</span></div>
          <h1 class="text-3xl font-bold md:text-4xl">{cart.title}</h1>
          <div class="mt-5 flex flex-wrap items-center gap-4">
            <a href="/author/{cart.owner ?? cart.author}" use:link class="flex items-center gap-2 text-foreground"><span class="flex size-8 items-center justify-center rounded-full bg-secondary font-display font-semibold">{(cart.owner ?? cart.author)[0]?.toUpperCase()}</span><strong class="text-sm">{cart.owner ?? cart.author}</strong></a>
            {#if cart.owner && currentUser.value?.username !== cart.owner}<Button size="sm" variant="secondary" onclick={follow}>{creator?.followed_by_me ? 'Following' : 'Follow'}</Button>{/if}
            <span class="h-6 w-px bg-border"></span><RatingStars value={cart.rating_avg} /><span class="text-sm text-muted-foreground">{cart.rating_avg.toFixed(1)} · {cart.rating_count} ratings</span>
            <span class="h-6 w-px bg-border"></span><span class="font-mono text-sm text-muted-foreground">{cart.plays.toLocaleString()} plays</span><span class="font-mono text-sm text-muted-foreground">v{cart.latest_version}</span>
          </div>
          <p class="mt-5 max-w-[70ch] text-base leading-relaxed text-muted-foreground">{cart.description}</p>
          <div class="mt-5 flex flex-wrap gap-2">{#each cart.tags as tag}<a href="/browse?tag={encodeURIComponent(tag)}" use:link class="rounded-full border border-border px-3 py-1 text-sm text-muted-foreground hover:border-primary hover:text-primary">{tag}</a>{/each}</div>
          <div class="mt-7 flex flex-wrap gap-2">
            <a href="/play/{cart.id}" use:link class={buttonVariants({ size: 'lg', class: 'ember-glow' })}><PlayIcon fill="currentColor" />Play now</a>
            <a href={api.cartUrl(cart.id)} class={buttonVariants({ variant: 'secondary', size: 'lg' })}><DownloadIcon />.cav</a>
            <Button variant="secondary" size="lg" onclick={openCollections}><FolderPlusIcon />Add to collection</Button>
            {#if isOwner}<a href="/upload?cart={cart.id}" use:link class={buttonVariants({ variant: 'secondary', size: 'lg' })}>New version</a>{/if}
          </div>
          {#if adding}<div class="surface-panel mt-3 max-w-md rounded-lg p-3">{#each collections.filter((c) => !c.carts.some((x) => x.id === id)) as collection}<button onclick={() => add(collection.slug)} class="flex w-full items-center justify-between rounded px-3 py-2 text-left text-sm hover:bg-secondary"><span>{collection.title}</span><span>+</span></button>{:else}<p class="p-2 text-sm text-muted-foreground">No available owned collections.</p>{/each}</div>{/if}
        </div>
        <div class="min-w-0 flex-1 basis-[340px] md:max-w-[430px]">
          <a href="/play/{cart.id}" use:link class="cart-notch relative block aspect-square overflow-hidden border border-border bg-black">
            <ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} alt={cart.title} />
            <div class="scanline-overlay crt-vignette pointer-events-none absolute inset-0 opacity-50"></div>
            <span class="ember-glow absolute right-3 bottom-3 flex size-14 items-center justify-center rounded-full bg-primary text-primary-foreground"><PlayIcon class="size-6" fill="currentColor" /></span>
          </a>
        </div>
      </div>
    </header>

    <div class="container-page py-2">
      <div class="max-w-[900px]">
        <div class="flex border-b border-border">{#each [{v:'overview',l:'Overview'},{v:'comments',l:'Comments'},{v:'versions',l:`Versions · ${cart.versions.length}`}] as item}<button onclick={() => (tab = item.v as typeof tab)} class="border-b-2 px-4 py-3 text-sm font-semibold" class:border-primary={tab === item.v} class:border-transparent={tab !== item.v} class:text-primary={tab === item.v} class:text-muted-foreground={tab !== item.v}>{item.l}</button>{/each}</div>
        {#if tab === 'overview'}
          <div class="space-y-8 py-7">
            <section><h2 class="mb-3 font-semibold">Controls</h2><div class="flex flex-wrap gap-2">{#each [['← →','move'],['↑ ↓','aim'],['J / Z','A'],['K / X','B'],['Gamepad','supported'],['Touch','mobile']] as control}<span class="rounded-md border border-border bg-card px-3 py-2 text-sm text-muted-foreground"><kbd class="mr-2 font-mono text-xs text-foreground">{control[0]}</kbd>{control[1]}</span>{/each}</div></section>
            {#if currentUser.value}<section><h2 class="mb-3 font-semibold">Rate this cart</h2><div class="surface-panel flex items-center gap-4 rounded-lg p-4"><RatingStars value={cart.own_rating ?? 0} interactive onrate={rate} /><span class="text-sm text-muted-foreground">Ratings shape Top rated.</span></div></section>{/if}
            {#if creatorCarts.length}<section><h2 class="mb-4 font-semibold">More from {cart.owner}</h2><div class="cart-grid">{#each creatorCarts.slice(0,4) as item}<CartCard cart={item} compact />{/each}</div></section>{/if}
          </div>
        {:else if tab === 'comments'}
          <div class="py-7"><CommentList cartId={cart.id} ownerUsername={cart.owner} /></div>
        {:else}
          <div class="py-7">{#each [...more].reverse() as version}<div class="flex flex-wrap items-center gap-4 border-b border-border py-4"><strong>v{version.version}</strong><span class="text-sm text-muted-foreground">{new Date(version.created_at).toLocaleDateString()}</span><span class="font-mono text-xs text-muted-foreground">{(version.cart_size / 1024).toFixed(1)} KB</span><span class="text-sm text-muted-foreground">{version.changelog}</span><a class="ml-auto text-sm" href={api.cartUrl(cart.id, version.version)}>download</a></div>{/each}</div>
        {/if}
      </div>
    </div>
  {/if}
</div>
