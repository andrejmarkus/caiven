<script lang="ts">
  import { api, type Cart } from '../api';
  import CartCard from '../components/CartCard.svelte';
  import { link } from '../router.svelte';

  let top = $state<Cart[]>([]);
  let trending = $state<Cart[]>([]);
  let loading = $state(true);
  let error = $state('');

  $effect(() => {
    (async () => {
      loading = true;
      try {
        const [t, p] = await Promise.all([
          api.listCarts({ per_page: 6, sort: 'top' }),
          api.listCarts({ per_page: 6, sort: 'popular' }),
        ]);
        top = t.carts;
        trending = p.carts;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      } finally {
        loading = false;
      }
    })();
  });
</script>

<div class="container">
  <div class="hero">
    <h1>Caiven Port</h1>
    <p class="muted">Discover, rate and share carts for the fantasy console.</p>
    <a href="/browse" use:link><button>Browse all carts</button></a>
  </div>

  {#if error}<p class="error">{error}</p>{/if}
  {#if loading}
    <p class="muted">loading…</p>
  {:else}
    <section>
      <h2>Top rated</h2>
      <div class="grid">
        {#each top as cart (cart.id)}
          <CartCard {cart} />
        {/each}
      </div>
    </section>

    <section>
      <h2>Trending</h2>
      <div class="grid">
        {#each trending as cart (cart.id)}
          <CartCard {cart} />
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .hero {
    text-align: center;
    padding: 2rem 0 2.5rem;
  }
  section {
    margin-bottom: 2rem;
  }
</style>
