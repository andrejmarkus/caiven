<script lang="ts">
  import { api, type Cart } from '../api';
  import CartCard from '../components/CartCard.svelte';
  import { link, navigate } from '../router.svelte';
  import { buttonVariants } from '$lib/components/ui/button';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import * as Alert from '$lib/components/ui/alert';
  import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
  import PlayIcon from '@lucide/svelte/icons/play';
  import StarIcon from '@lucide/svelte/icons/star';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';

  let top = $state<Cart[]>([]);
  let trending = $state<Cart[]>([]);
  let recent = $state<Cart[]>([]);
  let loading = $state(true);
  let error = $state('');

  const featured = $derived(top[0] ?? trending[0] ?? null);

  $effect(() => {
    (async () => {
      loading = true;
      try {
        const [t, p, r] = await Promise.all([
          api.listCarts({ per_page: 6, sort: 'top' }),
          api.listCarts({ per_page: 6, sort: 'popular' }),
          api.listCarts({ per_page: 6, sort: 'new' }),
        ]);
        top = t.carts;
        trending = p.carts;
        recent = r.carts;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      } finally {
        loading = false;
      }
    })();
  });

  function play(e: MouseEvent) {
    if (!featured) return;
    e.preventDefault();
    navigate(`/play/${featured.id}`);
  }
</script>

{#if loading}
  <div class="relative h-[52vh] min-h-[380px] w-full overflow-hidden bg-secondary">
    <Skeleton class="size-full rounded-none" />
  </div>
{:else if featured}
  <section class="relative w-full overflow-hidden">
    {#if featured.has_screenshot}
      <img
        src={api.screenshotUrl(featured.id)}
        alt=""
        class="absolute inset-0 size-full scale-125 object-cover opacity-70 blur-3xl saturate-150"
      />
    {:else}
      <div class="absolute inset-0 bg-gradient-to-br from-secondary to-background"></div>
    {/if}
    <div class="absolute inset-0 bg-gradient-to-t from-background via-background/70 to-background/20"></div>
    <div class="scanline-overlay pointer-events-none absolute inset-0 opacity-[0.06]"></div>

    <div class="container-page relative flex flex-col items-center gap-10 py-14 sm:flex-row sm:py-20">
      <div class="order-2 min-w-0 flex-1 text-center sm:order-1 sm:text-left">
        <span class="label-mono mb-3 inline-flex w-fit items-center gap-1.5 rounded-full bg-primary/15 px-3 py-1 text-[11px] font-semibold text-accent-foreground">
          <StarIcon class="size-3.5 fill-primary text-primary" />
          Featured cart
        </span>
        <h1 class="text-4xl leading-tight font-semibold sm:text-5xl">{featured.title}</h1>
        <p class="mt-2 text-muted-foreground">by {featured.author} · {featured.downloads} downloads</p>
        <div class="mt-6 flex flex-wrap items-center justify-center gap-3 sm:justify-start">
          <a href="/play/{featured.id}" use:link onclick={play} class={buttonVariants({ size: 'lg' })}>
            <PlayIcon data-icon="inline-start" />
            Play now
          </a>
          <a href="/cart/{featured.id}" use:link class={buttonVariants({ variant: 'secondary', size: 'lg' })}>Cart details</a>
        </div>
      </div>

      <a
        href="/cart/{featured.id}"
        use:link
        class="cart-notch group animate-power-on relative order-1 block size-56 shrink-0 overflow-hidden bg-secondary shadow-2xl shadow-black/60 ring-1 ring-white/10 sm:order-2 sm:size-72"
      >
        {#if featured.has_screenshot}
          <img
            src={api.screenshotUrl(featured.id)}
            alt={featured.title}
            class="size-full object-cover transition-transform duration-300 group-hover:scale-105"
            style="image-rendering: pixelated;"
          />
        {/if}
        <div class="scanline-overlay pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-300 group-hover:opacity-50"></div>
        <div class="label-mono absolute top-2 left-2 rounded-sm bg-black/55 px-1.5 py-0.5 text-[10px] text-white/65 backdrop-blur-sm">
          #{featured.id.slice(0, 6)}
        </div>
      </a>
    </div>
  </section>
{/if}

<div class="container-page py-12">
  {#if error}
    <Alert.Root variant="destructive" class="mb-6">
      <CircleAlertIcon />
      <Alert.Description>{error}</Alert.Description>
    </Alert.Root>
  {/if}

  {#if loading}
    <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-6">
      {#each Array(6) as _}
        <Skeleton class="aspect-square w-full rounded-lg" />
      {/each}
    </div>
  {:else}
    <section class="mb-10">
      <div class="mb-4 flex items-end justify-between">
        <h2 class="text-xl font-semibold">Top rated</h2>
        <a href="/browse?sort=top" use:link class="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
          See all
          <ArrowRightIcon class="size-3.5" />
        </a>
      </div>
      <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-6">
        {#each top as cart (cart.id)}
          <CartCard {cart} />
        {/each}
      </div>
    </section>

    <section class="mb-10">
      <div class="mb-4 flex items-end justify-between">
        <h2 class="text-xl font-semibold">Trending</h2>
        <a href="/browse?sort=popular" use:link class="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
          See all
          <ArrowRightIcon class="size-3.5" />
        </a>
      </div>
      <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-6">
        {#each trending as cart (cart.id)}
          <CartCard {cart} />
        {/each}
      </div>
    </section>

    <section>
      <div class="mb-4 flex items-end justify-between">
        <h2 class="text-xl font-semibold">New releases</h2>
        <a href="/browse?sort=new" use:link class="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
          See all
          <ArrowRightIcon class="size-3.5" />
        </a>
      </div>
      <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-6">
        {#each recent as cart (cart.id)}
          <CartCard {cart} />
        {/each}
      </div>
    </section>
  {/if}
</div>
