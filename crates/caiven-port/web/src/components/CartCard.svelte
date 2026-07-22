<script lang="ts">
  import type { Cart } from '../api';
  import { link } from '../router.svelte';
  import ScreenshotImg from './ScreenshotImg.svelte';
  import RatingStars from './RatingStars.svelte';

  let { cart }: { cart: Cart } = $props();
</script>

<a class="card" href="/cart/{cart.id}" use:link>
  <ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} alt={cart.title} />
  <div class="body">
    <h3>{cart.title}</h3>
    <p class="author">by {cart.author}</p>
    <div class="row meta">
      <RatingStars value={cart.rating_avg} />
      <span class="muted">({cart.rating_count})</span>
      <span class="muted">· {cart.downloads} dl</span>
    </div>
  </div>
</a>

<style>
  .card {
    display: block;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    color: inherit;
  }
  .card:hover {
    border-color: var(--accent);
    text-decoration: none;
  }
  .body {
    padding: 0.6rem 0.75rem;
  }
  h3 {
    margin: 0 0 0.15em;
    font-size: 1em;
    color: var(--text);
  }
  .author {
    margin: 0;
    font-size: 0.85em;
    color: var(--text-dim);
  }
  .meta {
    margin-top: 0.4em;
    font-size: 0.85em;
  }
</style>
