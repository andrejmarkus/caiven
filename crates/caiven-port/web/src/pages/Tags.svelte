<script lang="ts">
  import { api, type Cart, type TagCount } from '../api';
  import ScreenshotImg from '../components/ScreenshotImg.svelte';
  import { link } from '../router.svelte';

  let tags = $state<TagCount[]>([]);
  let previews = $state<Record<string, Cart[]>>({});
  let loading = $state(true);
  let error = $state('');

  $effect(() => {
    (async () => {
      try {
        tags = await api.listTags();
        const rows = await Promise.all(tags.map(async (tag) => [tag.tag, (await api.listCarts({ tag: tag.tag, per_page: 3 })).carts] as const));
        previews = Object.fromEntries(rows);
      } catch (e) { error = e instanceof Error ? e.message : String(e); }
      finally { loading = false; }
    })();
  });
</script>

<div class="container-page py-8 md:py-10">
  <h1 class="page-title">Tags & genres</h1>
  <p class="mt-1 mb-7 text-sm text-muted-foreground">Every tag creators used, with carts behind it.</p>
  {#if error}<div class="mb-6 rounded-lg border border-destructive/50 p-4 text-destructive">{error}</div>{/if}
  {#if loading}
    <div class="grid gap-[18px] md:grid-cols-2 xl:grid-cols-3">{#each Array(9) as _}<div class="h-52 animate-pulse rounded-lg bg-card"></div>{/each}</div>
  {:else}
    <div class="grid gap-[18px] md:grid-cols-2 xl:grid-cols-3">
      {#each tags as tag}
        <a href="/browse?tag={encodeURIComponent(tag.tag)}" use:link class="surface-panel rounded-lg p-5 text-foreground hover:border-primary hover:text-foreground">
          <div class="flex items-baseline gap-2"><h2 class="text-lg font-semibold">{tag.tag}</h2><span class="font-mono text-xs text-muted-foreground">{tag.count} carts</span></div>
          <div class="mt-4 grid grid-cols-3 gap-2">
            {#each previews[tag.tag] ?? [] as cart}
              <div class="cart-notch aspect-square overflow-hidden bg-secondary"><ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} alt="" /></div>
            {/each}
          </div>
          <p class="mt-3 truncate text-sm text-muted-foreground">{(previews[tag.tag] ?? []).map((c) => c.title).join(' · ')}</p>
        </a>
      {/each}
    </div>
  {/if}
</div>
