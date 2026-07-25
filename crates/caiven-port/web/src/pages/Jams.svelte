<script lang="ts">
  import { api, type JamInfo } from '../api';
  import { currentUser } from '../stores.svelte';
  import { link } from '../router.svelte';
  import { Button } from '$lib/components/ui/button';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import PlusIcon from '@lucide/svelte/icons/plus';

  let jams = $state<JamInfo[]>([]);
  let error = $state('');
  let creating = $state(false);
  let title = $state('');
  let description = $state('');
  let rules = $state('');
  let startsAt = $state('');
  let closesAt = $state('');
  let endsAt = $state('');

  async function load() { try { jams = await api.listJams(); } catch (e) { error = e instanceof Error ? e.message : String(e); } }
  $effect(() => { load(); });
  async function create(e: Event) {
    e.preventDefault();
    await api.createJam({ title, description, rules, starts_at: new Date(startsAt).toISOString(), submissions_close_at: new Date(closesAt).toISOString(), ends_at: new Date(endsAt).toISOString() });
    creating = false; await load();
  }
</script>

<div class="container-page py-8 md:py-10">
  <div class="flex items-end justify-between gap-4">
    <div><h1 class="page-title">Jams</h1><p class="mt-1 text-sm text-muted-foreground">Build small, ship fast, play together.</p></div>
    {#if currentUser.value?.is_admin}<Button onclick={() => (creating = !creating)}><PlusIcon />New jam</Button>{/if}
  </div>
  {#if creating}
    <form onsubmit={create} class="surface-panel mt-6 grid max-w-3xl gap-4 rounded-lg p-5 sm:grid-cols-2">
      <input bind:value={title} required placeholder="Jam title" class="h-10 rounded-md border border-border bg-background px-3 sm:col-span-2" />
      <textarea bind:value={description} placeholder="Description" class="rounded-md border border-border bg-background p-3 sm:col-span-2"></textarea>
      <textarea bind:value={rules} placeholder="Rules" class="rounded-md border border-border bg-background p-3 sm:col-span-2"></textarea>
      <label class="text-xs text-muted-foreground">Starts<input bind:value={startsAt} type="datetime-local" required class="mt-1 block h-10 w-full rounded-md border border-border bg-background px-3 text-foreground" /></label>
      <label class="text-xs text-muted-foreground">Submissions close<input bind:value={closesAt} type="datetime-local" required class="mt-1 block h-10 w-full rounded-md border border-border bg-background px-3 text-foreground" /></label>
      <label class="text-xs text-muted-foreground">Ends<input bind:value={endsAt} type="datetime-local" required class="mt-1 block h-10 w-full rounded-md border border-border bg-background px-3 text-foreground" /></label>
      <div class="flex items-end"><Button type="submit">Create jam</Button></div>
    </form>
  {/if}
  {#if error}<div class="mt-6 rounded-lg border border-destructive/50 p-4 text-destructive">{error}</div>{/if}
  <div class="mt-7 space-y-5">
    {#each jams as jam}
      <a href="/jams/{jam.slug}" use:link class="surface-panel relative block overflow-hidden rounded-xl p-7 text-foreground hover:border-primary hover:text-foreground">
        <div class="absolute -top-28 -right-16 size-80 bg-[radial-gradient(ellipse_at_center,rgba(254,176,93,.12),transparent_70%)]"></div>
        <div class="relative flex flex-wrap items-center gap-7">
          <div class="min-w-0 flex-1 basis-[460px]"><div class="label-mono flex items-center gap-2 text-[10px] text-primary"><TrophyIcon class="size-4" />{jam.status}</div><h2 class="mt-2 text-2xl font-bold">{jam.title}</h2><p class="mt-2 text-muted-foreground">{jam.description}</p></div>
          <div class="flex gap-7 font-mono"><div><strong class="block text-xl">{jam.entry_count}</strong><span class="label-mono text-[9px] text-muted-foreground">entries</span></div><div><strong class="block text-xl">{jam.creator_count}</strong><span class="label-mono text-[9px] text-muted-foreground">creators</span></div></div>
        </div>
      </a>
    {:else}
      <div class="py-20 text-center text-muted-foreground">No jams scheduled. Check back soon.</div>
    {/each}
  </div>
</div>
