<script lang="ts">
  import { api } from '../api';
  import { currentUser, setUser } from '../stores.svelte';
  import { link, navigate } from '../router.svelte';
  import Logo from '$lib/components/Logo.svelte';
  import { buttonVariants } from '@caiven/ui/button';
  import * as DropdownMenu from '@caiven/ui/dropdown-menu';
  import SearchIcon from '@lucide/svelte/icons/search';
  import UploadIcon from '@lucide/svelte/icons/upload';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import UserIcon from '@lucide/svelte/icons/user';
  import ChartIcon from '@lucide/svelte/icons/chart-no-axes-column-increasing';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import LogOutIcon from '@lucide/svelte/icons/log-out';
  import TagsIcon from '@lucide/svelte/icons/tags';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import BookIcon from '@lucide/svelte/icons/book-marked';

  let q = $state('');
  let searchInput = $state<HTMLInputElement | undefined>();

  function search(e: Event) {
    e.preventDefault();
    navigate(`/browse${q.trim() ? `?q=${encodeURIComponent(q.trim())}` : ''}`);
  }

  async function logout() {
    await api.logout();
    setUser(null);
    navigate('/');
  }

  $effect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        searchInput?.focus();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

<header class="sticky top-0 z-30 flex h-16 items-center gap-4 border-b border-border bg-background/95 px-4 backdrop-blur md:px-7">
  <a href="/" use:link class="flex items-center gap-2 md:hidden">
    <Logo size={28} />
    <span class="font-display text-sm font-semibold text-foreground">Caiven Port</span>
  </a>
  <form onsubmit={search} class="hidden w-full max-w-[520px] sm:block">
    <div class="flex h-10 items-center gap-2.5 rounded-md border border-border bg-card px-3 focus-within:border-primary">
      <SearchIcon class="size-4 text-muted-foreground" />
      <input
        bind:this={searchInput}
        bind:value={q}
        placeholder="Search carts, creators, tags…"
        class="min-w-0 flex-1 border-0 bg-transparent p-0 text-sm text-foreground outline-none ring-0 placeholder:text-muted-foreground"
      />
      <kbd class="hidden rounded border border-border px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground lg:block">⌘K</kbd>
    </div>
  </form>
  <div class="ml-auto flex items-center gap-2">
    {#if currentUser.value}
      <a href="/upload" use:link class={buttonVariants({ size: 'sm', class: 'hidden sm:inline-flex h-10' })}>
        <UploadIcon data-icon="inline-start" />
        Publish a cart
      </a>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class={buttonVariants({ variant: 'secondary', size: 'sm', class: 'h-10' })}>
          <span class="flex size-7 items-center justify-center rounded-full bg-accent font-display text-xs font-bold text-accent-foreground">
            {currentUser.value.username[0]?.toUpperCase()}
          </span>
          <span class="hidden lg:inline">{currentUser.value.username}</span>
          <ChevronDownIcon data-icon="inline-end" />
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="end">
          <DropdownMenu.Item class="md:hidden" onclick={() => navigate('/tags')}><TagsIcon />Tags</DropdownMenu.Item>
          <DropdownMenu.Item class="md:hidden" onclick={() => navigate('/collections')}><BookIcon />Collections</DropdownMenu.Item>
          <DropdownMenu.Item class="md:hidden" onclick={() => navigate('/jams')}><TrophyIcon />Jams</DropdownMenu.Item>
          <DropdownMenu.Item onclick={() => navigate(`/author/${currentUser.value?.username}`)}><UserIcon />Public profile</DropdownMenu.Item>
          <DropdownMenu.Item onclick={() => navigate('/dashboard')}><ChartIcon />Creator stats</DropdownMenu.Item>
          <DropdownMenu.Item onclick={() => navigate('/settings')}><SettingsIcon />Settings</DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item onclick={logout}><LogOutIcon />Log out</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    {:else}
      <a href="/login" use:link class={buttonVariants({ variant: 'ghost', size: 'sm' })}>Log in</a>
      <a href="/register" use:link class={buttonVariants({ size: 'sm' })}>Join</a>
    {/if}
  </div>
</header>
