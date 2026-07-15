<script lang="ts">
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
  <div class="row pager">
    <button class="secondary" disabled={page <= 0} onclick={() => onchange(page - 1)}>&larr; prev</button>
    <span class="muted">page {page + 1} / {pageCount}</span>
    <button class="secondary" disabled={page + 1 >= pageCount} onclick={() => onchange(page + 1)}>next &rarr;</button>
  </div>
{/if}

<style>
  .pager {
    justify-content: center;
    margin: 1.5rem 0;
  }
</style>
