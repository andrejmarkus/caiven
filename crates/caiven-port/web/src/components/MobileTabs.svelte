<script lang="ts">
  import { route, link } from '../router.svelte';
  import HomeIcon from '@lucide/svelte/icons/house';
  import BrowseIcon from '@lucide/svelte/icons/layout-grid';
  import LibraryIcon from '@lucide/svelte/icons/library';
  import UserIcon from '@lucide/svelte/icons/user';

  const tabs = [
    { href: '/', label: 'Home', icon: HomeIcon },
    { href: '/browse', label: 'Browse', icon: BrowseIcon },
    { href: '/library', label: 'Library', icon: LibraryIcon },
    { href: '/settings', label: 'You', icon: UserIcon },
  ];
</script>

<nav class="fixed inset-x-0 bottom-0 z-40 flex h-16 border-t border-border bg-card md:hidden">
  {#each tabs as tab}
    <a
      href={tab.href}
      use:link
      class="flex flex-1 flex-col items-center justify-center gap-1 text-[10px]"
      class:text-primary={route.path === tab.href || (tab.href !== '/' && route.path.startsWith(tab.href))}
      class:text-muted-foreground={!(route.path === tab.href || (tab.href !== '/' && route.path.startsWith(tab.href)))}
    >
      <tab.icon class="size-5" />
      {tab.label}
    </a>
  {/each}
</nav>
