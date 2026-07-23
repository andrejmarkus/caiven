<script lang="ts">
  import { link } from '../router.svelte';
  import Logo from '$lib/components/Logo.svelte';
</script>

<footer class="mt-16 border-t border-border bg-card/40">
  <div class="container-page flex flex-col items-center gap-4 py-8 sm:flex-row sm:justify-between">
    <a href="/" use:link class="flex items-center gap-2">
      <Logo size={22} />
      <span class="font-display text-sm font-semibold text-foreground">Caiven Port</span>
    </a>
    <nav class="flex items-center gap-5 text-sm text-muted-foreground">
      <a href="/browse" use:link class="hover:text-foreground">Browse</a>
      <a href="/upload" use:link class="hover:text-foreground">Upload</a>
    </nav>
  </div>
  <div class="border-t border-border/60">
    <div class="container-page py-3">
      <p class="label-mono text-center text-[10px] text-muted-foreground/70 sm:text-left">
        Caiven &middot; cart library for the fantasy console
      </p>
    </div>
  </div>
</footer>
