<script lang="ts">
  import { api, ApiError, type SessionInfo, type TokenInfo } from '../api';
  import { currentUser, setUser } from '../stores.svelte';
  import { navigate } from '../router.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Spinner } from '$lib/components/ui/spinner';
  import { toast } from 'svelte-sonner';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import KeyIcon from '@lucide/svelte/icons/key-round';
  import LockIcon from '@lucide/svelte/icons/lock-keyhole';
  import MonitorIcon from '@lucide/svelte/icons/monitor';
  import ShieldIcon from '@lucide/svelte/icons/shield-check';

  let tokens = $state<TokenInfo[]>([]);
  let sessions = $state<SessionInfo[]>([]);
  let tokenName = $state('');
  let created = $state<{ name: string; token: string } | null>(null);
  let currentPassword = $state('');
  let newPassword = $state('');
  let confirmPassword = $state('');
  let loadError = $state('');
  let passwordError = $state('');
  let busyPassword = $state(false);
  let busySessions = $state(false);

  function message(error: unknown): string {
    return error instanceof ApiError || error instanceof Error ? error.message : String(error);
  }

  function formatDate(value: string): string {
    return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(value));
  }

  async function load() {
    try {
      [tokens, sessions] = await Promise.all([api.listTokens(), api.listSessions()]);
      loadError = '';
    } catch (error) {
      loadError = message(error);
    }
  }

  $effect(() => {
    void load();
  });

  async function createToken(event: Event) {
    event.preventDefault();
    try {
      const token = await api.createToken(tokenName.trim() || 'token');
      created = token;
      tokenName = '';
      await load();
    } catch (error) {
      loadError = message(error);
    }
  }

  async function revokeToken(id: string) {
    try {
      await api.revokeToken(id);
      await load();
      toast.success('API token revoked');
    } catch (error) {
      loadError = message(error);
    }
  }

  async function copy() {
    if (!created) return;
    await navigator.clipboard.writeText(created.token);
    toast.success('Token copied');
  }

  async function changePassword(event: Event) {
    event.preventDefault();
    passwordError = '';
    if (newPassword !== confirmPassword) {
      passwordError = 'New passwords do not match.';
      return;
    }
    if ([...newPassword].length < 15) {
      passwordError = 'New password must contain at least 15 characters.';
      return;
    }
    busyPassword = true;
    try {
      await api.changePassword(currentPassword, newPassword);
      currentPassword = '';
      newPassword = '';
      confirmPassword = '';
      await load();
      toast.success('Password changed. Other sessions signed out.');
    } catch (error) {
      passwordError = message(error);
    } finally {
      busyPassword = false;
    }
  }

  async function revokeSession(session: SessionInfo) {
    busySessions = true;
    try {
      await api.revokeSession(session.id);
      if (session.current) {
        setUser(null);
        navigate('/login');
        return;
      }
      await load();
      toast.success('Session revoked');
    } catch (error) {
      loadError = message(error);
    } finally {
      busySessions = false;
    }
  }

  async function revokeAllSessions() {
    if (!window.confirm('Sign out every browser, including this one?')) return;
    busySessions = true;
    try {
      await api.revokeAllSessions();
      setUser(null);
      navigate('/login');
    } catch (error) {
      loadError = message(error);
      busySessions = false;
    }
  }
</script>

<div class="container-page max-w-[900px] py-8 md:py-10">
  <div class="mb-7 flex flex-wrap items-end justify-between gap-4">
    <div>
      <p class="eyebrow">Account console</p>
      <h1 class="page-title">Settings</h1>
      <p class="mt-1 text-sm text-muted-foreground">Password, browser sessions, and publishing keys.</p>
    </div>
    <div class="flex items-center gap-2 rounded-full border border-primary/25 bg-accent px-3 py-1.5 font-mono text-xs text-accent-foreground">
      <ShieldIcon class="size-3.5" />
      Rocket-managed auth
    </div>
  </div>

  {#if loadError}
    <div class="mb-6 rounded-lg border border-destructive/50 bg-destructive/5 p-4 text-sm text-destructive">{loadError}</div>
  {/if}

  {#if currentUser.value}
    <section class="surface-panel rounded-lg p-6">
      <div class="flex items-center gap-4">
        <span class="flex size-14 items-center justify-center rounded-full bg-accent font-display text-xl font-bold text-accent-foreground">{currentUser.value.username[0]?.toUpperCase()}</span>
        <div>
          <h2 class="text-lg font-semibold">{currentUser.value.username}</h2>
          <p class="font-mono text-xs text-muted-foreground">{currentUser.value.is_admin ? 'administrator · editor' : 'player · creator'}</p>
        </div>
      </div>
    </section>

    <div class="mt-5 grid gap-5 lg:grid-cols-2">
      <section class="surface-panel rounded-lg p-6">
        <h2 class="flex items-center gap-2 font-semibold"><LockIcon class="size-4 text-primary" />Change password</h2>
        <p class="mt-2 text-sm text-muted-foreground">Changing password signs out every other browser.</p>
        {#if passwordError}<p class="mt-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">{passwordError}</p>{/if}
        <form onsubmit={changePassword} class="mt-5 space-y-3">
          <label class="block">
            <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">Current password</span>
            <Input type="password" bind:value={currentPassword} autocomplete="current-password" required />
          </label>
          <label class="block">
            <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">New password</span>
            <Input type="password" bind:value={newPassword} autocomplete="new-password" minlength={15} maxlength={128} required />
          </label>
          <label class="block">
            <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">Confirm new password</span>
            <Input type="password" bind:value={confirmPassword} autocomplete="new-password" minlength={15} maxlength={128} required />
          </label>
          <p class="font-mono text-[11px] text-muted-foreground">15–128 characters · long phrases encouraged</p>
          <Button type="submit" disabled={busyPassword}>
            {#if busyPassword}<Spinner data-icon="inline-start" />{/if}
            Update password
          </Button>
        </form>
      </section>

      <section class="surface-panel overflow-hidden rounded-lg">
        <div class="p-6">
          <h2 class="flex items-center gap-2 font-semibold"><MonitorIcon class="size-4 text-primary" />Browser sessions</h2>
          <p class="mt-2 text-sm text-muted-foreground">Maximum 20 active sessions. Oldest sessions expire first.</p>
        </div>
        {#each sessions as session}
          <div class="flex items-center gap-3 border-t border-[var(--border-subtle)] px-6 py-4">
            <span class:animate-pulse={session.current} class="size-2 rounded-full bg-primary"></span>
            <div class="min-w-0 flex-1">
              <p class="text-sm font-semibold">{session.current ? 'This browser' : 'Browser session'}</p>
              <p class="truncate font-mono text-[11px] text-muted-foreground">Started {formatDate(session.created_at)}</p>
            </div>
            <Button variant={session.current ? 'outline' : 'ghost'} size="sm" disabled={busySessions} onclick={() => revokeSession(session)}>
              {session.current ? 'Sign out' : 'Revoke'}
            </Button>
          </div>
        {:else}
          <p class="border-t border-[var(--border-subtle)] px-6 py-5 text-sm text-muted-foreground">No browser sessions.</p>
        {/each}
        <div class="border-t border-[var(--border-subtle)] p-6">
          <Button variant="destructive" disabled={busySessions} onclick={revokeAllSessions}>Sign out everywhere</Button>
        </div>
      </section>
    </div>

    <section class="surface-panel mt-5 overflow-hidden rounded-lg">
      <div class="p-6">
        <h2 class="flex items-center gap-2 font-semibold"><KeyIcon class="size-4 text-primary" />API tokens</h2>
        <p class="mt-2 text-sm text-muted-foreground">Send token as <code class="text-foreground">X-Api-Key</code>. Plaintext shown once.</p>
      </div>
      {#if created}
        <div class="mx-6 mb-5 flex items-center gap-3 rounded-md border border-primary/40 bg-accent p-3">
          <code class="min-w-0 flex-1 truncate text-xs">{created.token}</code>
          <Button size="icon" variant="secondary" onclick={copy} aria-label="Copy API token"><CopyIcon /></Button>
        </div>
      {/if}
      {#each tokens as token}
        <div class="flex flex-wrap items-center gap-3 border-t border-[var(--border-subtle)] px-6 py-4">
          <strong class="font-mono text-sm">{token.name}</strong>
          <span class="font-mono text-xs text-muted-foreground">created {new Date(token.created_at).toLocaleDateString()}</span>
          <span class="font-mono text-xs text-muted-foreground">{token.last_used_at ? `used ${new Date(token.last_used_at).toLocaleDateString()}` : 'never used'}</span>
          <button onclick={() => revokeToken(token.id)} class="ml-auto text-xs font-semibold text-destructive">Revoke</button>
        </div>
      {/each}
      <form onsubmit={createToken} class="flex gap-2 border-t border-[var(--border-subtle)] p-6">
        <Input bind:value={tokenName} maxlength={64} placeholder="Token name — Studio on laptop" class="min-w-0 flex-1" />
        <Button type="submit">Create token</Button>
      </form>
    </section>
  {/if}
</div>
