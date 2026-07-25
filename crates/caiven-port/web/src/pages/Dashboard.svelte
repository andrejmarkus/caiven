<script lang="ts">
  import { api, type Cart, type DashboardInfo } from '../api';
  import { link } from '../router.svelte';
  import { Button, buttonVariants } from '$lib/components/ui/button';
  import * as AlertDialog from '$lib/components/ui/alert-dialog';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import UploadIcon from '@lucide/svelte/icons/upload';
  import TrashIcon from '@lucide/svelte/icons/trash-2';

  let data = $state<DashboardInfo | null>(null);
  let error = $state('');
  let editing = $state<Cart | null>(null);
  let title = $state('');
  let description = $state('');
  let tags = $state('');
  const peak = $derived(Math.max(1, ...(data?.daily ?? []).map((day) => day.plays)));

  async function load() { try { data = await api.dashboard(); } catch (e) { error = e instanceof Error ? e.message : String(e); } }
  $effect(() => { load(); });
  function start(cart: Cart) { editing = cart; title = cart.title; description = cart.description; tags = cart.tags.join(', '); }
  async function save() {
    if (!editing) return;
    await api.updateCart(editing.id, { title, description, tags: tags.split(',').map((x) => x.trim()).filter(Boolean) });
    editing = null; await load();
  }
  async function remove(id: string) { await api.deleteCart(id); await load(); }
  function delta(current: number, previous: number) {
    if (!previous) return current ? 'new this period' : 'no change';
    const pct = Math.round(((current - previous) / previous) * 100);
    return `${pct >= 0 ? '+' : ''}${pct}% vs previous`;
  }
</script>

<div class="container-page py-8 md:py-10">
  <h1 class="page-title">Creator stats</h1><p class="mt-1 mb-7 text-sm text-muted-foreground">How your public carts performed over last 30 days.</p>
  {#if error}<div class="mb-6 rounded-lg border border-destructive/50 p-4 text-destructive">{error}</div>{/if}
  {#if data}
    <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
      {#each [
        {label:'plays',value:data.plays.current,sub:delta(data.plays.current,data.plays.previous)},
        {label:'unique players',value:data.unique_players.current,sub:delta(data.unique_players.current,data.unique_players.previous)},
        {label:'average rating',value:data.rating_avg.toFixed(1),sub:'lifetime weighted average'},
        {label:'followers',value:data.followers,sub:`+${data.new_followers} this period`}
      ] as stat}
        <div class="surface-panel rounded-lg p-5"><div class="label-mono text-[10px] text-muted-foreground">{stat.label}</div><strong class="mt-2 block font-display text-3xl">{stat.value}</strong><span class="mt-1 block font-mono text-xs text-primary">{stat.sub}</span></div>
      {/each}
    </div>
    <div class="mt-5 flex flex-wrap items-start gap-5">
      <section class="surface-panel min-w-0 flex-1 basis-[440px] rounded-lg p-5">
        <div class="mb-5 flex items-baseline justify-between"><h2 class="font-semibold">Plays per day</h2><span class="font-mono text-xs text-muted-foreground">30 days</span></div>
        <div class="flex h-44 items-end gap-1">
          {#each data.daily as day}
            <div class="group relative flex h-full flex-1 items-end"><div class="w-full rounded-t-sm bg-border group-hover:bg-primary" style:height={`${Math.max(2, (day.plays / peak) * 100)}%`}></div><span class="absolute -top-7 left-1/2 hidden -translate-x-1/2 rounded bg-black px-1.5 py-1 text-[9px] group-hover:block">{day.plays}</span></div>
          {/each}
        </div>
      </section>
      <section class="surface-panel min-w-0 flex-1 basis-[360px] overflow-hidden rounded-lg">
        <h2 class="p-5 font-semibold">Your carts</h2>
        {#each data.carts as cart}
          <div class="flex items-center gap-3 border-t border-[var(--border-subtle)] px-5 py-3">
            <a href="/cart/{cart.id}" use:link class="min-w-0 flex-1 text-foreground"><strong class="block truncate text-sm">{cart.title}</strong><span class="font-mono text-[10px] text-muted-foreground">v{cart.latest_version} · {cart.rating_avg.toFixed(1)}★</span></a>
            <span class="font-mono text-xs text-muted-foreground">{cart.plays} plays</span>
            <Button size="icon" variant="ghost" onclick={() => start(cart)} aria-label="Edit"><PencilIcon /></Button>
            <a href="/upload?cart={cart.id}" use:link class={buttonVariants({ variant: 'ghost', size: 'icon' })} aria-label="New version"><UploadIcon /></a>
            <AlertDialog.Root><AlertDialog.Trigger class={buttonVariants({ variant: 'ghost', size: 'icon' })}><TrashIcon /></AlertDialog.Trigger><AlertDialog.Content><AlertDialog.Header><AlertDialog.Title>Delete “{cart.title}”?</AlertDialog.Title><AlertDialog.Description>Removes every version. Cannot be undone.</AlertDialog.Description></AlertDialog.Header><AlertDialog.Footer><AlertDialog.Cancel>Cancel</AlertDialog.Cancel><AlertDialog.Action variant="destructive" onclick={() => remove(cart.id)}>Delete</AlertDialog.Action></AlertDialog.Footer></AlertDialog.Content></AlertDialog.Root>
          </div>
        {:else}<p class="border-t border-border p-5 text-sm text-muted-foreground">No carts published.</p>{/each}
      </section>
    </div>
  {/if}
  {#if editing}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4">
      <form onsubmit={(e) => { e.preventDefault(); save(); }} class="surface-panel w-full max-w-lg space-y-4 rounded-xl p-6">
        <h2 class="text-xl font-semibold">Edit cart</h2>
        <input bind:value={title} maxlength={64} required class="h-10 w-full rounded-md border border-border bg-background px-3" />
        <textarea bind:value={description} maxlength={512} rows={4} class="w-full rounded-md border border-border bg-background p-3"></textarea>
        <input bind:value={tags} class="h-10 w-full rounded-md border border-border bg-background px-3" />
        <div class="flex gap-2"><Button type="submit">Save</Button><Button type="button" variant="ghost" onclick={() => (editing = null)}>Cancel</Button></div>
      </form>
    </div>
  {/if}
</div>
