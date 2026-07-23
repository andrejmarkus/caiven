<script lang="ts">
  import { api, ApiError } from '../api';
  import { route, navigate } from '../router.svelte';
  import * as Card from '$lib/components/ui/card';
  import * as Field from '$lib/components/ui/field';
  import { Input } from '$lib/components/ui/input';
  import { Textarea } from '$lib/components/ui/textarea';
  import { Button } from '$lib/components/ui/button';
  import { Spinner } from '$lib/components/ui/spinner';
  import * as Alert from '$lib/components/ui/alert';
  import { cn } from '$lib/utils';
  import UploadCloudIcon from '@lucide/svelte/icons/upload-cloud';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';

  const cartId = $derived(route.search.get('cart') ?? '');

  let title = $state('');
  let author = $state('');
  let description = $state('');
  let tags = $state('');
  let changelog = $state('');
  let cartFile = $state<File | null>(null);
  let dragOver = $state(false);
  let error = $state('');
  let busy = $state(false);

  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const f = e.dataTransfer?.files?.[0];
    if (f) cartFile = f;
  }

  function onPick(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (f) cartFile = f;
  }

  async function submit(e: Event) {
    e.preventDefault();
    if (!cartFile) {
      error = 'Select a .cav file';
      return;
    }
    busy = true;
    error = '';
    try {
      if (cartId) {
        const cart = await api.createVersion(cartId, cartFile, changelog);
        navigate(`/cart/${cart.id}`);
      } else {
        const tagList = tags.split(',').map((s) => s.trim()).filter(Boolean);
        const cart = await api.createCart(cartFile, { title, author, description, tags: tagList });
        navigate(`/cart/${cart.id}`);
      }
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Upload failed';
    } finally {
      busy = false;
    }
  }
</script>

<div class="container-narrow py-16">
  <Card.Root>
    <Card.Header>
      <Card.Title class="text-xl">{cartId ? 'Publish new version' : 'Upload a cart'}</Card.Title>
    </Card.Header>
    <Card.Content>
      <form onsubmit={submit}>
        {#if error}
          <Alert.Root variant="destructive" class="mb-4">
            <CircleAlertIcon />
            <Alert.Description>{error}</Alert.Description>
          </Alert.Root>
        {/if}

        <div
          class={cn(
            'mb-5 flex flex-col items-center gap-2 rounded-lg border border-dashed border-border p-6 text-center transition-colors',
            dragOver && 'border-primary bg-accent/40'
          )}
          role="button"
          tabindex="0"
          ondragover={(e) => {
            e.preventDefault();
            dragOver = true;
          }}
          ondragleave={() => (dragOver = false)}
          ondrop={onDrop}
        >
          <UploadCloudIcon class="size-6 text-muted-foreground" />
          {#if cartFile}
            <p class="text-sm text-foreground">{cartFile.name} ({(cartFile.size / 1024).toFixed(1)} KB)</p>
          {:else}
            <p class="text-sm text-muted-foreground">Drag a .cav file here, or browse</p>
          {/if}
          <input type="file" accept=".cav" onchange={onPick} class="text-xs" />
        </div>

        <Field.FieldGroup>
          {#if cartId}
            <Field.Field>
              <Field.FieldLabel for="changelog">Changelog</Field.FieldLabel>
              <Textarea id="changelog" bind:value={changelog} rows={3} />
            </Field.Field>
          {:else}
            <Field.Field>
              <Field.FieldLabel for="title">Title</Field.FieldLabel>
              <Input id="title" bind:value={title} maxlength={64} required />
            </Field.Field>
            <Field.Field>
              <Field.FieldLabel for="author">Author</Field.FieldLabel>
              <Input id="author" bind:value={author} maxlength={64} required />
            </Field.Field>
            <Field.Field>
              <Field.FieldLabel for="description">Description</Field.FieldLabel>
              <Textarea id="description" bind:value={description} rows={4} maxlength={512} />
            </Field.Field>
            <Field.Field>
              <Field.FieldLabel for="tags">Tags</Field.FieldLabel>
              <Input id="tags" bind:value={tags} placeholder="platformer, retro" />
              <Field.FieldDescription>Comma separated</Field.FieldDescription>
            </Field.Field>
          {/if}

          <Button type="submit" disabled={busy}>
            {#if busy}<Spinner data-icon="inline-start" />{/if}
            Publish
          </Button>
        </Field.FieldGroup>
      </form>
    </Card.Content>
  </Card.Root>
</div>
