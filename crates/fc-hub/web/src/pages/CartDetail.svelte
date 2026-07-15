<script lang="ts">
  import { api, ApiError, type CartDetail } from '../api';
  import RatingStars from '../components/RatingStars.svelte';
  import TagChips from '../components/TagChips.svelte';
  import ScreenshotImg from '../components/ScreenshotImg.svelte';
  import CommentList from '../components/CommentList.svelte';
  import { currentUser } from '../stores.svelte';
  import { navigate, link } from '../router.svelte';

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
    if (!confirm(`Delete "${cart.title}"? This cannot be undone.`)) return;
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

<div class="container">
  {#if error}<p class="error">{error}</p>{/if}
  {#if loading}
    <p class="muted">loading…</p>
  {:else if cart}
    <div class="detail">
      <div class="shot">
        <ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} alt={cart.title} />
      </div>
      <div class="info">
        <h1>{cart.title}</h1>
        <p class="muted">
          by {#if cart.owner}<a href="/author/{cart.owner}" use:link>{cart.owner}</a>{:else}unknown{/if}
          · v{cart.latest_version} · {cart.downloads} downloads
        </p>
        <div class="row">
          <RatingStars value={cart.rating_avg} />
          <span class="muted">{cart.rating_avg.toFixed(1)} ({cart.rating_count})</span>
        </div>
        <TagChips tags={cart.tags} />
        <p>{cart.description}</p>

        <div class="row actions">
          <a href={api.romUrl(cart.id)}><button>Download ROM</button></a>
          {#if isOwner}
            <a href="/upload?cart={cart.id}" use:link><button class="secondary">New version</button></a>
            <button class="danger" disabled={deleteBusy} onclick={removeCart}>Delete</button>
          {/if}
        </div>

        {#if currentUser.value}
          <div class="rate-widget">
            <span class="muted">Your rating:</span>
            <RatingStars value={cart.own_rating ?? 0} interactive onrate={rate} />
            {#if rateBusy}<span class="muted">saving…</span>{/if}
          </div>
        {/if}
      </div>
    </div>

    {#if cart.versions.length > 1}
      <section class="panel versions">
        <h2>Versions</h2>
        <ul>
          {#each [...cart.versions].reverse() as v (v.version)}
            <li class="row">
              <strong>v{v.version}</strong>
              <span class="muted">{v.created_at}</span>
              <span class="muted">{(v.rom_size / 1024).toFixed(1)} KB</span>
              {#if v.changelog}<span class="muted">— {v.changelog}</span>{/if}
              <a href={api.romUrl(cart.id, v.version)}>download</a>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <section class="panel">
      <CommentList cartId={cart.id} ownerUsername={cart.owner} />
    </section>
  {/if}
</div>

<style>
  .detail {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 1.5rem;
    margin-bottom: 1.5rem;
  }
  .info h1 {
    margin: 0 0 0.25em;
  }
  .actions {
    margin: 1rem 0;
  }
  .rate-widget {
    margin-top: 0.5rem;
  }
  .versions {
    margin-bottom: 1.5rem;
  }
  .versions ul {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  @media (max-width: 640px) {
    .detail {
      grid-template-columns: 1fr;
    }
  }
</style>
