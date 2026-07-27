<script lang="ts">
  import type { Snippet } from 'svelte';
  import NavRail from './NavRail.svelte';
  import TopBar from './TopBar.svelte';
  import MobileTabs from './MobileTabs.svelte';
  import { currentUser } from '../stores.svelte';
  import { api } from '../api';

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
        <button
          type="button"
          class="shrink-0 underline underline-offset-2 disabled:opacity-60"
          disabled={resent}
          onclick={resend}
        >
          {resent ? 'Link sent' : 'Resend link'}
        </button>
      </div>
    {/if}
    <main class="min-h-[calc(100vh-4rem)] pb-16 md:pb-0">
      {@render children()}
    </main>
  </div>
  <MobileTabs />
</div>
