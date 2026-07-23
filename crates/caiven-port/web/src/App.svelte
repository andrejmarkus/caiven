<script lang="ts">
  import './app.css';
  import { route, matchRoute } from './router.svelte';
  import { hydrateUser, currentUser } from './stores.svelte';
  import Navbar from './components/Navbar.svelte';
  import Footer from './components/Footer.svelte';
  import Home from './pages/Home.svelte';
  import Browse from './pages/Browse.svelte';
  import CartDetail from './pages/CartDetail.svelte';
  import Play from './pages/Play.svelte';
  import Author from './pages/Author.svelte';
  import Login from './pages/Login.svelte';
  import Register from './pages/Register.svelte';
  import Upload from './pages/Upload.svelte';
  import Profile from './pages/Profile.svelte';
  import { Toaster } from '$lib/components/ui/sonner';

  hydrateUser();

  const match = $derived(matchRoute(route.path));
</script>

<Toaster position="bottom-right" />
<Navbar />

{#if !currentUser.loaded}
  <div class="container-page py-10 text-sm text-muted-foreground">Loading…</div>
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
  <div class="container-page py-20 text-center">
    <h1 class="text-2xl">Page not found</h1>
    <p class="mt-2 text-muted-foreground">This page doesn't exist. Check the address, or head back to the browse page.</p>
  </div>
{/if}

<Footer />
