<script lang="ts">
  import { api, ApiError } from '../api';
  import { navigate, link, route } from '../router.svelte';
  import * as Card from '@caiven/ui/card';
  import * as Field from '@caiven/ui/field';
  import { Input } from '@caiven/ui/input';
  import { Button } from '@caiven/ui/button';
  import { Spinner } from '@caiven/ui/spinner';
  import * as Alert from '@caiven/ui/alert';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';

  const token = route.search.get('token');

  let newPassword = $state('');
  let error = $state('');
  let busy = $state(false);

  async function submit(e: Event) {
    e.preventDefault();
    if (!token) {
      error = 'Missing reset token.';
      return;
    }
    busy = true;
    error = '';
    try {
      await api.resetPassword(token, newPassword);
      navigate('/login');
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Reset failed';
    } finally {
      busy = false;
    }
  }
</script>

<div class="container-narrow py-16">
  <Card.Root>
    <Card.Header>
      <Card.Title class="text-xl">Choose a new password</Card.Title>
      <Card.Description>This will sign you out everywhere else.</Card.Description>
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
            <Field.FieldLabel for="p">New password</Field.FieldLabel>
            <Input id="p" type="password" bind:value={newPassword} autocomplete="new-password" minlength={8} maxlength={128} required />
            <Field.FieldDescription>At least 8 characters, with an uppercase letter and a special character.</Field.FieldDescription>
          </Field.Field>
          <Button type="submit" disabled={busy}>
            {#if busy}<Spinner data-icon="inline-start" />{/if}
            Reset password
          </Button>
        </Field.FieldGroup>
      </form>
    </Card.Content>
  </Card.Root>
  <p class="mt-4 text-center text-sm text-muted-foreground"><a href="/login" use:link>Back to log in</a></p>
</div>
