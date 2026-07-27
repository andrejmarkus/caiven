<script lang="ts">
  import { api, ApiError, type AuditEntry, type PasskeyInfo, type SessionInfo, type TokenInfo } from '../api';
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
  import SmartphoneIcon from '@lucide/svelte/icons/smartphone';
  import FingerprintIcon from '@lucide/svelte/icons/fingerprint';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import { passkeysSupported, createPasskey } from '../webauthn';

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

  let mfaEnabled = $state(false);
  let mfaBusy = $state(false);
  let mfaError = $state('');
  let mfaSetup = $state<{ secret: string; otpauth_url: string; qr_png_base64: string } | null>(null);
  let mfaConfirmCode = $state('');
  let backupCodes = $state<string[] | null>(null);
  let mfaDisablePassword = $state('');
  let mfaDisableCode = $state('');

  let passkeys = $state<PasskeyInfo[]>([]);
  let passkeyBusy = $state(false);
  let passkeyError = $state('');
  let passkeyLabel = $state('');

  let auditEntries = $state<AuditEntry[]>([]);

  let deletePassword = $state('');
  let deleteCode = $state('');
  let deleteConfirm = $state('');
  let deleteBusy = $state(false);
  let deleteError = $state('');

  function message(error: unknown): string {
    return error instanceof ApiError || error instanceof Error ? error.message : String(error);
  }

  function formatDate(value: string): string {
    return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(value));
  }

  async function load() {
    try {
      const [t, s, mfa, pk, audit] = await Promise.all([
        api.listTokens(),
        api.listSessions(),
        api.mfaStatus(),
        api.listPasskeys(),
        api.auditLog(),
      ]);
      tokens = t;
      sessions = s;
      mfaEnabled = mfa.enabled;
      passkeys = pk;
      auditEntries = audit;
      loadError = '';
    } catch (error) {
      loadError = message(error);
    }
  }

  $effect(() => {
    void load();
  });

  async function setPassword(event: Event) {
    event.preventDefault();
    passwordError = '';
    if (newPassword !== confirmPassword) {
      passwordError = 'New passwords do not match.';
      return;
    }
    if ([...newPassword].length < 8) {
      passwordError = 'New password must contain at least 8 characters, an uppercase letter, and a special character.';
      return;
    }
    busyPassword = true;
    try {
      await api.setPassword(newPassword);
      newPassword = '';
      confirmPassword = '';
      setUser(currentUser.value ? { ...currentUser.value, password_set: true } : null);
      toast.success('Password set. You can now log in with it.');
    } catch (error) {
      passwordError = message(error);
    } finally {
      busyPassword = false;
    }
  }

  async function startMfaSetup() {
    mfaBusy = true;
    mfaError = '';
    try {
      mfaSetup = await api.mfaSetup();
    } catch (error) {
      mfaError = message(error);
    } finally {
      mfaBusy = false;
    }
  }

  async function confirmMfa(event: Event) {
    event.preventDefault();
    mfaBusy = true;
    mfaError = '';
    try {
      const result = await api.mfaConfirm(mfaConfirmCode);
      backupCodes = result.backup_codes;
      mfaEnabled = true;
      mfaSetup = null;
      mfaConfirmCode = '';
      toast.success('Two-factor authentication enabled');
    } catch (error) {
      mfaError = message(error);
    } finally {
      mfaBusy = false;
    }
  }

  async function disableMfa(event: Event) {
    event.preventDefault();
    mfaBusy = true;
    mfaError = '';
    try {
      await api.mfaDisable(mfaDisablePassword, mfaDisableCode);
      mfaEnabled = false;
      mfaDisablePassword = '';
      mfaDisableCode = '';
      backupCodes = null;
      toast.success('Two-factor authentication disabled');
    } catch (error) {
      mfaError = message(error);
    } finally {
      mfaBusy = false;
    }
  }

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
    if ([...newPassword].length < 8) {
      passwordError = 'New password must contain at least 8 characters, an uppercase letter, and a special character.';
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

  async function addPasskey(event: Event) {
    event.preventDefault();
    passkeyBusy = true;
    passkeyError = '';
    try {
      const { token, options } = await api.webauthnRegisterStart();
      const credential = await createPasskey(options);
      await api.webauthnRegisterFinish(token, passkeyLabel.trim() || 'Passkey', credential);
      passkeyLabel = '';
      await load();
      toast.success('Passkey added');
    } catch (error) {
      passkeyError = message(error);
    } finally {
      passkeyBusy = false;
    }
  }

  async function removePasskey(id: string) {
    try {
      await api.deletePasskey(id);
      await load();
      toast.success('Passkey removed');
    } catch (error) {
      passkeyError = message(error);
    }
  }

  async function deleteAccount(event: Event) {
    event.preventDefault();
    deleteError = '';
    if (deleteConfirm !== currentUser.value?.username) {
      deleteError = 'Type your username exactly to confirm.';
      return;
    }
    deleteBusy = true;
    try {
      await api.deleteAccount(deletePassword, deleteCode || undefined);
      setUser(null);
      navigate('/login');
    } catch (error) {
      deleteError = message(error);
      deleteBusy = false;
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
        {#if currentUser.value.password_set}
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
              <Input type="password" bind:value={newPassword} autocomplete="new-password" minlength={8} maxlength={128} required />
            </label>
            <label class="block">
              <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">Confirm new password</span>
              <Input type="password" bind:value={confirmPassword} autocomplete="new-password" minlength={8} maxlength={128} required />
            </label>
            <p class="font-mono text-[11px] text-muted-foreground">8–128 characters · needs an uppercase letter and a special character</p>
            <Button type="submit" disabled={busyPassword}>
              {#if busyPassword}<Spinner data-icon="inline-start" />{/if}
              Update password
            </Button>
          </form>
        {:else}
          <h2 class="flex items-center gap-2 font-semibold"><LockIcon class="size-4 text-primary" />Set a password</h2>
          <p class="mt-2 text-sm text-muted-foreground">This account signed up via social login and has no password yet. Add one to also be able to log in directly.</p>
          {#if passwordError}<p class="mt-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">{passwordError}</p>{/if}
          <form onsubmit={setPassword} class="mt-5 space-y-3">
            <label class="block">
              <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">New password</span>
              <Input type="password" bind:value={newPassword} autocomplete="new-password" minlength={8} maxlength={128} required />
            </label>
            <label class="block">
              <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">Confirm new password</span>
              <Input type="password" bind:value={confirmPassword} autocomplete="new-password" minlength={8} maxlength={128} required />
            </label>
            <p class="font-mono text-[11px] text-muted-foreground">8–128 characters · needs an uppercase letter and a special character</p>
            <Button type="submit" disabled={busyPassword}>
              {#if busyPassword}<Spinner data-icon="inline-start" />{/if}
              Set password
            </Button>
          </form>
        {/if}
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
              <p class="text-sm font-semibold">{session.current ? 'This browser' : (session.user_agent ?? 'Browser session')}</p>
              <p class="truncate font-mono text-[11px] text-muted-foreground">
                {session.ip ?? 'unknown IP'} · started {formatDate(session.created_at)} · last active {formatDate(session.last_seen_at)}
              </p>
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

    <section class="surface-panel mt-5 rounded-lg p-6">
      <h2 class="flex items-center gap-2 font-semibold"><SmartphoneIcon class="size-4 text-primary" />Two-factor authentication</h2>
      <p class="mt-2 text-sm text-muted-foreground">Require a code from an authenticator app to log in.</p>
      {#if mfaError}<p class="mt-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">{mfaError}</p>{/if}

      {#if backupCodes}
        <div class="mt-5 rounded-md border border-primary/40 bg-accent p-4">
          <p class="text-sm font-semibold">Save your backup codes</p>
          <p class="mt-1 text-xs text-muted-foreground">Each code works once, if you lose access to your authenticator app. Shown only now.</p>
          <div class="mt-3 grid grid-cols-2 gap-2 font-mono text-xs">
            {#each backupCodes as code}<span>{code}</span>{/each}
          </div>
          <Button class="mt-4" size="sm" variant="secondary" onclick={() => (backupCodes = null)}>Done</Button>
        </div>
      {:else if mfaEnabled}
        <p class="mt-4 text-sm">Enabled.</p>
        <form onsubmit={disableMfa} class="mt-3 space-y-3">
          <label class="block">
            <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">Current password</span>
            <Input type="password" bind:value={mfaDisablePassword} autocomplete="current-password" required />
          </label>
          <label class="block">
            <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">Code or backup code</span>
            <Input bind:value={mfaDisableCode} required />
          </label>
          <Button type="submit" variant="destructive" disabled={mfaBusy}>
            {#if mfaBusy}<Spinner data-icon="inline-start" />{/if}
            Disable two-factor authentication
          </Button>
        </form>
      {:else if mfaSetup}
        <div class="mt-4 space-y-3">
          <img src={`data:image/png;base64,${mfaSetup.qr_png_base64}`} alt="Scan with your authenticator app" class="size-40 rounded-md border border-[var(--border-subtle)] bg-white p-2" />
          <p class="font-mono text-[11px] text-muted-foreground break-all">Or enter manually: {mfaSetup.secret}</p>
          <form onsubmit={confirmMfa} class="space-y-3">
            <label class="block">
              <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">Code from your app</span>
              <Input bind:value={mfaConfirmCode} inputmode="numeric" required />
            </label>
            <Button type="submit" disabled={mfaBusy}>
              {#if mfaBusy}<Spinner data-icon="inline-start" />{/if}
              Confirm
            </Button>
          </form>
        </div>
      {:else}
        <Button class="mt-4" disabled={mfaBusy} onclick={startMfaSetup}>
          {#if mfaBusy}<Spinner data-icon="inline-start" />{/if}
          Enable two-factor authentication
        </Button>
      {/if}
    </section>

    {#if passkeysSupported()}
      <section class="surface-panel mt-5 overflow-hidden rounded-lg">
        <div class="p-6">
          <h2 class="flex items-center gap-2 font-semibold"><FingerprintIcon class="size-4 text-primary" />Passkeys</h2>
          <p class="mt-2 text-sm text-muted-foreground">Sign in without a password, using your device's biometrics or security key.</p>
          {#if passkeyError}<p class="mt-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">{passkeyError}</p>{/if}
        </div>
        {#each passkeys as passkey}
          <div class="flex flex-wrap items-center gap-3 border-t border-[var(--border-subtle)] px-6 py-4">
            <strong class="font-mono text-sm">{passkey.label}</strong>
            <span class="font-mono text-xs text-muted-foreground">added {formatDate(passkey.created_at)}</span>
            <span class="font-mono text-xs text-muted-foreground">{passkey.last_used_at ? `used ${formatDate(passkey.last_used_at)}` : 'never used'}</span>
            <button onclick={() => removePasskey(passkey.id)} class="ml-auto text-xs font-semibold text-destructive">Remove</button>
          </div>
        {/each}
        <form onsubmit={addPasskey} class="flex gap-2 border-t border-[var(--border-subtle)] p-6">
          <Input bind:value={passkeyLabel} maxlength={64} placeholder="Passkey name — e.g. YubiKey" class="min-w-0 flex-1" />
          <Button type="submit" disabled={passkeyBusy}>
            {#if passkeyBusy}<Spinner data-icon="inline-start" />{/if}
            Add passkey
          </Button>
        </form>
      </section>
    {/if}

    <section class="surface-panel mt-5 overflow-hidden rounded-lg">
      <div class="p-6">
        <h2 class="flex items-center gap-2 font-semibold"><HistoryIcon class="size-4 text-primary" />Security activity</h2>
        <p class="mt-2 text-sm text-muted-foreground">Recent security events on this account.</p>
      </div>
      {#each auditEntries as entry}
        <div class="flex flex-wrap items-center gap-3 border-t border-[var(--border-subtle)] px-6 py-3 text-sm">
          <span class="font-mono">{entry.event.replace(/_/g, ' ')}</span>
          {#if entry.ip}<span class="font-mono text-xs text-muted-foreground">{entry.ip}</span>{/if}
          <span class="ml-auto font-mono text-xs text-muted-foreground">{formatDate(entry.created_at)}</span>
        </div>
      {:else}
        <p class="border-t border-[var(--border-subtle)] px-6 py-5 text-sm text-muted-foreground">No activity recorded yet.</p>
      {/each}
    </section>

    <section class="surface-panel mt-5 rounded-lg border-destructive/40 p-6">
      <h2 class="flex items-center gap-2 font-semibold text-destructive"><LockIcon class="size-4" />Danger zone</h2>
      <p class="mt-2 text-sm text-muted-foreground">Export your data, or permanently delete your account.</p>

      <div class="mt-4">
        <Button variant="outline" size="sm" href={api.exportUrl()} target="_blank" rel="noopener">
          Download my data (JSON)
        </Button>
      </div>

      {#if deleteError}<p class="mt-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">{deleteError}</p>{/if}
      <form onsubmit={deleteAccount} class="mt-4 space-y-3">
        <label class="block">
          <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">Current password</span>
          <Input type="password" bind:value={deletePassword} autocomplete="current-password" required />
        </label>
        {#if mfaEnabled}
          <label class="block">
            <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">Two-factor code or backup code</span>
            <Input bind:value={deleteCode} required />
          </label>
        {/if}
        <label class="block">
          <span class="mb-1.5 block text-xs font-semibold text-muted-foreground">Type "{currentUser.value.username}" to confirm</span>
          <Input bind:value={deleteConfirm} required />
        </label>
        <Button type="submit" variant="destructive" disabled={deleteBusy}>
          {#if deleteBusy}<Spinner data-icon="inline-start" />{/if}
          Permanently delete my account
        </Button>
      </form>
    </section>
  {/if}
</div>
