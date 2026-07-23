<script lang="ts">
  import type { Cart } from '../api';
  import { link, navigate } from '../router.svelte';
  import ScreenshotImg from './ScreenshotImg.svelte';
  import RatingStars from './RatingStars.svelte';

  let { cart }: { cart: Cart } = $props();

  function play(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    navigate(`/play/${cart.id}`);
  }
</script>

<a class="card" href="/cart/{cart.id}" use:link>
  <div class="shot">
    <ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} alt={cart.title} />
    <button class="play-btn" onclick={play}>Play</button>
  </div>
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
  .shot {
    position: relative;
  }
  .play-btn {
    position: absolute;
    inset: 0;
    margin: auto;
    width: 3.5rem;
    height: 3.5rem;
    border-radius: 50%;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .shot:hover .play-btn {
    opacity: 1;
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
