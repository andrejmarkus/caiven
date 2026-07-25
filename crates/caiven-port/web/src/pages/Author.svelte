<script lang="ts">
  import { api, type UserProfile } from '../api';
  import { currentUser } from '../stores.svelte';
  import CartCard from '../components/CartCard.svelte';
  import { Button } from '$lib/components/ui/button';
  import { navigate } from '../router.svelte';

  let { username }: { username: string } = $props();
  let profile = $state<UserProfile | null>(null);
  let loading = $state(true);
  let error = $state('');
  async function load() { loading = true; try { profile = await api.userProfile(username, 0, 100); } catch (e) { error = e instanceof Error ? e.message : String(e); } finally { loading = false; } }
  $effect(() => { username; load(); });
  async function follow() {
    if (!profile) return;
    if (!currentUser.value) { navigate(`/login?next=/author/${username}`); return; }
    profile.followed_by_me ? await api.unfollowUser(username) : await api.followUser(username);
    await load();
  }
</script>

<div>
  {#if error}<div class="container-page py-6"><div class="rounded-lg border border-destructive/50 p-4 text-destructive">{error}</div></div>{/if}
  {#if loading}<div class="h-40 animate-pulse bg-card"></div>
  {:else if profile}
    <header class="border-b border-border bg-card">
      <div class="container-page flex flex-wrap items-center gap-5 py-8">
        <span class="flex size-18 items-center justify-center rounded-full bg-secondary font-display text-2xl font-bold">{profile.username[0]?.toUpperCase()}</span>
        <div><h1 class="text-3xl font-bold">{profile.username}</h1><div class="mt-2 flex flex-wrap gap-5 font-mono text-xs text-muted-foreground"><span>{profile.total} carts</span><span>{profile.total_plays.toLocaleString()} plays</span><span>{profile.follower_count} followers</span><span>joined {new Date(profile.created_at).toLocaleDateString()}</span></div></div>
        {#if currentUser.value?.username !== profile.username}<Button class="ml-auto" variant={profile.followed_by_me ? 'secondary' : 'default'} onclick={follow}>{profile.followed_by_me ? 'Following' : 'Follow'}</Button>{/if}
      </div>
    </header>
    <div class="container-page py-8"><div class="cart-grid">{#each profile.carts as cart (cart.id)}<CartCard {cart} />{:else}<p class="col-span-full py-20 text-center text-muted-foreground">No carts published.</p>{/each}</div></div>
  {/if}
</div>
