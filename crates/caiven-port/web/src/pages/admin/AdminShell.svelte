<script lang="ts">
  import { link } from '../../router.svelte';
  import AdminUsers from './AdminUsers.svelte';

  let { section }: { section: string } = $props();

  const tabs = [{ id: 'users', label: 'Users' }];
</script>

<div class="container-page py-8 md:py-10">
  <h1 class="page-title">Admin</h1>
  <p class="mt-1 text-sm text-muted-foreground">Port-wide management. Admin-only.</p>

  <nav class="mt-6 flex gap-1 border-b border-border">
    {#each tabs as tab}
      <a
        href="/admin/{tab.id}"
        use:link
        aria-current={section === tab.id ? 'page' : undefined}
        class="border-b-2 px-3 py-2 text-sm font-semibold transition-colors"
        class:border-primary={section === tab.id}
        class:text-primary={section === tab.id}
        class:border-transparent={section !== tab.id}
        class:text-muted-foreground={section !== tab.id}
      >
        {tab.label}
      </a>
    {/each}
  </nav>

  <div class="mt-6">
    {#if section === 'users'}
      <AdminUsers />
    {/if}
  </div>
</div>
