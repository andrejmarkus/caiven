<script lang="ts">
  import { api, type CommentInfo } from '../api';
  import { currentUser } from '../stores.svelte';

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

<div class="comments">
  <h3>Comments</h3>
  {#if error}<p class="error">{error}</p>{/if}
  {#if loading}
    <p class="muted">loading…</p>
  {:else if comments.length === 0}
    <p class="muted">No comments yet.</p>
  {:else}
    <ul>
      {#each comments as c (c.id)}
        <li class="panel comment">
          <div class="row">
            <strong>{c.author}</strong>
            <span class="muted">{c.created_at}</span>
            {#if canDelete(c)}
              <button class="secondary danger-link" onclick={() => remove(c.id)}>delete</button>
            {/if}
          </div>
          <p>{c.body}</p>
        </li>
      {/each}
    </ul>
  {/if}

  {#if currentUser.value}
    <div class="field">
      <textarea bind:value={body} rows="3" maxlength="1000" placeholder="Add a comment…"></textarea>
    </div>
    <button onclick={submit} disabled={posting || !body.trim()}>Post comment</button>
  {:else}
    <p class="muted"><a href="/login">Log in</a> to comment.</p>
  {/if}
</div>

<style>
  ul {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    margin: 0.75rem 0;
  }
  .comment p {
    margin: 0.4em 0 0;
  }
  .danger-link {
    margin-left: auto;
    color: var(--danger);
    background: transparent;
    border: none;
    padding: 0;
    font-size: 0.85em;
  }
  .danger-link:hover {
    background: transparent;
    text-decoration: underline;
  }
</style>
