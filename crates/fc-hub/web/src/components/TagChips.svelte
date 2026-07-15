<script lang="ts">
  import { link } from '../router.svelte';

  let { tags }: { tags: string[] } = $props();
</script>

{#if tags.length}
  <span class="chips">
    {#each tags as tag (tag)}
      <a href="/browse?tag={encodeURIComponent(tag)}" use:link class="chip">{tag}</a>
    {/each}
  </span>
{/if}

<style>
  .chips {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0.35em;
  }
  .chip {
    font-size: 0.8em;
    background: var(--bg-panel-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.15em 0.6em;
    color: var(--text-dim);
  }
  .chip:hover {
    color: var(--accent);
    text-decoration: none;
    border-color: var(--accent);
  }
</style>
