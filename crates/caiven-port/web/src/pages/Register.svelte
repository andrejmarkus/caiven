<script lang="ts">
  import { api, ApiError, type AuthConfigInfo } from '../api';
  import { setUser } from '../stores.svelte';
  import { navigate, link } from '../router.svelte';
  import * as Card from '$lib/components/ui/card';
  import * as Field from '$lib/components/ui/field';
  import { Input } from '$lib/components/ui/input';
  import { Button } from '$lib/components/ui/button';
  import { Spinner } from '$lib/components/ui/spinner';
  import * as Alert from '$lib/components/ui/alert';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
  import Turnstile from '../components/Turnstile.svelte';
  import OAuthButtons from '../components/OAuthButtons.svelte';

  let username = $state('');
  let email = $state('');
  let password = $state('');
  let error = $state('');
  let busy = $state(false);
  let turnstileToken = $state('');
  let authConfig = $state<AuthConfigInfo | null>(null);

  api.authConfig().then((c) => (authConfig = c)).catch(() => {});

  async function submit(e: Event) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      const u = await api.register(username, email, password, turnstileToken);
      setUser(u);
      navigate('/');
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Registration failed';
    } finally {
      busy = false;
    }
  }
</script>

<div class="container-narrow py-16">
  <Card.Root>
    <Card.Header>
      <Card.Title class="text-xl">Create an account</Card.Title>
      <Card.Description>Publish and rate carts.</Card.Description>
    </Card.Header>
    <Card.Content>
      <form onsubmit={submit}>
        {#if error}
          <Alert.Root variant="destructive" class="mb-4">
            <CircleAlertIcon />
            <Alert.Description>{error}</Alert.Description>
          </Alert.Root>
        {/if}
        {#if authConfig}
          <OAuthButtons providers={authConfig.providers} />
        {/if}
        <Field.FieldGroup>
          <Field.Field>
            <Field.FieldLabel for="u">Username</Field.FieldLabel>
            <Input id="u" bind:value={username} autocomplete="username" minlength={3} maxlength={32} required />
            <Field.FieldDescription>3-32 chars, lowercase letters/digits/_/-</Field.FieldDescription>
          </Field.Field>
          <Field.Field>
            <Field.FieldLabel for="e">Email</Field.FieldLabel>
            <Input id="e" type="email" bind:value={email} autocomplete="email" required />
            <Field.FieldDescription>We'll send a confirmation link.</Field.FieldDescription>
          </Field.Field>
          <Field.Field>
            <Field.FieldLabel for="p">Password</Field.FieldLabel>
            <Input id="p" type="password" bind:value={password} autocomplete="new-password" minlength={8} maxlength={128} required />
            <Field.FieldDescription>At least 8 characters, with an uppercase letter and a special character.</Field.FieldDescription>
          </Field.Field>
          {#if authConfig?.turnstile_site_key}
            <Turnstile siteKey={authConfig.turnstile_site_key} onToken={(t) => (turnstileToken = t)} />
          {/if}
          <Button type="submit" disabled={busy}>
            {#if busy}<Spinner data-icon="inline-start" />{/if}
            Create account
          </Button>
        </Field.FieldGroup>
      </form>
    </Card.Content>
  </Card.Root>
  <p class="mt-4 text-center text-sm text-muted-foreground">Have an account? <a href="/login" use:link>Log in</a></p>
</div>
