<script lang="ts">
  import { route, link } from '../router.svelte';
  import { currentUser } from '../stores.svelte';
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
  import ShieldIcon from '@lucide/svelte/icons/shield';

  const baseGroups = [
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

  const groups = $derived(
    currentUser.value?.is_admin
      ? [
          ...baseGroups,
          {
            label: 'Admin',
            items: [{ href: '/admin/users', label: 'Users', icon: ShieldIcon }],
          },
        ]
      : baseGroups,
  );

  function active(href: string) {
    return href === '/' ? route.path === '/' : route.path === href || route.path.startsWith(`${href}/`);
  }
</script>

<aside class="hidden h-screen w-[236px] shrink-0 flex-col border-r border-border bg-background md:sticky md:top-0 md:flex">
  <a href="/" use:link class="flex items-center gap-3 px-5 py-[22px] text-foreground hover:text-foreground">
    <Logo size={30} />
    <span class="font-display text-base font-bold">Caiven</span>
  </a>

  <nav class="flex flex-1 flex-col gap-5 overflow-y-auto px-3">
    {#each groups as group}
      <div>
        <div class="px-2.5 pb-1.5 text-xs text-muted-foreground">{group.label}</div>
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

  <a
    href="https://github.com/andrejmarkus/caiven/releases/latest"
    class="m-3 flex items-center gap-2.5 rounded-md px-2.5 py-2 text-sm text-muted-foreground hover:bg-secondary hover:text-foreground"
  >
    <DownloadIcon class="size-4" />
    Download Studio
  </a>
</aside>
