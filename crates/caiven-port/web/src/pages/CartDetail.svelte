<script lang="ts">
  import { api, ApiError, type CartDetail } from '../api';
  import RatingStars from '../components/RatingStars.svelte';
  import TagChips from '../components/TagChips.svelte';
  import ScreenshotImg from '../components/ScreenshotImg.svelte';
  import CommentList from '../components/CommentList.svelte';
  import { currentUser } from '../stores.svelte';
  import { navigate, link } from '../router.svelte';
  import { Button, buttonVariants } from '$lib/components/ui/button';
  import * as AlertDialog from '$lib/components/ui/alert-dialog';
  import * as Alert from '$lib/components/ui/alert';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import PlayIcon from '@lucide/svelte/icons/play';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import UploadIcon from '@lucide/svelte/icons/upload';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';

  let { id }: { id: string } = $props();

  let cart = $state<CartDetail | null>(null);
  let loading = $state(true);
  let error = $state('');
  let rateBusy = $state(false);
  let deleteBusy = $state(false);

  async function load() {
    loading = true;
    error = '';
    try {
      cart = await api.getCart(id);
    } catch (e) {
      error = e instanceof ApiError ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    id;
    load();
  });

  const isOwner = $derived(!!cart && !!currentUser.value && (currentUser.value.username === cart.owner || currentUser.value.is_admin));

  async function rate(score: number) {
    if (!cart) return;
    rateBusy = true;
    try {
      await api.rateCart(cart.id, score);
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      rateBusy = false;
    }
  }

  async function removeCart() {
    if (!cart) return;
    deleteBusy = true;
    try {
      await api.deleteCart(cart.id);
      navigate('/browse');
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      deleteBusy = false;
    }
  }
</script>

<div class="container-page py-10">
  {#if error}
    <Alert.Root variant="destructive" class="mb-6">
      <CircleAlertIcon />
      <Alert.Description>{error}</Alert.Description>
    </Alert.Root>
  {/if}

  {#if loading}
    <div class="grid grid-cols-1 gap-8 sm:grid-cols-[320px_1fr]">
      <Skeleton class="aspect-square w-full rounded-xl" />
      <div class="flex flex-col gap-3">
        <Skeleton class="h-9 w-2/3" />
        <Skeleton class="h-4 w-1/3" />
        <Skeleton class="h-20 w-full" />
      </div>
    </div>
  {:else if cart}
    <div class="grid grid-cols-1 gap-8 sm:grid-cols-[320px_1fr]">
      <div class="cart-notch group animate-power-on relative aspect-square w-full overflow-hidden bg-secondary ring-1 ring-white/10">
        <ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} alt={cart.title} />
        <div class="scanline-overlay pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-300 group-hover:opacity-50"></div>
        <div class="label-mono absolute top-2 left-2 rounded-sm bg-black/55 px-1.5 py-0.5 text-[10px] text-white/65 backdrop-blur-sm">
          #{cart.id.slice(0, 6)}
        </div>
      </div>
      <div>
        <h1 class="text-3xl font-semibold">{cart.title}</h1>
        <p class="mt-2 text-sm text-muted-foreground">
          by {#if cart.owner}<a href="/author/{cart.owner}" use:link>{cart.owner}</a>{:else}unknown{/if}
          · v{cart.latest_version} · {cart.downloads} downloads
        </p>
        <div class="mt-3 flex items-center gap-2">
          <RatingStars value={cart.rating_avg} />
          <span class="text-sm text-muted-foreground">{cart.rating_avg.toFixed(1)} ({cart.rating_count})</span>
        </div>
        {#if cart.tags.length}<div class="mt-3"><TagChips tags={cart.tags} /></div>{/if}
        {#if cart.description}<p class="mt-4 max-w-2xl text-sm leading-relaxed text-foreground/90">{cart.description}</p>{/if}

        <div class="mt-6 flex flex-wrap items-center gap-3">
          <a href="/play/{cart.id}" use:link class={buttonVariants({ size: 'lg' })}>
            <PlayIcon data-icon="inline-start" />
            Play
          </a>
          <Button href={api.cartUrl(cart.id)} variant="secondary" size="lg">
            <DownloadIcon data-icon="inline-start" />
            Download
          </Button>
          {#if isOwner}
            <a href="/upload?cart={cart.id}" use:link class={buttonVariants({ variant: 'secondary', size: 'lg' })}>
              <UploadIcon data-icon="inline-start" />
              New version
            </a>
            <AlertDialog.Root>
              <AlertDialog.Trigger class={buttonVariants({ variant: 'destructive', size: 'lg' })}>
                <Trash2Icon data-icon="inline-start" />
                Delete
              </AlertDialog.Trigger>
              <AlertDialog.Content>
                <AlertDialog.Header>
                  <AlertDialog.Title>Delete "{cart.title}"?</AlertDialog.Title>
                  <AlertDialog.Description>This removes the cart and every published version. This cannot be undone.</AlertDialog.Description>
                </AlertDialog.Header>
                <AlertDialog.Footer>
                  <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
                  <AlertDialog.Action variant="destructive" disabled={deleteBusy} onclick={removeCart}>Delete</AlertDialog.Action>
                </AlertDialog.Footer>
              </AlertDialog.Content>
            </AlertDialog.Root>
          {/if}
        </div>

        {#if currentUser.value}
          <div class="mt-5 flex items-center gap-2 text-sm text-muted-foreground">
            Your rating:
            <RatingStars value={cart.own_rating ?? 0} interactive onrate={rate} />
            {#if rateBusy}<span>saving…</span>{/if}
          </div>
        {/if}
      </div>
    </div>

    {#if cart.versions.length > 1}
      <section class="mt-10 rounded-xl bg-card p-5">
        <h2 class="mb-3 text-lg font-semibold">Versions</h2>
        <ul class="flex flex-col divide-y divide-border text-sm">
          {#each [...cart.versions].reverse() as v (v.version)}
            <li class="flex flex-wrap items-center gap-3 py-2.5">
              <strong class="text-foreground">v{v.version}</strong>
              <span class="text-muted-foreground">{v.created_at}</span>
              <span class="text-muted-foreground">{(v.cart_size / 1024).toFixed(1)} KB</span>
              {#if v.changelog}<span class="text-muted-foreground">— {v.changelog}</span>{/if}
              <a href={api.cartUrl(cart.id, v.version)} class="ml-auto">download</a>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <section class="mt-10 rounded-xl bg-card p-5">
      <CommentList cartId={cart.id} ownerUsername={cart.owner} />
    </section>
  {/if}
</div>
