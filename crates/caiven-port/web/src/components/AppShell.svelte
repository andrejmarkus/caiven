<script lang="ts">
  import type { Snippet } from 'svelte';
  import NavRail from './NavRail.svelte';
  import TopBar from './TopBar.svelte';
  import MobileTabs from './MobileTabs.svelte';

  let { children }: { children: Snippet } = $props();
</script>

<div class="flex min-h-screen bg-background">
  <NavRail />
  <div class="min-w-0 flex-1">
    <TopBar />
    <main class="min-h-[calc(100vh-4rem)] pb-16 md:pb-0">
      {@render children()}
    </main>
  </div>
  <MobileTabs />
</div>
