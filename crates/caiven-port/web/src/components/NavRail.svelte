<script lang="ts">
  import { route, link } from '../router.svelte';
  import Logo from '$lib/components/Logo.svelte';
  import HomeIcon from '@lucide/svelte/icons/house';
  import GridIcon from '@lucide/svelte/icons/layout-grid';
  import TagsIcon from '@lucide/svelte/icons/tags';
  import CollectionsIcon from '@lucide/svelte/icons/book-marked';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import ActivityIcon from '@lucide/svelte/icons/activity';
  import LibraryIcon from '@lucide/svelte/icons/library';
  import ChartIcon from '@lucide/svelte/icons/chart-no-axes-column-increasing';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import DownloadIcon from '@lucide/svelte/icons/download';

  const groups = [
    {
      label: 'Discover',
      items: [
        { href: '/', label: 'Home', icon: HomeIcon },
        { href: '/browse', label: 'Browse', icon: GridIcon },
        { href: '/tags', label: 'Tags', icon: TagsIcon },
        { href: '/collections', label: 'Collections', icon: CollectionsIcon },
        { href: '/jams', label: 'Jams', icon: TrophyIcon },
      ],
    },
    {
      label: 'You',
      items: [
        { href: '/activity', label: 'Activity', icon: ActivityIcon },
        { href: '/library', label: 'Library', icon: LibraryIcon },
        { href: '/dashboard', label: 'Creator stats', icon: ChartIcon },
        { href: '/settings', label: 'Settings', icon: SettingsIcon },
      ],
    },
  ];

  function active(href: string) {
    return href === '/' ? route.path === '/' : route.path === href || route.path.startsWith(`${href}/`);
  }
</script>

<aside class="hidden h-screen w-[236px] shrink-0 flex-col border-r border-border bg-background md:sticky md:top-0 md:flex">
  <a href="/" use:link class="flex items-center gap-3 px-5 py-[22px] text-foreground hover:text-foreground">
    <Logo size={30} />
    <span>
      <span class="block font-display text-base font-semibold leading-none">Caiven</span>
      <span class="label-mono mt-1 block text-[10px] text-muted-foreground">Port</span>
    </span>
  </a>

  <nav class="flex flex-1 flex-col gap-5 overflow-y-auto px-3">
    {#each groups as group}
      <div>
        <div class="label-mono px-2 pb-1.5 text-[10px] text-muted-foreground">{group.label}</div>
        <div class="space-y-0.5">
          {#each group.items as item}
            <a
              href={item.href}
              use:link
              aria-current={active(item.href) ? 'page' : undefined}
              class="flex items-center gap-2.5 rounded-md px-2.5 py-2 text-sm transition-colors hover:bg-secondary hover:text-foreground"
              class:bg-card={active(item.href)}
              class:font-semibold={active(item.href)}
              class:text-primary={active(item.href)}
              class:text-muted-foreground={!active(item.href)}
            >
              <item.icon class="size-4" />
              {item.label}
            </a>
          {/each}
        </div>
      </div>
    {/each}
  </nav>

  <div class="m-3 rounded-lg border border-border bg-card p-3.5">
    <div class="font-display text-sm font-semibold">Build a cart</div>
    <p class="mt-1 text-xs leading-relaxed text-muted-foreground">Caiven Studio is free, offline, and ships with the console.</p>
    <a
      href="https://github.com/andrejmarkus/caiven"
      class="mt-3 flex h-8 items-center justify-center gap-2 rounded-md bg-secondary px-3 text-xs font-semibold text-foreground hover:bg-muted"
    >
      <DownloadIcon class="size-3.5" />
      Get Studio
    </a>
  </div>
</aside>
