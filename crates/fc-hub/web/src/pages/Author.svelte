<script lang="ts">
  import { api, type UserProfile } from '../api';
  import CartCard from '../components/CartCard.svelte';

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

<div class="container">
  {#if error}<p class="error">{error}</p>{/if}
  {#if loading}
    <p class="muted">loading…</p>
  {:else if profile}
    <h1>{profile.username}{profile.is_admin ? ' (admin)' : ''}</h1>
    <p class="muted">Joined {profile.created_at} · {profile.total} carts</p>
    <div class="grid">
      {#each profile.carts as cart (cart.id)}
        <CartCard {cart} />
      {/each}
    </div>
  {/if}
</div>
