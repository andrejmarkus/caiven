<script lang="ts">
  import { route, matchRoute } from './router.svelte';
  import { hydrateUser, currentUser } from './stores.svelte';
  import Navbar from './components/Navbar.svelte';
  import Home from './pages/Home.svelte';
  import Browse from './pages/Browse.svelte';
  import CartDetail from './pages/CartDetail.svelte';
  import Play from './pages/Play.svelte';
  import Author from './pages/Author.svelte';
  import Login from './pages/Login.svelte';
  import Register from './pages/Register.svelte';
  import Upload from './pages/Upload.svelte';
  import Profile from './pages/Profile.svelte';

  hydrateUser();

  const match = $derived(matchRoute(route.path));
</script>

<Navbar />

{#if !currentUser.loaded}
  <div class="container"><p class="muted">loading…</p></div>
{:else if match.name === 'home'}
  <Home />
{:else if match.name === 'browse'}
  <Browse />
{:else if match.name === 'cart'}
  <CartDetail id={match.params.id} />
{:else if match.name === 'play'}
  <Play id={match.params.id} />
{:else if match.name === 'author'}
  <Author username={match.params.username} />
{:else if match.name === 'login'}
  <Login />
{:else if match.name === 'register'}
  <Register />
{:else if match.name === 'upload'}
  {#if currentUser.value}
    <Upload />
  {:else}
    <Login />
  {/if}
{:else if match.name === 'profile'}
  {#if currentUser.value}
    <Profile />
  {:else}
    <Login />
  {/if}
{:else}
  <div class="container"><h1>Not found</h1></div>
{/if}
