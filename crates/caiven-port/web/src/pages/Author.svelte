<script lang="ts">
  import { api, type UserProfile } from '../api';
  import CartCard from '../components/CartCard.svelte';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import * as Alert from '$lib/components/ui/alert';
  import * as Empty from '$lib/components/ui/empty';
  import { Badge } from '$lib/components/ui/badge';
  import UserIcon from '@lucide/svelte/icons/user';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';

  let { username }: { username: string } = $props();

  let profile = $state<UserProfile | null>(null);
  let loading = $state(true);
  let error = $state('');

  $effect(() => {
    username;
    (async () => {
      loading = true;
      error = '';
      try {
        profile = await api.userProfile(username);
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      } finally {
        loading = false;
      }
    })();
  });
</script>

<div class="container-page py-10">
  {#if error}
    <Alert.Root variant="destructive" class="mb-6">
      <CircleAlertIcon />
      <Alert.Description>{error}</Alert.Description>
    </Alert.Root>
  {/if}

  {#if loading}
    <Skeleton class="mb-6 h-8 w-48" />
    <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-6">
      {#each Array(6) as _}
        <Skeleton class="aspect-square w-full rounded-lg" />
      {/each}
    </div>
  {:else if profile}
    <h1 class="mb-1 flex items-center gap-2 text-2xl font-semibold">
      {profile.username}
      {#if profile.is_admin}<Badge variant="secondary">admin</Badge>{/if}
    </h1>
    <p class="mb-6 text-sm text-muted-foreground">Joined {profile.created_at} · {profile.total} carts</p>
    {#if profile.carts.length === 0}
      <Empty.Root>
        <Empty.Header>
          <Empty.Media variant="icon"><UserIcon /></Empty.Media>
          <Empty.Title>No carts yet</Empty.Title>
        </Empty.Header>
      </Empty.Root>
    {:else}
      <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-6">
        {#each profile.carts as cart (cart.id)}
          <CartCard {cart} />
        {/each}
      </div>
    {/if}
  {/if}
</div>
