<script lang="ts">
  import { readHistory, type HistoryEntry } from '../history';
  import ScreenshotImg from '../components/ScreenshotImg.svelte';
  import { link } from '../router.svelte';
  let history = $state<HistoryEntry[]>([]);
  $effect(() => { history = readHistory(); });
</script>
<div class="container-page py-8 md:py-10">
  <h1 class="page-title">Your library</h1><p class="mt-1 mb-7 text-sm text-muted-foreground">Carts played in this browser, newest first.</p>
  <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
    {#each history as entry}
      <a href="/play/{entry.cart.id}" use:link class="surface-panel flex items-center gap-3 rounded-lg p-3.5 text-foreground hover:border-primary hover:text-foreground">
        <span class="cart-notch size-14 shrink-0 overflow-hidden bg-secondary"><ScreenshotImg id={entry.cart.id} hasScreenshot={entry.cart.has_screenshot} alt="" /></span>
        <span class="min-w-0 flex-1"><strong class="block truncate">{entry.cart.title}</strong><span class="label-mono mt-1 block text-[9px] text-muted-foreground">played {new Date(entry.played_at).toLocaleDateString()}</span></span>
        <span class="flex size-9 items-center justify-center rounded-full bg-primary text-primary-foreground">▶</span>
      </a>
    {:else}<div class="col-span-full py-20 text-center text-muted-foreground">Play a cart. It will appear here.</div>{/each}
  </div>
</div>
