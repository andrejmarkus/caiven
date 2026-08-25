<script lang="ts">
  import { api, type AdminUserInfo } from '../../api';
  import { currentUser } from '../../stores.svelte';
  import { Button } from '@caiven/ui/button';
  import { Badge } from '@caiven/ui/badge';

  let users = $state<AdminUserInfo[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let error = $state('');
  let q = $state('');
  let filter = $state<'' | 'admin' | 'banned'>('');
  let banningId = $state<string | null>(null);
  let banReason = $state('');
  let busyId = $state<string | null>(null);

  async function load() {
    loading = true;
    error = '';
    try {
      const result = await api.adminListUsers({ q: q || undefined, filter: filter || undefined, per_page: 50 });
      users = result.users;
      total = result.total;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }
  $effect(() => { load(); });

  async function withBusy(id: string, fn: () => Promise<AdminUserInfo>) {
    busyId = id;
    error = '';
    try {
      const updated = await fn();
      users = users.map((u) => (u.id === id ? updated : u));
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busyId = null;
    }
  }

  function startBan(id: string) {
    banningId = id;
    banReason = '';
  }

  async function confirmBan(id: string) {
    if (!banReason.trim()) return;
    await withBusy(id, () => api.adminBanUser(id, banReason.trim()));
    banningId = null;
  }
</script>

<div class="flex flex-wrap items-center gap-2">
  <input
    bind:value={q}
    onkeydown={(e) => e.key === 'Enter' && load()}
    placeholder="Search username or email…"
    class="h-9 w-64 rounded-md border border-border bg-background px-3 text-sm"
  />
  <select bind:value={filter} onchange={load} class="h-9 rounded-md border border-border bg-background px-2 text-sm">
    <option value="">All users</option>
    <option value="admin">Admins</option>
    <option value="banned">Banned</option>
  </select>
  <Button variant="secondary" onclick={load}>Search</Button>
  <span class="ml-auto text-xs text-muted-foreground">{total} users</span>
</div>

{#if error}<div class="mt-4 rounded-lg border border-destructive/50 p-3 text-sm text-destructive">{error}</div>{/if}

{#if loading}
  <div class="mt-4 space-y-2">{#each Array(5) as _}<div class="h-12 animate-pulse rounded-md bg-card"></div>{/each}</div>
{:else}
  <div class="mt-4 overflow-x-auto">
    <table class="w-full text-sm">
      <thead>
        <tr class="border-b border-border text-left text-xs text-muted-foreground">
          <th class="py-2 pr-3 font-medium">Username</th>
          <th class="py-2 pr-3 font-medium">Email</th>
          <th class="py-2 pr-3 font-medium">Carts</th>
          <th class="py-2 pr-3 font-medium">Status</th>
          <th class="py-2 pr-3 font-medium">Joined</th>
          <th class="py-2 pr-3 font-medium">Actions</th>
        </tr>
      </thead>
      <tbody>
        {#each users as u (u.id)}
          <tr class="border-b border-border/50">
            <td class="py-2 pr-3 font-semibold">{u.username}</td>
            <td class="py-2 pr-3 text-muted-foreground">{u.email ?? '—'}</td>
            <td class="py-2 pr-3">{u.cart_count}</td>
            <td class="py-2 pr-3">
              <div class="flex gap-1.5">
                {#if u.is_admin}<Badge variant="secondary">Admin</Badge>{/if}
                {#if u.is_banned}<Badge variant="destructive">Banned</Badge>{/if}
              </div>
              {#if u.is_banned && u.banned_reason}<div class="mt-1 text-xs text-muted-foreground">{u.banned_reason}</div>{/if}
            </td>
            <td class="py-2 pr-3 text-muted-foreground">{new Date(u.created_at).toLocaleDateString()}</td>
            <td class="py-2 pr-3">
              {#if banningId === u.id}
                <div class="flex items-center gap-1.5">
                  <input
                    bind:value={banReason}
                    placeholder="Reason…"
                    class="h-8 w-40 rounded-md border border-border bg-background px-2 text-xs"
                  />
                  <Button size="sm" variant="destructive" disabled={busyId === u.id} onclick={() => confirmBan(u.id)}>Confirm</Button>
                  <Button size="sm" variant="ghost" onclick={() => (banningId = null)}>Cancel</Button>
                </div>
              {:else}
                <div class="flex flex-wrap gap-1.5">
                  {#if u.is_banned}
                    <Button size="sm" variant="secondary" disabled={busyId === u.id} onclick={() => withBusy(u.id, () => api.adminUnbanUser(u.id))}>Unban</Button>
                  {:else if u.id !== currentUser.value?.id}
                    <Button size="sm" variant="destructive" disabled={busyId === u.id} onclick={() => startBan(u.id)}>Ban</Button>
                  {/if}
                  {#if u.is_admin}
                    <Button size="sm" variant="ghost" disabled={busyId === u.id} onclick={() => withBusy(u.id, () => api.adminDemoteUser(u.id))}>Demote</Button>
                  {:else}
                    <Button size="sm" variant="ghost" disabled={busyId === u.id} onclick={() => withBusy(u.id, () => api.adminPromoteUser(u.id))}>Promote</Button>
                  {/if}
                </div>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if !users.length}<p class="py-8 text-center text-sm text-muted-foreground">No users match this search.</p>{/if}
  </div>
{/if}
