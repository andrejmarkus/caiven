<script lang="ts">
  import { api, ApiError } from '../api';
  import { route, link, navigate } from '../router.svelte';
  import { setUser } from '../stores.svelte';
  import { pendingPublish } from '../lib/pending-publish';
  import * as Card from '@caiven/ui/card';
  import * as Alert from '@caiven/ui/alert';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';

  let status = $state<'checking' | 'ok' | 'error'>('checking');
  let error = $state('');
  // A remix waiting on this confirmation goes straight back to its Publish form.
  const remixId = pendingPublish();

  const token = route.search.get('token');

  if (!token) {
    status = 'error';
    error = 'Missing verification token.';
  } else {
    api
      .verifyEmail(token)
      .then(async () => {
        status = 'ok';
        if (!remixId) return;
        await api.me().then(setUser).catch(() => {});
        navigate(`/remix/${remixId}?publish=1`);
      })
      .catch((e) => {
        status = 'error';
        error = e instanceof ApiError ? e.message : 'Verification failed';
      });
  }
</script>

<div class="container-narrow py-16">
  <Card.Root>
    <Card.Header>
      <Card.Title class="text-xl">Confirm your email</Card.Title>
    </Card.Header>
    <Card.Content>
      {#if status === 'checking'}
        <p class="text-sm text-muted-foreground">Confirming…</p>
      {:else if status === 'ok' && remixId}
        <p class="text-sm">Your email is confirmed. Taking you back to your remix…</p>
      {:else if status === 'ok'}
        <p class="text-sm">Your email is confirmed. <a href="/" use:link>Go to Port</a>.</p>
        <p class="mt-2 text-sm text-muted-foreground">Publishing a remix? Go back to its tab and press Publish.</p>
      {:else}
        <Alert.Root variant="destructive">
          <CircleAlertIcon />
          <Alert.Description>{error}</Alert.Description>
        </Alert.Root>
        <p class="mt-4 text-sm text-muted-foreground">
          Links expire after 24 hours. You can request a new one from <a href="/settings" use:link>Settings</a>.
        </p>
      {/if}
    </Card.Content>
  </Card.Root>
</div>
