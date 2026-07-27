<script lang="ts">
  import { api, ApiError, type AuthConfigInfo } from '../api';
  import { setUser } from '../stores.svelte';
  import { navigate, link, route } from '../router.svelte';
  import * as Card from '@caiven/ui/card';
  import * as Field from '@caiven/ui/field';
  import { Input } from '@caiven/ui/input';
  import { Button } from '@caiven/ui/button';
  import { Spinner } from '@caiven/ui/spinner';
  import * as Alert from '@caiven/ui/alert';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
  import Turnstile from '../components/Turnstile.svelte';
  import OAuthButtons from '../components/OAuthButtons.svelte';
  import { passkeysSupported, getPasskey } from '../webauthn';

  let identifier = $state('');
  let password = $state('');
  let error = $state('');
  let busy = $state(false);
  let turnstileToken = $state('');
  let showTurnstile = $state(false);
  let authConfig = $state<AuthConfigInfo | null>(null);
  let pendingToken = $state('');
  let mfaCode = $state('');
  let passkeyBusy = $state(false);

  api.authConfig().then((c) => (authConfig = c)).catch(() => {});

  function goNext() {
    const next = route.search.get('next') ?? (route.path !== '/login' ? route.path : null);
    navigate(next?.startsWith('/') ? next : '/');
  }

  async function submit(e: Event) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      const outcome = await api.login(identifier, password, turnstileToken);
      if (outcome.mfa_required && outcome.pending_token) {
        pendingToken = outcome.pending_token;
      } else if (outcome.user) {
        setUser(outcome.user);
        goNext();
      }
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Login failed';
      // The backend only demands Turnstile after repeated failures; once it
      // rejects for that reason, show the widget so the retry can pass it.
      if (authConfig?.turnstile_site_key) showTurnstile = true;
    } finally {
      busy = false;
    }
  }

  async function signInWithPasskey() {
    if (!identifier.trim()) {
      error = 'Enter your username or email first.';
      return;
    }
    passkeyBusy = true;
    error = '';
    try {
      const { token, options } = await api.webauthnLoginStart(identifier);
      const credential = await getPasskey(options as { publicKey: unknown });
      const u = await api.webauthnLoginFinish(token, credential);
      setUser(u);
      goNext();
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Passkey sign-in failed';
    } finally {
      passkeyBusy = false;
    }
  }

  async function submitMfa(e: Event) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      const u = await api.loginMfa(pendingToken, mfaCode);
      setUser(u);
      goNext();
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Invalid code';
    } finally {
      busy = false;
    }
  }
</script>

<div class="container-narrow py-16">
  <Card.Root>
    <Card.Header>
      <Card.Title class="text-xl">{pendingToken ? 'Two-factor authentication' : 'Log in'}</Card.Title>
      <Card.Description>{pendingToken ? 'Enter a code from your authenticator app, or a backup code.' : 'Welcome back.'}</Card.Description>
    </Card.Header>
    <Card.Content>
      {#if error}
        <Alert.Root variant="destructive" class="mb-4">
          <CircleAlertIcon />
          <Alert.Description>{error}</Alert.Description>
        </Alert.Root>
      {/if}
      {#if pendingToken}
        <form onsubmit={submitMfa}>
          <Field.FieldGroup>
            <Field.Field>
              <Field.FieldLabel for="code">Authentication code</Field.FieldLabel>
              <Input id="code" bind:value={mfaCode} autocomplete="one-time-code" inputmode="numeric" required />
              <Field.FieldDescription>6-digit code, or one of your backup codes.</Field.FieldDescription>
            </Field.Field>
            <Button type="submit" disabled={busy}>
              {#if busy}<Spinner data-icon="inline-start" />{/if}
              Verify
            </Button>
          </Field.FieldGroup>
        </form>
      {:else}
        <form onsubmit={submit}>
          {#if authConfig}
            <OAuthButtons providers={authConfig.providers} />
          {/if}
          <Field.FieldGroup>
            <Field.Field>
              <Field.FieldLabel for="u">Username or email</Field.FieldLabel>
              <Input id="u" bind:value={identifier} autocomplete="username" required />
            </Field.Field>
            <Field.Field>
              <Field.FieldLabel for="p">Password</Field.FieldLabel>
              <Input id="p" type="password" bind:value={password} autocomplete="current-password" required />
            </Field.Field>
            {#if showTurnstile && authConfig?.turnstile_site_key}
              <Turnstile siteKey={authConfig.turnstile_site_key} onToken={(t) => (turnstileToken = t)} />
            {/if}
            <Button type="submit" disabled={busy}>
              {#if busy}<Spinner data-icon="inline-start" />{/if}
              Log in
            </Button>
            {#if passkeysSupported()}
              <Button type="button" variant="outline" disabled={passkeyBusy} onclick={signInWithPasskey}>
                {#if passkeyBusy}<Spinner data-icon="inline-start" />{/if}
                Sign in with a passkey
              </Button>
            {/if}
          </Field.FieldGroup>
        </form>
      {/if}
    </Card.Content>
  </Card.Root>
  {#if !pendingToken}
    <p class="mt-2 text-center text-sm text-muted-foreground"><a href="/forgot-password" use:link>Forgot password?</a></p>
    <p class="mt-2 text-center text-sm text-muted-foreground">No account? <a href="/register" use:link>Register</a></p>
  {/if}
</div>
