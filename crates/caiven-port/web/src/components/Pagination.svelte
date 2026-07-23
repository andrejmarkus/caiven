<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';

  let {
    page,
    perPage,
    total,
    onchange,
  }: {
    page: number;
    perPage: number;
    total: number;
    onchange: (page: number) => void;
  } = $props();

  const pageCount = $derived(Math.max(1, Math.ceil(total / perPage)));
</script>

{#if pageCount > 1}
  <div class="mt-10 flex items-center justify-center gap-3">
    <Button variant="secondary" size="sm" disabled={page <= 0} onclick={() => onchange(page - 1)}>
      <ChevronLeftIcon data-icon="inline-start" />
      Prev
    </Button>
    <span class="text-sm text-muted-foreground">Page {page + 1} of {pageCount}</span>
    <Button variant="secondary" size="sm" disabled={page + 1 >= pageCount} onclick={() => onchange(page + 1)}>
      Next
      <ChevronRightIcon data-icon="inline-end" />
    </Button>
  </div>
{/if}
