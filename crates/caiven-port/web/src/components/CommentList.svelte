<script lang="ts">
  import { api, type CommentInfo } from '../api';
  import { currentUser } from '../stores.svelte';
  import { Textarea } from '$lib/components/ui/textarea';
  import { Button } from '$lib/components/ui/button';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import * as Alert from '$lib/components/ui/alert';
  import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';

  let { cartId, ownerUsername }: { cartId: string; ownerUsername: string | null } = $props();

  let comments = $state<CommentInfo[]>([]);
  let loading = $state(true);
  let error = $state('');
  let body = $state('');
  let posting = $state(false);

  async function load() {
    loading = true;
    try {
      comments = await api.listComments(cartId);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    cartId;
    load();
  });

  async function submit() {
    if (!body.trim()) return;
    posting = true;
    error = '';
    try {
      const c = await api.addComment(cartId, body.trim());
      comments = [...comments, c];
      body = '';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      posting = false;
    }
  }

  async function remove(id: string) {
    try {
      await api.deleteComment(cartId, id);
      comments = comments.filter((c) => c.id !== id);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function canDelete(c: CommentInfo): boolean {
    const u = currentUser.value;
    if (!u) return false;
    return u.is_admin || u.username === c.author || u.username === ownerUsername;
  }
</script>

<div>
  <h2 class="mb-4 text-lg font-semibold">Comments</h2>

  {#if error}
    <Alert.Root variant="destructive" class="mb-3">
      <CircleAlertIcon />
      <Alert.Description>{error}</Alert.Description>
    </Alert.Root>
  {/if}

  {#if loading}
    <div class="flex flex-col gap-2">
      <Skeleton class="h-16 w-full" />
      <Skeleton class="h-16 w-full" />
    </div>
  {:else if comments.length === 0}
    <p class="mb-4 text-sm text-muted-foreground">No comments yet.</p>
  {:else}
    <ul class="mb-5 flex flex-col gap-2">
      {#each comments as c (c.id)}
        <li class="rounded-lg bg-secondary/50 p-3">
          <div class="flex items-center gap-2">
            <strong class="text-sm text-foreground">{c.author}</strong>
            <span class="text-xs text-muted-foreground">{c.created_at}</span>
            {#if canDelete(c)}
              <button type="button" onclick={() => remove(c.id)} class="ml-auto text-xs text-destructive hover:underline">delete</button>
            {/if}
          </div>
          <p class="mt-1.5 text-sm text-foreground/90">{c.body}</p>
        </li>
      {/each}
    </ul>
  {/if}

  {#if currentUser.value}
    <Textarea bind:value={body} rows={3} maxlength={1000} placeholder="Add a comment…" class="mb-2" />
    <Button onclick={submit} disabled={posting || !body.trim()}>Post comment</Button>
  {:else}
    <p class="text-sm text-muted-foreground"><a href="/login">Log in</a> to comment.</p>
  {/if}
</div>
