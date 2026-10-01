<script lang="ts">
  import type { Snippet } from 'svelte';
  import NavRail from './NavRail.svelte';
  import TopBar from './TopBar.svelte';
  import MobileTabs from './MobileTabs.svelte';
  import { currentUser } from '../stores.svelte';
  import { link } from '../router.svelte';
  import { api } from '../api';
  import { Button } from '@caiven/ui/button';

  let { children }: { children: Snippet } = $props();

  let resent = $state(false);
  async function resend() {
    try {
      await api.resendVerification();
      resent = true;
    } catch {
      // best-effort; rate limiting etc. is surfaced via the disabled state
    }
  }

  const needsVerification = $derived(
    !!currentUser.value?.email && !currentUser.value.email_verified,
  );
</script>

<div class="flex min-h-screen bg-background">
  <NavRail />
  <div class="min-w-0 flex-1">
    <TopBar />
    {#if needsVerification}
      <div class="flex items-center justify-between gap-4 bg-amber-500/10 px-4 py-2 text-sm text-amber-700 dark:text-amber-300">
        <span>Confirm your email to publish, comment, or join jams.</span>
        <Button
          type="button"
          variant="link"
          size="sm"
          class="h-auto shrink-0 p-0 text-amber-700 underline-offset-2 dark:text-amber-300"
          disabled={resent}
          onclick={resend}
        >
          {resent ? 'Link sent' : 'Resend link'}
        </Button>
      </div>
    {/if}
    <main class="flex min-h-[calc(100vh-4rem)] flex-col pb-16 md:pb-0">
      <div class="flex-1">{@render children()}</div>
      <footer class="mx-auto flex w-full max-w-[92rem] flex-wrap px-6 gap-x-5 gap-y-1 py-6 text-xs text-muted-foreground">
        <a href="/terms" use:link class="hover:text-foreground">Terms</a>
        <a href="/privacy" use:link class="hover:text-foreground">Privacy</a>
        <a href="/report" use:link class="hover:text-foreground">Report content</a>
        <a href="/terms#contact" use:link class="hover:text-foreground">Contact</a>
      </footer>
    </main>
  </div>
  <MobileTabs />
</div>
