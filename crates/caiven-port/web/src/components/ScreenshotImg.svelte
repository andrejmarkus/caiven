<script lang="ts">
  import { api } from '../api';

  let { id, hasScreenshot, version, alt = '' }: {
    id: string;
    hasScreenshot: boolean;
    version?: number;
    alt?: string;
  } = $props();
</script>

{#if hasScreenshot}
  <img src={api.screenshotUrl(id, version)} {alt} loading="lazy" />
{:else}
  <div class="placeholder" aria-label={alt}>no screenshot</div>
{/if}

<style>
  img {
    width: 100%;
    aspect-ratio: 4 / 3;
    object-fit: cover;
    background: var(--bg-panel-2);
    border-radius: var(--radius);
    display: block;
  }
  .placeholder {
    width: 100%;
    aspect-ratio: 4 / 3;
    background: var(--bg-panel-2);
    border-radius: var(--radius);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-dim);
    font-size: 0.85em;
  }
</style>
