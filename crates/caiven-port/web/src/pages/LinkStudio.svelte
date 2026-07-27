<script lang="ts">
  import { api, ApiError } from '../api';
  import { currentUser } from '../stores.svelte';
  import { navigate, route } from '../router.svelte';
  import * as Card from '@caiven/ui/card';
  import { Button } from '@caiven/ui/button';
  const requestId = route.search.get('request') ?? '';
  let busy = $state(false);
  let message = $state('');
  let failed = $state(false);
  function login() { navigate(`/login?next=${encodeURIComponent(`/link-studio?request=${requestId}`)}`); }
  async function approve() {
    busy = true; message = ''; failed = false;
    try {
      await api.approveStudioLink(requestId);
      message = 'Caiven Studio linked. You can close this tab.';
      setTimeout(() => window.close(), 1200);
    } catch (error) {
      message = error instanceof ApiError ? error.message : 'Could not link Studio.';
      failed = true;
    } finally { busy = false; }
  }
</script>
<div class="container-narrow py-16"><Card.Root><Card.Header><Card.Title class="text-xl">Link Caiven Studio</Card.Title><Card.Description>Creates one local Studio token. Token never appears in this browser.</Card.Description></Card.Header><Card.Content>
  {#if !requestId}<p>Invalid link request.</p>
  {:else if !currentUser.value}<p>Sign in or register, then return here to link Studio.</p><Button onclick={login}>Log in</Button>
  {:else if message}<p class="text-sm">{message}</p>{#if failed}<div class="mt-4 flex gap-2"><Button disabled={busy} onclick={approve}>Retry</Button><Button variant="outline" onclick={() => navigate('/')}>Cancel</Button></div>{/if}
  {:else}<p>Link Studio to <strong>{currentUser.value.username}</strong>?</p><div class="mt-4 flex gap-2"><Button disabled={busy} onclick={approve}>Link Caiven Studio</Button><Button variant="outline" onclick={() => navigate('/')}>Cancel</Button></div>{/if}
</Card.Content></Card.Root></div>
