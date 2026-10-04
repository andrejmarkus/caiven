<script lang="ts">
  import { api, type Cart } from '../api';
  import ScreenshotImg from './ScreenshotImg.svelte';
  import { currentUser, setUser } from '../stores.svelte';
  import { link, navigate } from '../router.svelte';
  import Logo from '$lib/components/Logo.svelte';
  import { buttonVariants } from '@caiven/ui/button';
  import { Input } from '@caiven/ui/input';
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
  let searchInput = $state<HTMLInputElement | null>(null);
  let results = $state<Cart[]>([]);
  let open = $state(false);
  let active = $state(-1);

  $effect(() => {
    const term = q.trim();
    active = -1;
    if (!term) return void (results = []);
    let stale = false;
    const timer = setTimeout(async () => {
      try {
        const list = await api.listCarts({ q: term, per_page: 5 });
        if (!stale) results = list.carts;
      } catch {
        if (!stale) results = [];
      }
    }, 200);
    return () => { stale = true; clearTimeout(timer); };
  });

  function close() {
    open = false;
    active = -1;
  }

  function search(e: Event) {
    e.preventDefault();
    const picked = results[active];
    close();
    if (picked) return navigate(`/cart/${picked.id}`);
    navigate(`/browse${q.trim() ? `?q=${encodeURIComponent(q.trim())}` : ''}`);
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === 'Escape') return close();
    if (!results.length || (e.key !== 'ArrowDown' && e.key !== 'ArrowUp')) return;
    e.preventDefault();
    open = true;
    const n = results.length;
    active = e.key === 'ArrowDown' ? (active + 1) % n : (active - 1 + n) % n;
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

<header class="sticky top-0 z-30 box-content flex h-16 pt-[env(safe-area-inset-top)] items-center gap-4 border-b border-border bg-background/95 px-4 backdrop-blur md:px-7">
  <a href="/" use:link class="flex items-center gap-2 md:hidden">
    <Logo size={28} />
    <span class="font-display text-sm font-semibold text-foreground">Caiven Port</span>
  </a>
  <form onsubmit={search} onfocusout={(e) => !e.currentTarget.contains(e.relatedTarget as Node) && close()} class="relative hidden w-full max-w-[520px] sm:block">
    <div class="flex h-10 items-center gap-2.5 rounded-md border border-border bg-card px-3 focus-within:border-primary">
      <SearchIcon class="size-4 text-muted-foreground" />
      <Input
        bind:ref={searchInput}
        bind:value={q}
        placeholder="Search carts, creators, tags…"
        role="combobox"
        aria-expanded={open && results.length > 0}
        aria-controls="search-preview"
        aria-activedescendant={active >= 0 ? `search-preview-${active}` : undefined}
        autocomplete="off"
        onfocus={() => (open = true)}
        oninput={() => (open = true)}
        onkeydown={onSearchKey}
        class="h-auto min-w-0 flex-1 border-0 bg-transparent p-0 text-sm text-foreground shadow-none outline-none ring-0 placeholder:text-muted-foreground focus-visible:border-0 focus-visible:ring-0"
      />
    </div>
    {#if open && q.trim() && results.length}
      <ul id="search-preview" role="listbox" class="absolute inset-x-0 top-full mt-1 overflow-hidden rounded-md border border-border bg-popover py-1 shadow-lg">
        {#each results as cart, i (cart.id)}
          <li id="search-preview-{i}" role="option" aria-selected={i === active}>
            <a
              href="/cart/{cart.id}"
              use:link
              onclick={close}
              class="flex items-center gap-3 px-3 py-2 text-foreground hover:bg-accent hover:no-underline {i === active ? 'bg-accent' : ''}"
            >
              <span class="h-8 w-12 shrink-0 overflow-hidden rounded-sm"><ScreenshotImg id={cart.id} hasScreenshot={cart.has_screenshot} /></span>
              <span class="min-w-0">
                <span class="block truncate text-sm font-medium">{cart.title}</span>
                <span class="block truncate text-xs text-muted-foreground">{cart.owner ?? cart.author}</span>
              </span>
            </a>
          </li>
        {/each}
        <li><button type="submit" class="w-full px-3 py-2 text-left text-xs text-muted-foreground hover:bg-accent">See all results for “{q.trim()}”</button></li>
      </ul>
    {/if}
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
