<script lang="ts">
  import { api, ApiError, type Cart, type TokenInfo } from '../api';
  import { currentUser } from '../stores.svelte';
  import { link } from '../router.svelte';
  import * as Card from '$lib/components/ui/card';
  import * as Field from '$lib/components/ui/field';
  import { Input } from '$lib/components/ui/input';
  import { Textarea } from '$lib/components/ui/textarea';
  import { Button, buttonVariants } from '$lib/components/ui/button';
  import * as AlertDialog from '$lib/components/ui/alert-dialog';
  import * as Alert from '$lib/components/ui/alert';
  import * as Empty from '$lib/components/ui/empty';
  import { toast } from 'svelte-sonner';
  import KeyRoundIcon from '@lucide/svelte/icons/key-round';
  import PackageIcon from '@lucide/svelte/icons/package';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';

  let tokens = $state<TokenInfo[]>([]);
  let carts = $state<Cart[]>([]);
  let newTokenName = $state('');
  let justCreated = $state<{ name: string; token: string } | null>(null);
  let error = $state('');
  let editingId = $state<string | null>(null);
  let editTitle = $state('');
  let editDescription = $state('');
  let editTags = $state('');

  async function load() {
    const u = currentUser.value;
    if (!u) return;
    try {
      const [t, p] = await Promise.all([api.listTokens(), api.userProfile(u.username, 0, 100)]);
      tokens = t;
      carts = p.carts;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  $effect(() => {
    load();
  });

  async function createToken(e: Event) {
    e.preventDefault();
    try {
      const t = await api.createToken(newTokenName || 'token');
      justCreated = { name: t.name, token: t.token };
      newTokenName = '';
      await load();
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Failed to create token';
    }
  }

  function copyToken() {
    if (!justCreated) return;
    navigator.clipboard.writeText(justCreated.token);
    toast.success('Token copied to clipboard');
  }

  async function revoke(id: string) {
    try {
      await api.revokeToken(id);
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function startEdit(cart: Cart) {
    editingId = cart.id;
    editTitle = cart.title;
    editDescription = cart.description;
    editTags = cart.tags.join(', ');
  }

  async function saveEdit() {
    if (!editingId) return;
    try {
      await api.updateCart(editingId, {
        title: editTitle,
        description: editDescription,
        tags: editTags.split(',').map((s) => s.trim()).filter(Boolean),
      });
      editingId = null;
      await load();
      toast.success('Cart updated');
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function removeCart(id: string) {
    try {
      await api.deleteCart(id);
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="container-page py-10">
  <h1 class="mb-6 text-2xl font-semibold">Profile</h1>

  {#if error}
    <Alert.Root variant="destructive" class="mb-6">
      <CircleAlertIcon />
      <Alert.Description>{error}</Alert.Description>
    </Alert.Root>
  {/if}

  <Card.Root class="mb-6">
    <Card.Header>
      <Card.Title class="flex items-center gap-2 text-base"><KeyRoundIcon class="size-4 text-primary" /> API tokens</Card.Title>
      <Card.Description>Use a token with the <code class="text-xs">X-Api-Key</code> header from the CLI or Studio.</Card.Description>
    </Card.Header>
    <Card.Content>
      {#if justCreated}
        <div class="mb-4 flex items-center justify-between gap-3 rounded-lg border border-primary/40 bg-accent/40 p-3">
          <div class="min-w-0">
            <p class="text-sm text-foreground">Token <strong>{justCreated.name}</strong> created — copy it now, it won't be shown again:</p>
            <code class="mt-1 block truncate text-xs text-foreground">{justCreated.token}</code>
          </div>
          <Button size="icon" variant="secondary" onclick={copyToken} aria-label="Copy token" class="shrink-0">
            <CopyIcon />
          </Button>
        </div>
      {/if}

      <ul class="mb-4 flex flex-col divide-y divide-border text-sm">
        {#each tokens as t (t.id)}
          <li class="flex flex-wrap items-center gap-3 py-2.5">
            <strong class="text-foreground">{t.name}</strong>
            <span class="text-muted-foreground">created {t.created_at}</span>
            <span class="text-muted-foreground">{t.last_used_at ? `last used ${t.last_used_at}` : 'never used'}</span>
            <button type="button" onclick={() => revoke(t.id)} class="ml-auto text-destructive hover:underline">revoke</button>
          </li>
        {:else}
          <li class="py-2.5 text-muted-foreground">No tokens yet.</li>
        {/each}
      </ul>
      <form onsubmit={createToken} class="flex gap-2">
        <Input bind:value={newTokenName} placeholder="Token name (e.g. Studio)" />
        <Button type="submit" class="shrink-0">Create</Button>
      </form>
    </Card.Content>
  </Card.Root>

  <Card.Root>
    <Card.Header>
      <Card.Title class="flex items-center gap-2 text-base"><PackageIcon class="size-4 text-primary" /> My carts</Card.Title>
    </Card.Header>
    <Card.Content>
      {#if carts.length === 0}
        <Empty.Root>
          <Empty.Header>
            <Empty.Media variant="icon"><PackageIcon /></Empty.Media>
            <Empty.Title>No carts published</Empty.Title>
            <Empty.Description>Head to Upload to publish your first cart.</Empty.Description>
          </Empty.Header>
        </Empty.Root>
      {:else}
        <div class="flex flex-col divide-y divide-border">
          {#each carts as cart (cart.id)}
            <div class="py-4 first:pt-0 last:pb-0">
              {#if editingId === cart.id}
                <Field.FieldGroup>
                  <Field.Field>
                    <Field.FieldLabel for="et">Title</Field.FieldLabel>
                    <Input id="et" bind:value={editTitle} maxlength={64} />
                  </Field.Field>
                  <Field.Field>
                    <Field.FieldLabel for="ed">Description</Field.FieldLabel>
                    <Textarea id="ed" bind:value={editDescription} rows={3} maxlength={512} />
                  </Field.Field>
                  <Field.Field>
                    <Field.FieldLabel for="etg">Tags</Field.FieldLabel>
                    <Input id="etg" bind:value={editTags} />
                  </Field.Field>
                  <div class="flex gap-2">
                    <Button size="sm" onclick={saveEdit}>Save</Button>
                    <Button size="sm" variant="secondary" onclick={() => (editingId = null)}>Cancel</Button>
                  </div>
                </Field.FieldGroup>
              {:else}
                <div class="flex flex-wrap items-center gap-3">
                  <a href="/cart/{cart.id}" use:link class="font-medium text-foreground hover:text-primary">{cart.title}</a>
                  <span class="text-xs text-muted-foreground">v{cart.latest_version} · {cart.downloads} dl</span>
                  <div class="ml-auto flex gap-2">
                    <Button size="icon" variant="secondary" onclick={() => startEdit(cart)} aria-label="Edit {cart.title}">
                      <PencilIcon />
                    </Button>
                    <AlertDialog.Root>
                      <AlertDialog.Trigger class={buttonVariants({ variant: 'destructive', size: 'icon' })} aria-label="Delete {cart.title}">
                        <Trash2Icon />
                      </AlertDialog.Trigger>
                      <AlertDialog.Content>
                        <AlertDialog.Header>
                          <AlertDialog.Title>Delete "{cart.title}"?</AlertDialog.Title>
                          <AlertDialog.Description>This removes the cart and every published version. This cannot be undone.</AlertDialog.Description>
                        </AlertDialog.Header>
                        <AlertDialog.Footer>
                          <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
                          <AlertDialog.Action variant="destructive" onclick={() => removeCart(cart.id)}>Delete</AlertDialog.Action>
                        </AlertDialog.Footer>
                      </AlertDialog.Content>
                    </AlertDialog.Root>
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </Card.Content>
  </Card.Root>
</div>
