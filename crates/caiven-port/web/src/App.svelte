<script lang="ts">
  import './app.css';
  import { route, matchRoute } from './router.svelte';
  import { hydrateUser, currentUser } from './stores.svelte';
  import AppShell from './components/AppShell.svelte';
  import Home from './pages/Home.svelte';
  import Browse from './pages/Browse.svelte';
  import Tags from './pages/Tags.svelte';
  import Collections from './pages/Collections.svelte';
  import CollectionDetail from './pages/CollectionDetail.svelte';
  import Jams from './pages/Jams.svelte';
  import JamDetail from './pages/JamDetail.svelte';
  import Activity from './pages/Activity.svelte';
  import Library from './pages/Library.svelte';
  import Dashboard from './pages/Dashboard.svelte';
  import Settings from './pages/Settings.svelte';
  import CartDetail from './pages/CartDetail.svelte';
  import Play from './pages/Play.svelte';
  import Author from './pages/Author.svelte';
  import Login from './pages/Login.svelte';
  import Register from './pages/Register.svelte';
  import VerifyEmail from './pages/VerifyEmail.svelte';
  import ForgotPassword from './pages/ForgotPassword.svelte';
  import ResetPassword from './pages/ResetPassword.svelte';
  import Upload from './pages/Upload.svelte';
  import { Toaster } from '$lib/components/ui/sonner';

  hydrateUser();
  const match = $derived(matchRoute(route.path));
  const protectedPage = $derived(['activity', 'dashboard', 'settings', 'upload'].includes(match.name));
</script>

<Toaster position="bottom-right" />
<AppShell>
  {#if !currentUser.loaded}
    <div class="container-page py-16 text-sm text-muted-foreground">Loading Port…</div>
  {:else if protectedPage && !currentUser.value}
    <Login />
  {:else if match.name === 'home'}<Home />
  {:else if match.name === 'browse'}<Browse />
  {:else if match.name === 'tags'}<Tags />
  {:else if match.name === 'collections'}<Collections />
  {:else if match.name === 'collection'}<CollectionDetail slug={match.params.slug} />
  {:else if match.name === 'jams'}<Jams />
  {:else if match.name === 'jam'}<JamDetail slug={match.params.slug} />
  {:else if match.name === 'activity'}<Activity />
  {:else if match.name === 'library'}<Library />
  {:else if match.name === 'dashboard'}<Dashboard />
  {:else if match.name === 'settings'}<Settings />
  {:else if match.name === 'cart'}<CartDetail id={match.params.id} />
  {:else if match.name === 'play'}<Play id={match.params.id} />
  {:else if match.name === 'author'}<Author username={match.params.username} />
  {:else if match.name === 'login'}<Login />
  {:else if match.name === 'register'}<Register />
  {:else if match.name === 'verify-email'}<VerifyEmail />
  {:else if match.name === 'forgot-password'}<ForgotPassword />
  {:else if match.name === 'reset-password'}<ResetPassword />
  {:else if match.name === 'upload'}<Upload />
  {:else}
    <div class="container-page py-24 text-center"><h1 class="text-2xl font-semibold">Page not found</h1><p class="mt-2 text-muted-foreground">Address points beyond Port.</p></div>
  {/if}
</AppShell>
