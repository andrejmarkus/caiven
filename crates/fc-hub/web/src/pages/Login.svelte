<script lang="ts">
  import { api, ApiError } from '../api';
  import { setUser } from '../stores.svelte';
  import { navigate, link } from '../router.svelte';

  let username = $state('');
  let password = $state('');
  let error = $state('');
  let busy = $state(false);

  async function submit(e: Event) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      const u = await api.login(username, password);
      setUser(u);
      navigate('/');
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Login failed';
    } finally {
      busy = false;
    }
  }
</script>

<div class="container narrow">
  <h1>Log in</h1>
  <form class="panel" onsubmit={submit}>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="field">
      <label for="u">Username</label>
      <input id="u" bind:value={username} autocomplete="username" required />
    </div>
    <div class="field">
      <label for="p">Password</label>
      <input id="p" type="password" bind:value={password} autocomplete="current-password" required />
    </div>
    <button type="submit" disabled={busy}>Log in</button>
  </form>
  <p class="muted">No account? <a href="/register" use:link>Register</a></p>
</div>

<style>
  .narrow {
    max-width: 360px;
  }
</style>
