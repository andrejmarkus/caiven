<script lang="ts">
  import { api } from '../api';
  import { Button } from '@caiven/ui/button';

  let { providers }: { providers: string[] } = $props();

  const labels: Record<string, string> = {
    google: 'Google',
    github: 'GitHub',
    discord: 'Discord',
  };

  function start(provider: string) {
    window.location.href = api.oauthStartUrl(provider);
  }
</script>

{#if providers.length > 0}
  <div class="flex flex-col gap-2">
    {#each providers as provider (provider)}
      <Button type="button" variant="outline" class="w-full" onclick={() => start(provider)}>
        Continue with {labels[provider] ?? provider}
      </Button>
    {/each}
  </div>
  <div class="my-2 flex items-center gap-3 text-xs text-muted-foreground">
    <div class="h-px flex-1 bg-border"></div>
    or
    <div class="h-px flex-1 bg-border"></div>
  </div>
{/if}
