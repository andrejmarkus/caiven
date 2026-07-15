<script lang="ts">
  import { api, ApiError } from '../api';
  import { route, navigate } from '../router.svelte';

  const cartId = $derived(route.search.get('cart') ?? '');

  let title = $state('');
  let author = $state('');
  let description = $state('');
  let tags = $state('');
  let changelog = $state('');
  let rom = $state<File | null>(null);
  let dragOver = $state(false);
  let error = $state('');
  let busy = $state(false);

  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const f = e.dataTransfer?.files?.[0];
    if (f) rom = f;
  }

  function onPick(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (f) rom = f;
  }

  async function submit(e: Event) {
    e.preventDefault();
    if (!rom) {
      error = 'Select a .rom file';
      return;
    }
    busy = true;
    error = '';
    try {
      if (cartId) {
        const cart = await api.createVersion(cartId, rom, changelog);
        navigate(`/cart/${cart.id}`);
      } else {
        const tagList = tags.split(',').map((s) => s.trim()).filter(Boolean);
        const cart = await api.createCart(rom, { title, author, description, tags: tagList });
        navigate(`/cart/${cart.id}`);
      }
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Upload failed';
    } finally {
      busy = false;
    }
  }
</script>

<div class="container narrow">
  <h1>{cartId ? 'Publish new version' : 'Upload a cart'}</h1>
  <form class="panel" onsubmit={submit}>
    {#if error}<p class="error">{error}</p>{/if}

    <div
      class="dropzone"
      class:dragover={dragOver}
      role="button"
      tabindex="0"
      ondragover={(e) => { e.preventDefault(); dragOver = true; }}
      ondragleave={() => (dragOver = false)}
      ondrop={onDrop}
    >
      {#if rom}
        <p>{rom.name} ({(rom.size / 1024).toFixed(1)} KB)</p>
      {:else}
        <p class="muted">Drag a .rom file here, or</p>
      {/if}
      <input type="file" accept=".rom" onchange={onPick} />
    </div>

    {#if cartId}
      <div class="field">
        <label for="changelog">Changelog</label>
        <textarea id="changelog" bind:value={changelog} rows="3"></textarea>
      </div>
    {:else}
      <div class="field">
        <label for="title">Title</label>
        <input id="title" bind:value={title} maxlength="64" required />
      </div>
      <div class="field">
        <label for="author">Author</label>
        <input id="author" bind:value={author} maxlength="64" required />
      </div>
      <div class="field">
        <label for="description">Description</label>
        <textarea id="description" bind:value={description} rows="4" maxlength="512"></textarea>
      </div>
      <div class="field">
        <label for="tags">Tags (comma separated)</label>
        <input id="tags" bind:value={tags} placeholder="platformer, retro" />
      </div>
    {/if}

    <button type="submit" disabled={busy}>Publish</button>
  </form>
</div>

<style>
  .narrow {
    max-width: 480px;
  }
  .dropzone {
    border: 2px dashed var(--border);
    border-radius: var(--radius);
    padding: 1.5rem;
    text-align: center;
    margin-bottom: 1rem;
  }
  .dropzone.dragover {
    border-color: var(--accent);
  }
</style>
