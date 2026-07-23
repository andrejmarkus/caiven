<script lang="ts">
  import { api } from '../api';
  import Gamepad2Icon from '@lucide/svelte/icons/gamepad-2';

  let { id, hasScreenshot, version, alt = '' }: {
    id: string;
    hasScreenshot: boolean;
    version?: number;
    alt?: string;
  } = $props();
</script>

{#if hasScreenshot}
  <img class="size-full bg-secondary object-cover" src={api.screenshotUrl(id, version)} {alt} loading="lazy" />
{:else}
  <div class="flex size-full items-center justify-center bg-secondary" aria-label={alt}>
    <Gamepad2Icon class="size-8 text-muted-foreground" />
  </div>
{/if}
