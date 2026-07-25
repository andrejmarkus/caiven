<script lang="ts">
  import { api, type FeedEvent } from '../api';
  import ScreenshotImg from '../components/ScreenshotImg.svelte';
  import { link } from '../router.svelte';
  let events = $state<FeedEvent[]>([]);
  let error = $state('');
  $effect(() => { (async () => { try { events = (await api.feed()).events; } catch (e) { error = e instanceof Error ? e.message : String(e); } })(); });
  function text(event: FeedEvent) {
    if (event.kind === 'version_published') return `shipped v${event.version} of`;
    if (event.kind === 'collection_addition') return `added a cart to ${event.collection_title}`;
    if (event.kind === 'jam_entry') return `entered ${event.jam_title} with`;
    return 'published';
  }
</script>
<div class="container-page max-w-[820px] py-8 md:py-10">
  <h1 class="page-title">Activity</h1><p class="mt-1 mb-7 text-sm text-muted-foreground">New work from creators and collections you follow.</p>
  {#if error}<div class="mb-5 rounded-lg border border-destructive/50 p-4 text-destructive">{error}</div>{/if}
  <div class="space-y-3">
    {#each events as event}
      <article class="surface-panel flex gap-4 rounded-lg p-4">
        <span class="flex size-9 shrink-0 items-center justify-center rounded-full bg-secondary font-display font-semibold">{event.actor[0]?.toUpperCase()}</span>
        <div class="min-w-0 flex-1"><p class="text-sm text-muted-foreground"><strong class="text-foreground">{event.actor}</strong> {text(event)} · <time class="font-mono text-xs">{new Date(event.occurred_at).toLocaleDateString()}</time></p>
          <a href="/cart/{event.cart.id}" use:link class="mt-3 flex items-center gap-3 rounded-md border border-[var(--border-subtle)] bg-background p-3 text-foreground hover:border-primary hover:text-foreground">
            <span class="cart-notch size-14 shrink-0 overflow-hidden bg-secondary"><ScreenshotImg id={event.cart.id} hasScreenshot={event.cart.has_screenshot} alt="" /></span>
            <span class="min-w-0 flex-1"><strong class="block truncate">{event.cart.title}</strong><span class="mt-0.5 block truncate text-sm text-muted-foreground">{event.cart.description}</span></span>
            <span class="flex size-8 items-center justify-center rounded-full bg-primary text-primary-foreground">▶</span>
          </a>
        </div>
      </article>
    {:else}<div class="py-20 text-center text-muted-foreground">Follow creators or collections to build your feed.</div>{/each}
  </div>
</div>
