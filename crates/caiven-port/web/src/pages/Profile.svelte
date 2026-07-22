<script lang="ts">
  import { api, ApiError, type Cart, type TokenInfo } from '../api';
  import { currentUser } from '../stores.svelte';
  import { link } from '../router.svelte';

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
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function removeCart(id: string, title: string) {
    if (!confirm(`Delete "${title}"? This cannot be undone.`)) return;
    try {
      await api.deleteCart(id);
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="container">
  {#if error}<p class="error">{error}</p>{/if}

  <h1>Profile</h1>

  <section class="panel">
    <h2>API tokens</h2>
    <p class="muted">Use a token with the <code>X-Api-Key</code> header from the CLI or Studio.</p>
    {#if justCreated}
      <p class="panel token-reveal">
        Token <strong>{justCreated.name}</strong> created — copy it now, it won't be shown again:<br />
        <code>{justCreated.token}</code>
      </p>
    {/if}
    <ul>
      {#each tokens as t (t.id)}
        <li class="row">
          <strong>{t.name}</strong>
          <span class="muted">created {t.created_at}</span>
          <span class="muted">{t.last_used_at ? `last used ${t.last_used_at}` : 'never used'}</span>
          <button class="secondary danger-link" onclick={() => revoke(t.id)}>revoke</button>
        </li>
      {:else}
        <li class="muted">No tokens yet.</li>
      {/each}
    </ul>
    <form class="row" onsubmit={createToken}>
      <input bind:value={newTokenName} placeholder="Token name (e.g. Studio)" />
      <button type="submit">Create token</button>
    </form>
  </section>

  <section class="panel">
    <h2>My carts</h2>
    {#each carts as cart (cart.id)}
      <div class="cart-row">
        {#if editingId === cart.id}
          <div class="field">
            <label for="et">Title</label>
            <input id="et" bind:value={editTitle} maxlength="64" />
          </div>
          <div class="field">
            <label for="ed">Description</label>
            <textarea id="ed" bind:value={editDescription} rows="3" maxlength="512"></textarea>
          </div>
          <div class="field">
            <label for="etg">Tags</label>
            <input id="etg" bind:value={editTags} />
          </div>
          <div class="row">
            <button onclick={saveEdit}>Save</button>
            <button class="secondary" onclick={() => (editingId = null)}>Cancel</button>
          </div>
        {:else}
          <div class="row">
            <a href="/cart/{cart.id}" use:link><strong>{cart.title}</strong></a>
            <span class="muted">v{cart.latest_version} · {cart.downloads} dl</span>
            <button class="secondary" onclick={() => startEdit(cart)}>Edit</button>
            <button class="danger" onclick={() => removeCart(cart.id, cart.title)}>Delete</button>
          </div>
        {/if}
      </div>
    {:else}
      <p class="muted">You haven't published any carts yet.</p>
    {/each}
  </section>
</div>

<style>
  section {
    margin-bottom: 1.5rem;
  }
  ul {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin: 0.5rem 0 1rem;
  }
  .token-reveal {
    word-break: break-all;
    margin: 0.75rem 0;
  }
  .cart-row {
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border);
  }
  .cart-row:last-child {
    border-bottom: none;
  }
  .danger-link {
    margin-left: auto;
  }
</style>
