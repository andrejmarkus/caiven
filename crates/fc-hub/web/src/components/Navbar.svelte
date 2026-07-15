<script lang="ts">
  import { link, navigate } from '../router.svelte';
  import { currentUser, setUser } from '../stores.svelte';
  import { api } from '../api';

  async function logout() {
    await api.logout();
    setUser(null);
    navigate('/');
  }
</script>

<nav class="navbar">
  <div class="container row">
    <a class="brand" href="/" use:link>FC Hub</a>
    <a href="/browse" use:link>Browse</a>
    <div class="spacer"></div>
    {#if currentUser.value}
      <a href="/upload" use:link>Upload</a>
      <a href="/profile" use:link>{currentUser.value.username}</a>
      <button class="secondary" onclick={logout}>Log out</button>
    {:else}
      <a href="/login" use:link>Log in</a>
      <a href="/register" use:link>Register</a>
    {/if}
  </div>
</nav>

<style>
  .navbar {
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
  }
  .navbar .container {
    padding: 0.75rem 1rem;
  }
  .brand {
    font-family: var(--font-head);
    color: var(--accent);
    font-weight: 700;
    font-size: 1.1em;
  }
  .spacer {
    flex: 1;
  }
</style>
