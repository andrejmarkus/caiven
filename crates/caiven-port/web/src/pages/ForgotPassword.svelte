<script lang="ts">
  import { api, ApiError, type AuthConfigInfo } from '../api';
  import { link } from '../router.svelte';
  import * as Card from '$lib/components/ui/card';
  import * as Field from '$lib/components/ui/field';
  import { Input } from '$lib/components/ui/input';
  import { Button } from '$lib/components/ui/button';
  import { Spinner } from '$lib/components/ui/spinner';
  import * as Alert from '$lib/components/ui/alert';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
  import Turnstile from '../components/Turnstile.svelte';

  let email = $state('');
  let error = $state('');
  let busy = $state(false);
  let sent = $state(false);
  let turnstileToken = $state('');
  let authConfig = $state<AuthConfigInfo | null>(null);

  api.authConfig().then((c) => (authConfig = c)).catch(() => {});

  async function submit(e: Event) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      await api.forgotPassword(email, turnstileToken);
      sent = true;
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Something went wrong';
    } finally {
      busy = false;
    }
  }
</script>

<div class="container-narrow py-16">
  <Card.Root>
    <Card.Header>
      <Card.Title class="text-xl">Reset your password</Card.Title>
      <Card.Description>We'll email you a link if the address is registered.</Card.Description>
    </Card.Header>
    <Card.Content>
      {#if sent}
        <p class="text-sm">Check your inbox for a reset link. It expires in 1 hour.</p>
      {:else}
        <form onsubmit={submit}>
          {#if error}
            <Alert.Root variant="destructive" class="mb-4">
              <CircleAlertIcon />
              <Alert.Description>{error}</Alert.Description>
            </Alert.Root>
          {/if}
          <Field.FieldGroup>
            <Field.Field>
              <Field.FieldLabel for="e">Email</Field.FieldLabel>
              <Input id="e" type="email" bind:value={email} autocomplete="email" required />
            </Field.Field>
            {#if authConfig?.turnstile_site_key}
              <Turnstile siteKey={authConfig.turnstile_site_key} onToken={(t) => (turnstileToken = t)} />
            {/if}
            <Button type="submit" disabled={busy}>
              {#if busy}<Spinner data-icon="inline-start" />{/if}
              Send reset link
            </Button>
          </Field.FieldGroup>
        </form>
      {/if}
    </Card.Content>
  </Card.Root>
  <p class="mt-4 text-center text-sm text-muted-foreground"><a href="/login" use:link>Back to log in</a></p>
</div>
