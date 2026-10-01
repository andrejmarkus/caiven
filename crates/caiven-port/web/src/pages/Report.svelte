<script lang="ts">
  import { api, ApiError, type AuthConfigInfo } from '../api';
  import { route, link } from '../router.svelte';
  import * as Field from '@caiven/ui/field';
  import { Input } from '@caiven/ui/input';
  import { Button } from '@caiven/ui/button';
  import { Spinner } from '@caiven/ui/spinner';
  import Turnstile from '../components/Turnstile.svelte';

  const categories = [
    ['copyright', 'Copyright or trademark infringement'],
    ['illegal', 'Other illegal content'],
    ['harassment', 'Harassment, hate, or threats'],
    ['child_safety', 'Child safety'],
    ['other', 'Breaks the Terms in another way'],
  ];

  let url = $state(route.search.get('url') ?? '');
  let category = $state(route.search.get('category') ?? 'illegal');
  let explanation = $state('');
  let name = $state('');
  let email = $state('');
  let goodFaith = $state(false);
  let turnstileToken = $state('');
  let authConfig = $state<AuthConfigInfo | null>(null);
  let busy = $state(false);
  let error = $state('');
  let sent = $state(false);

  api.authConfig().then((c) => (authConfig = c)).catch(() => {});

  async function submit(e: Event) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      await api.report({ url, category, explanation, name, email, good_faith: goodFaith, turnstile_token: turnstileToken });
      sent = true;
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Could not send the report';
    } finally {
      busy = false;
    }
  }
</script>

<div class="container-narrow py-10 md:py-14">
  <h1 class="page-title">Report content</h1>
  {#if sent}
    <p class="mt-4 text-sm">Thanks. Your report reached us, and a person will review it. {email ? "We've emailed you a confirmation and will tell you what we decide." : ''}</p>
  {:else}
    <p class="mt-1 mb-6 text-sm text-muted-foreground">
      Tell us about a cart, comment, or profile that you believe is illegal or breaks the <a href="/terms#content-rules" use:link class="underline">Terms</a>.
      A person reviews every report.
    </p>
    <form onsubmit={submit}>
      {#if error}<p class="mb-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</p>{/if}
      <Field.FieldGroup>
        <Field.Field>
          <Field.FieldLabel for="r-url">Link to the content</Field.FieldLabel>
          <Input id="r-url" bind:value={url} maxlength={500} required placeholder="https://…/cart/…" />
        </Field.Field>
        <Field.Field>
          <Field.FieldLabel for="r-cat">What's wrong</Field.FieldLabel>
          <select id="r-cat" bind:value={category} class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm">
            {#each categories as [value, label]}<option {value}>{label}</option>{/each}
          </select>
        </Field.Field>
        <Field.Field>
          <Field.FieldLabel for="r-why">Explain why</Field.FieldLabel>
          <textarea id="r-why" bind:value={explanation} minlength={10} maxlength={5000} required rows="6" class="w-full rounded-md border border-border bg-background p-3 text-sm"></textarea>
          {#if category === 'copyright'}
            <Field.FieldDescription>Name the original work and say whether you own it or act for the owner. Include your postal address for a DMCA notice.</Field.FieldDescription>
          {/if}
        </Field.Field>
        <Field.Field>
          <Field.FieldLabel for="r-name">Your name {category === 'copyright' ? '(your signature)' : '(optional)'}</Field.FieldLabel>
          <Input id="r-name" bind:value={name} maxlength={100} required={category === 'copyright'} autocomplete="name" />
        </Field.Field>
        <Field.Field>
          <Field.FieldLabel for="r-email">Your email {category === 'copyright' ? '' : '(optional)'}</Field.FieldLabel>
          <Input id="r-email" type="email" bind:value={email} maxlength={254} required={category === 'copyright'} autocomplete="email" />
          <Field.FieldDescription>Used only to confirm receipt and tell you our decision. Leave both blank to report child abuse material anonymously.</Field.FieldDescription>
        </Field.Field>
        <label class="flex items-start gap-2 text-sm">
          <input type="checkbox" bind:checked={goodFaith} required class="mt-1" />
          <span>
            I believe in good faith that this report is accurate and complete.
            {#if category === 'copyright'}Under penalty of perjury, I am the copyright owner or authorized to act for them, and the use is not authorized by the owner, its agent, or the law.{/if}
          </span>
        </label>
        {#if authConfig?.turnstile_site_key}
          <Turnstile siteKey={authConfig.turnstile_site_key} onToken={(t) => (turnstileToken = t)} />
        {/if}
        <Button type="submit" disabled={busy}>
          {#if busy}<Spinner data-icon="inline-start" />{/if}
          Send report
        </Button>
      </Field.FieldGroup>
    </form>
  {/if}
</div>
