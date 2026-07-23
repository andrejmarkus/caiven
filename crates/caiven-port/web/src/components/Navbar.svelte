<script lang="ts">
  import { link, navigate } from '../router.svelte';
  import { currentUser, setUser } from '../stores.svelte';
  import { api } from '../api';
  import Logo from '$lib/components/Logo.svelte';
  import { buttonVariants } from '$lib/components/ui/button';
  import * as InputGroup from '$lib/components/ui/input-group';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import UploadIcon from '@lucide/svelte/icons/upload';
  import LibraryIcon from '@lucide/svelte/icons/library';
  import UserIcon from '@lucide/svelte/icons/user';
  import LogOutIcon from '@lucide/svelte/icons/log-out';
  import SearchIcon from '@lucide/svelte/icons/search';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';

  let q = $state('');

  async function logout() {
    await api.logout();
    setUser(null);
    navigate('/');
  }

  function onSearch(e: Event) {
    e.preventDefault();
    navigate(`/browse${q ? `?q=${encodeURIComponent(q)}` : ''}`);
  }
</script>

<header class="sticky top-0 z-20 border-b border-border bg-secondary shadow-md shadow-black/20">
  <div class="container-page flex h-16 items-center gap-4">
    <a href="/" use:link class="mr-4 flex shrink-0 items-center gap-2.5">
      <Logo size={28} />
      <span class="font-display text-base font-semibold text-foreground">Caiven Port</span>
    </a>

    <nav class="hidden md:flex">
      <a href="/browse" use:link class={buttonVariants({ variant: 'ghost', size: 'sm' })}>
        <LibraryIcon data-icon="inline-start" />
        Browse
      </a>
    </nav>

    <form onsubmit={onSearch} class="ml-auto hidden max-w-xs flex-1 sm:block">
      <InputGroup.Root class="h-9 border-border bg-background">
        <InputGroup.Addon>
          <SearchIcon />
        </InputGroup.Addon>
        <InputGroup.Input placeholder="Search carts…" bind:value={q} />
      </InputGroup.Root>
    </form>

    <div class="flex items-center gap-2">
      {#if currentUser.value}
        <a href="/upload" use:link class={buttonVariants({ variant: 'secondary', size: 'sm', class: 'hidden sm:inline-flex' })}>
          <UploadIcon data-icon="inline-start" />
          Upload
        </a>
        <DropdownMenu.Root>
          <DropdownMenu.Trigger class={buttonVariants({ variant: 'secondary', size: 'sm' })}>
            <UserIcon data-icon="inline-start" />
            {currentUser.value.username}
            <ChevronDownIcon data-icon="inline-end" />
          </DropdownMenu.Trigger>
          <DropdownMenu.Content align="end">
            <DropdownMenu.Group>
              <DropdownMenu.Item onclick={() => navigate('/profile')}>
                <UserIcon data-icon="inline-start" />
                Profile
              </DropdownMenu.Item>
              <DropdownMenu.Item onclick={logout}>
                <LogOutIcon data-icon="inline-start" />
                Log out
              </DropdownMenu.Item>
            </DropdownMenu.Group>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      {:else}
        <a href="/login" use:link class={buttonVariants({ variant: 'ghost', size: 'sm' })}>Log in</a>
        <a href="/register" use:link class={buttonVariants({ variant: 'default', size: 'sm' })}>Register</a>
      {/if}
    </div>
  </div>
</header>
