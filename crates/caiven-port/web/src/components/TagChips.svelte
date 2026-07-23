<script lang="ts">
  import { Badge } from '$lib/components/ui/badge';
  import { navigate } from '../router.svelte';

  let { tags }: { tags: string[] } = $props();

  function go(e: MouseEvent, tag: string) {
    e.preventDefault();
    navigate(`/browse?tag=${encodeURIComponent(tag)}`);
  }
</script>

{#if tags.length}
  <span class="inline-flex flex-wrap gap-1.5">
    {#each tags as tag (tag)}
      <Badge variant="secondary" href="/browse?tag={encodeURIComponent(tag)}" onclick={(e: MouseEvent) => go(e, tag)} class="hover:bg-primary/20 hover:text-accent-foreground">
        {tag}
      </Badge>
    {/each}
  </span>
{/if}
