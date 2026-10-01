<script lang="ts">
  import type { Snippet } from 'svelte';
  import { api, type LegalInfo } from '../api';
  import { scrollToHash } from '../router.svelte';

  let { title, updated, lead, children }: { title: string; updated: string; lead: string; children: Snippet<[LegalInfo]> } = $props();

  let legal = $state<LegalInfo>({ operator_name: null, operator_address: null, contact_email: null });
  api.legal().then((l) => (legal = l)).catch(() => {});

  let body = $state<HTMLElement>();
  let toc = $state<{ id: string; label: string }[]>([]);
  let current = $state('');

  // Contents come from the page's own headings, so a new section can't be forgotten here.
  $effect(() => {
    if (!body) return;
    const headings = [...body.querySelectorAll('h2')];
    for (const h of headings) h.id ||= (h.textContent ?? '').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    toc = headings.map((h) => ({ id: h.id, label: h.textContent ?? '' }));
    const seen = new IntersectionObserver(
      (entries) => {
        for (const e of entries) if (e.isIntersecting) current = e.target.id;
      },
      { rootMargin: '0px 0px -75% 0px' },
    );
    headings.forEach((h) => seen.observe(h));
    // A fresh load of /terms#contact arrives before these ids exist.
    if (location.hash) scrollToHash(location.hash);
    return () => seen.disconnect();
  });
</script>

<div class="legal mx-auto w-full max-w-[68rem] px-4 py-10 md:px-8 md:py-14">
  <header class="max-w-[44rem]">
    <h1 class="font-display text-3xl font-bold tracking-tight md:text-4xl">{title}</h1>
    <p class="mt-2 text-sm text-muted-foreground">Last updated {updated}</p>
    <p class="mt-6 font-display text-xl leading-snug text-foreground md:text-2xl">{lead}</p>
  </header>

  <div class="mt-10 border-t border-[var(--border-subtle)] pt-8 lg:grid lg:grid-cols-[13rem_minmax(0,1fr)] lg:gap-12">
    <nav aria-label="Contents" class="mb-8 lg:mb-0">
      <details class="lg:hidden rounded-md border border-[var(--border-subtle)] px-4 py-3 text-sm">
        <summary class="cursor-pointer font-semibold">Contents</summary>
        <ol class="mt-3 space-y-2">
          {#each toc as item}<li><a href={`#${item.id}`} class="text-muted-foreground hover:text-foreground">{item.label}</a></li>{/each}
        </ol>
      </details>
      <ol class="sticky top-24 hidden space-y-1 text-sm lg:block">
        {#each toc as item}
          <li>
            <a
              href={`#${item.id}`}
              class="block border-l-2 py-1 pl-3 leading-snug transition-colors hover:text-foreground"
              class:border-primary={current === item.id}
              class:text-foreground={current === item.id}
              class:border-transparent={current !== item.id}
              class:text-muted-foreground={current !== item.id}
            >{item.label}</a>
          </li>
        {/each}
      </ol>
    </nav>

    <article bind:this={body} class="prose-legal max-w-[68ch]">
      {@render children(legal)}
    </article>
  </div>
</div>

<style>
  .prose-legal { font-size: 0.9375rem; line-height: 1.7; }
  .prose-legal :global(h2) {
    font-family: var(--font-display);
    font-size: 1.25rem;
    font-weight: 600;
    line-height: 1.3;
    margin-top: 3rem;
    scroll-margin-top: 5.5rem;
  }
  .prose-legal :global(h2:first-child) { margin-top: 0; }
  .prose-legal :global(h3) { font-weight: 600; margin-top: 1.75rem; }
  .prose-legal :global(p), .prose-legal :global(ul) { margin-top: 0.875rem; }
  .prose-legal :global(ul) { list-style: disc; padding-left: 1.25rem; }
  .prose-legal :global(li + li) { margin-top: 0.4rem; }
  .prose-legal :global(li::marker) { color: var(--muted-foreground); }
  .prose-legal :global(a) { color: var(--primary); text-decoration: underline; text-underline-offset: 3px; }
  .prose-legal :global(code) { font-family: var(--font-mono); font-size: 0.85em; }
  .legal a:focus-visible { outline: 2px solid var(--primary); outline-offset: 2px; }
  @media (prefers-reduced-motion: no-preference) { :global(html:has(.legal)) { scroll-behavior: smooth; } }
</style>
