<script lang="ts">
  import { api, type Cart, type JamInfo } from '../api';
  import { currentUser } from '../stores.svelte';
  import CartCard from '../components/CartCard.svelte';
  import { Button } from '$lib/components/ui/button';
  import { navigate } from '../router.svelte';

  let { slug }: { slug: string } = $props();
  let jam = $state<JamInfo | null>(null);
  let mine = $state<Cart[]>([]);
  let selected = $state('');
  let error = $state('');
  let now = $state(Date.now());
  const remaining = $derived(jam ? Math.max(0, new Date(jam.submissions_close_at).getTime() - now) : 0);
  const days = $derived(Math.floor(remaining / 86400000));
  const hours = $derived(Math.floor((remaining % 86400000) / 3600000));
  const minutes = $derived(Math.floor((remaining % 3600000) / 60000));

  async function load() {
    try {
      jam = await api.getJam(slug);
      if (currentUser.value) mine = (await api.userProfile(currentUser.value.username, 0, 100)).carts;
    } catch (e) { error = e instanceof Error ? e.message : String(e); }
  }
  $effect(() => {
    slug; load();
    const timer = window.setInterval(() => (now = Date.now()), 30000);
    return () => clearInterval(timer);
  });
  async function enter() {
    if (!currentUser.value) { navigate(`/login?next=/jams/${slug}`); return; }
    if (selected) jam = await api.enterJam(slug, selected);
  }
  async function withdraw(cartId: string) {
    jam = await api.withdrawJam(slug, cartId);
  }
</script>

<div class="container-page py-8 md:py-10">
  {#if error}<div class="rounded-lg border border-destructive/50 p-4 text-destructive">{error}</div>{/if}
  {#if jam}
    <header class="surface-panel relative overflow-hidden rounded-xl p-7 md:p-10">
      <div class="absolute -top-36 -right-16 size-[460px] bg-[radial-gradient(ellipse_at_center,rgba(254,176,93,.14),transparent_70%)]"></div>
      <div class="relative flex flex-wrap items-center gap-8">
        <div class="min-w-0 flex-1 basis-[430px]"><div class="label-mono inline-flex rounded-full bg-accent px-3 py-1 text-[10px] text-accent-foreground">{jam.status}</div><h1 class="mt-4 text-3xl font-bold">{jam.title}</h1><p class="mt-3 max-w-2xl text-muted-foreground">{jam.description}</p>{#if jam.rules}<p class="mt-4 whitespace-pre-line text-sm text-foreground">{jam.rules}</p>{/if}<div class="mt-6 flex gap-7 font-mono text-sm"><span>{jam.entry_count} entries</span><span>{jam.creator_count} creators</span></div></div>
        {#if jam.status === 'open'}
          <div class="min-w-[280px]">
            <div class="mb-4 flex justify-center gap-2">{#each [{n:days,l:'days'},{n:hours,l:'hrs'},{n:minutes,l:'min'}] as unit}<div class="w-20 rounded-md border border-border bg-background p-3 text-center"><strong class="block font-mono text-xl text-primary">{String(unit.n).padStart(2,'0')}</strong><span class="label-mono text-[9px] text-muted-foreground">{unit.l}</span></div>{/each}</div>
            {#if currentUser.value && mine.length}
              <select bind:value={selected} class="mb-2 h-10 w-full rounded-md border border-border bg-background px-3 text-sm"><option value="">Choose your cart…</option>{#each mine.filter((c) => !jam?.carts.some((x) => x.id === c.id)) as cart}<option value={cart.id}>{cart.title}</option>{/each}</select>
              <Button class="w-full" disabled={!selected} onclick={enter}>Enter selected cart</Button>
            {:else}
              <Button class="w-full" onclick={enter}>Log in to enter</Button>
            {/if}
          </div>
        {/if}
      </div>
    </header>
    <section class="mt-8"><h2 class="mb-5 text-xl font-semibold">Entries</h2><div class="cart-grid">{#each jam.carts as cart (cart.id)}<div class="relative"><CartCard {cart} />{#if jam.status === 'open' && mine.some((owned) => owned.id === cart.id)}<button onclick={() => withdraw(cart.id)} class="absolute top-2 left-2 z-10 rounded-full bg-black/80 px-3 py-1.5 text-xs font-semibold text-white hover:bg-destructive">Withdraw</button>{/if}</div>{:else}<p class="col-span-full py-16 text-center text-muted-foreground">No entries yet.</p>{/each}</div></section>
  {/if}
</div>
