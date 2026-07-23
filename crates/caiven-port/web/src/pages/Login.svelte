<script lang="ts">
  import { api, ApiError } from '../api';
  import { setUser } from '../stores.svelte';
  import { navigate, link } from '../router.svelte';
  import * as Card from '$lib/components/ui/card';
  import * as Field from '$lib/components/ui/field';
  import { Input } from '$lib/components/ui/input';
  import { Button } from '$lib/components/ui/button';
  import { Spinner } from '$lib/components/ui/spinner';
  import * as Alert from '$lib/components/ui/alert';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';

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

<div class="container-narrow py-16">
  <Card.Root>
    <Card.Header>
      <Card.Title class="text-xl">Log in</Card.Title>
      <Card.Description>Welcome back.</Card.Description>
    </Card.Header>
    <Card.Content>
      <form onsubmit={submit}>
        {#if error}
          <Alert.Root variant="destructive" class="mb-4">
            <CircleAlertIcon />
            <Alert.Description>{error}</Alert.Description>
          </Alert.Root>
        {/if}
        <Field.FieldGroup>
          <Field.Field>
            <Field.FieldLabel for="u">Username</Field.FieldLabel>
            <Input id="u" bind:value={username} autocomplete="username" required />
          </Field.Field>
          <Field.Field>
            <Field.FieldLabel for="p">Password</Field.FieldLabel>
            <Input id="p" type="password" bind:value={password} autocomplete="current-password" required />
          </Field.Field>
          <Button type="submit" disabled={busy}>
            {#if busy}<Spinner data-icon="inline-start" />{/if}
            Log in
          </Button>
        </Field.FieldGroup>
      </form>
    </Card.Content>
  </Card.Root>
  <p class="mt-4 text-center text-sm text-muted-foreground">No account? <a href="/register" use:link>Register</a></p>
</div>
