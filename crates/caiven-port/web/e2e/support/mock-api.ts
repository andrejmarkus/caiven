import { readFile } from 'node:fs/promises';
import type { BrowserContext, Page, Route } from '@playwright/test';

export interface Invocation {
  method: string;
  path: string;
  query: Record<string, string>;
  headers: Record<string, string>;
  body: string | null;
}

export interface Fault {
  method: string;
  path: string;
  status?: number;
  body?: unknown;
  delayMs?: number;
  malformed?: boolean;
  offline?: boolean;
  once?: boolean;
}

type User = {
  id: string; username: string; is_admin: boolean; email: string;
  email_verified: boolean; password_set: boolean;
};

type Cart = {
  id: string; title: string; author: string; description: string; tags: string[];
  uploaded_at: string; downloads: number; plays: number; owner: string | null;
  rating_avg: number; rating_count: number; latest_version: number;
  cart_size: number; has_screenshot: boolean; versions?: unknown[]; own_rating?: number | null;
};

type Collection = {
  slug: string; title: string; description: string; kind: 'editorial' | 'player';
  featured_rank: number | null; owner: string; cart_count: number; follower_count: number;
  followed_by_me: boolean; carts: Cart[]; created_at: string; updated_at: string;
};

type Jam = {
  slug: string; title: string; description: string; rules: string; starts_at: string;
  submissions_close_at: string; ends_at: string; status: 'upcoming' | 'open' | 'closed';
  entry_count: number; creator_count: number; carts: Cart[];
};

export const UI_CONTRACTS = [
  'GET /api/v2/auth/config', 'POST /api/v2/auth/register', 'POST /api/v2/auth/login',
  'POST /api/v2/auth/login/mfa', 'POST /api/v2/auth/logout', 'GET /api/v2/auth/me',
  'POST /api/v2/auth/set-password', 'GET /api/v2/auth/mfa/status', 'POST /api/v2/auth/mfa/setup',
  'POST /api/v2/auth/mfa/confirm', 'POST /api/v2/auth/mfa/disable', 'POST /api/v2/auth/verify-email',
  'POST /api/v2/auth/resend-verification', 'POST /api/v2/auth/forgot-password',
  'POST /api/v2/auth/reset-password', 'POST /api/v2/auth/password', 'GET /api/v2/auth/sessions',
  'DELETE /api/v2/auth/sessions/:id', 'DELETE /api/v2/auth/sessions', 'GET /api/v2/auth/tokens',
  'POST /api/v2/auth/tokens', 'DELETE /api/v2/auth/tokens/:id',
  'POST /api/v2/auth/studio-link/:id/approve', 'POST /api/v2/auth/webauthn/register/start',
  'POST /api/v2/auth/webauthn/register/finish', 'POST /api/v2/auth/webauthn/login/start',
  'POST /api/v2/auth/webauthn/login/finish', 'GET /api/v2/auth/webauthn/credentials',
  'DELETE /api/v2/auth/webauthn/credentials/:id', 'GET /api/v2/auth/audit-log',
  'DELETE /api/v2/auth/account', 'GET /api/v2/auth/export',
  'GET /api/v2/auth/oauth/:provider/start',
  'GET /api/v2/carts', 'POST /api/v2/carts', 'GET /api/v2/carts/:id',
  'PATCH /api/v2/carts/:id', 'DELETE /api/v2/carts/:id', 'POST /api/v2/carts/:id/versions',
  'GET /api/v2/carts/:id/cart', 'GET /api/v2/carts/:id/screenshot',
  'PUT /api/v2/carts/:id/rating', 'DELETE /api/v2/carts/:id/rating',
  'GET /api/v2/carts/:id/comments', 'POST /api/v2/carts/:id/comments',
  'DELETE /api/v2/carts/:id/comments/:commentId', 'POST /api/v2/carts/:id/play',
  'GET /api/v2/tags', 'GET /api/v2/users/:username', 'PUT /api/v2/users/:username/follow',
  'DELETE /api/v2/users/:username/follow', 'GET /api/v2/feed', 'GET /api/v2/dashboard',
  'GET /api/v2/collections', 'POST /api/v2/collections', 'POST /api/v2/admin/collections',
  'GET /api/v2/collections/:slug', 'PATCH /api/v2/collections/:slug',
  'DELETE /api/v2/collections/:slug', 'POST /api/v2/collections/:slug/carts',
  'DELETE /api/v2/collections/:slug/carts/:cartId', 'PUT /api/v2/collections/:slug/order',
  'PUT /api/v2/collections/:slug/follow', 'DELETE /api/v2/collections/:slug/follow',
  'GET /api/v2/jams', 'GET /api/v2/jams/:slug', 'POST /api/v2/admin/jams',
  'POST /api/v2/jams/:slug/entries', 'DELETE /api/v2/jams/:slug/entries/:cartId',
] as const;

const now = '2026-07-28T12:00:00Z';
const version = (n = 1) => ({ version: n, cart_size: 4096, changelog: n === 1 ? 'First release' : 'More polish', has_screenshot: false, created_at: now, editor: 'admin' });
const cart = (id: string, title: string, tags: string[], owner = 'admin'): Cart => ({
  id, title, author: owner, description: `${title} is a tiny deterministic adventure.`, tags,
  uploaded_at: now, downloads: 12, plays: id === 'demo' ? 321 : 42, owner,
  rating_avg: 4.5, rating_count: 8, latest_version: 1, cart_size: 4096,
  has_screenshot: false, versions: [version()], own_rating: null,
});

export class MockApi {
  readonly invocations: Invocation[] = [];
  readonly unknown: string[] = [];
  readonly faults: Fault[] = [];
  readonly allowedConsoleStatuses = new Map<number, number>();
  readonly allowedRequestFailures = new Set<string>();
  user: User | null = null;
  users = new Map<string, User>();
  carts = [cart('demo', 'Ember Quest', ['adventure', 'pixel']), cart('orbit', 'Tiny Orbit', ['arcade']), cart('garden', 'Pocket Garden', ['cozy'])];
  comments = new Map<string, Array<{ id: string; author: string; body: string; created_at: string }>>([['demo', [{ id: 'comment-1', author: 'player', body: 'Excellent tiny world.', created_at: now }]]]);
  collections: Collection[];
  jams: Jam[];
  tokens = [{ id: 'token-1', name: 'Studio', created_at: now, last_used_at: null }];
  passkeys = [{ id: 'passkey-1', label: 'Laptop', created_at: now, last_used_at: null }];
  cartBytes = Buffer.alloc(0);

  constructor(private readonly page: Page) {
    const admin: User = { id: 'user-admin', username: 'admin', is_admin: true, email: 'admin@example.test', email_verified: true, password_set: true };
    const player: User = { id: 'user-player', username: 'player', is_admin: false, email: 'player@example.test', email_verified: true, password_set: true };
    this.users.set('admin', admin); this.users.set('player', player);
    this.collections = [{
      slug: 'staff-picks', title: 'Staff Picks', description: 'Small games, big ideas.', kind: 'editorial',
      featured_rank: 1, owner: 'admin', cart_count: 2, follower_count: 5, followed_by_me: false,
      carts: this.carts.slice(0, 2), created_at: now, updated_at: now,
    }];
    this.jams = [{
      slug: 'one-button', title: 'One Button Jam', description: 'Make one input sing.', rules: 'One action button.',
      starts_at: '2026-07-01T00:00:00Z', submissions_close_at: '2026-08-10T00:00:00Z',
      ends_at: '2026-08-12T00:00:00Z', status: 'open', entry_count: 1, creator_count: 1, carts: [this.carts[0]],
    }];
  }

  async install(): Promise<void> {
    this.cartBytes = await readFile('../../../carts/demo_smoke.cav');
    await this.page.addInitScript(() => {
      Object.defineProperty(window, '__CAIVEN_PORT_E2E__', {
        value: Object.freeze({ mode: 'mock' }), configurable: false, enumerable: false, writable: false,
      });
    });
    await this.page.route('**/api/v2/**', (route) => this.dispatch(route));
  }

  async loginAs(username = 'admin'): Promise<void> {
    this.user = this.users.get(username) ?? null;
    await this.page.context().addCookies([
      { name: 'caiven_session', value: 'mock-session', url: 'http://127.0.0.1:1430', httpOnly: true, sameSite: 'Lax' },
      { name: 'caiven_csrf', value: 'mock-csrf', url: 'http://127.0.0.1:1430', sameSite: 'Lax' },
    ]);
  }

  fault(fault: Fault): void {
    this.faults.push(fault);
    if (!fault.offline && !fault.malformed) this.allowStatus(fault.status ?? 500);
  }
  allowStatus(status: number, count = 1): void { this.allowedConsoleStatuses.set(status, (this.allowedConsoleStatuses.get(status) ?? 0) + count); }
  allowRequestFailure(method: string, path: string): void { this.allowedRequestFailures.add(`${method} ${path}`); }
  calls(method: string, path: string): Invocation[] { return this.invocations.filter((x) => x.method === method && x.path === path); }

  private async json(route: Route, body: unknown, status = 200, headers: Record<string, string> = {}): Promise<void> {
    await route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body), headers });
  }

  private async dispatch(route: Route): Promise<void> {
    const request = route.request();
    const url = new URL(request.url());
    const method = request.method();
    const path = url.pathname;
    this.invocations.push({ method, path, query: Object.fromEntries(url.searchParams), headers: request.headers(), body: request.postData() });

    const faultIndex = this.faults.findIndex((x) => x.method === method && x.path === path);
    if (faultIndex >= 0) {
      const fault = this.faults[faultIndex];
      if (fault.once) this.faults.splice(faultIndex, 1);
      if (fault.delayMs) await new Promise((resolve) => setTimeout(resolve, fault.delayMs));
      if (fault.offline) return route.abort('internetdisconnected');
      if (fault.malformed) return route.fulfill({ status: fault.status ?? 200, contentType: 'application/json', body: '{broken' });
      return this.json(route, fault.body ?? { error: 'Injected failure' }, fault.status ?? 500);
    }

    if (!['GET', 'HEAD', 'OPTIONS'].includes(method) && this.user && !['/api/v2/auth/register', '/api/v2/auth/login', '/api/v2/auth/login/mfa', '/api/v2/auth/forgot-password', '/api/v2/auth/reset-password', '/api/v2/auth/verify-email', '/api/v2/auth/webauthn/login/start', '/api/v2/auth/webauthn/login/finish'].includes(path)) {
      if (request.headers()['x-csrf-token'] !== 'mock-csrf') return this.json(route, { error: 'CSRF token missing or invalid' }, 403);
    }

    if (method === 'GET' && path === '/api/v2/auth/config') return this.json(route, { turnstile_site_key: null, providers: ['github', 'google'] });
    if (method === 'GET' && path === '/api/v2/auth/me') {
      if (!this.user) this.allowStatus(401);
      return this.user ? this.json(route, this.user) : this.json(route, { error: 'Unauthorized' }, 401);
    }
    if (method === 'POST' && path === '/api/v2/auth/register') {
      const input = JSON.parse(request.postData() ?? '{}');
      if (this.users.has(input.username)) return this.json(route, { error: 'username or email already in use' }, 409);
      this.user = { id: `user-${input.username}`, username: input.username, is_admin: false, email: input.email, email_verified: true, password_set: true };
      this.users.set(this.user.username, this.user); await this.loginAs(this.user.username); return this.json(route, this.user);
    }
    if (method === 'POST' && path === '/api/v2/auth/login') {
      const input = JSON.parse(request.postData() ?? '{}');
      const found = this.users.get(input.identifier);
      if (!found || input.password === 'wrong') return this.json(route, { error: 'Unauthorized' }, 401);
      if (input.identifier === 'player' && input.password === 'MfaPass!1') return this.json(route, { mfa_required: true, pending_token: 'pending-mfa' });
      this.user = found; await this.loginAs(found.username); return this.json(route, { mfa_required: false, user: found });
    }
    if (method === 'POST' && path === '/api/v2/auth/login/mfa') { this.user = this.users.get('player')!; await this.loginAs('player'); return this.json(route, this.user); }
    if (method === 'POST' && path === '/api/v2/auth/logout') {
      this.user = null;
      await route.fulfill({ status: 204 });
      return;
    }
    if (path === '/api/v2/auth/mfa/status' && method === 'GET') return this.json(route, { enabled: false });
    if (path === '/api/v2/auth/mfa/setup' && method === 'POST') return this.json(route, { secret: 'JBSWY3DPEHPK3PXP', otpauth_url: 'otpauth://totp/Caiven', qr_png_base64: 'iVBORw0KGgo=' });
    if (path === '/api/v2/auth/mfa/confirm' && method === 'POST') return this.json(route, { backup_codes: ['backup-1', 'backup-2'] });
    if (['/api/v2/auth/mfa/disable', '/api/v2/auth/password', '/api/v2/auth/set-password', '/api/v2/auth/resend-verification', '/api/v2/auth/verify-email', '/api/v2/auth/forgot-password', '/api/v2/auth/reset-password'].includes(path) && method === 'POST') return route.fulfill({ status: 204 });
    if (path === '/api/v2/auth/sessions' && method === 'GET') return this.json(route, [{ id: 'session-1', created_at: now, expires_at: '2026-08-28T00:00:00Z', last_seen_at: now, ip: '127.0.0.1', user_agent: 'Playwright', current: true }]);
    if (path === '/api/v2/auth/sessions' && method === 'DELETE') return route.fulfill({ status: 204 });
    if (/^\/api\/v2\/auth\/sessions\/[^/]+$/.test(path) && method === 'DELETE') return route.fulfill({ status: 204 });
    if (path === '/api/v2/auth/tokens' && method === 'GET') return this.json(route, this.tokens);
    if (path === '/api/v2/auth/tokens' && method === 'POST') { const token = { id: `token-${this.tokens.length + 1}`, name: JSON.parse(request.postData() ?? '{}').name, token: 'caiven_test_secret', created_at: now, last_used_at: null }; this.tokens.push(token); return this.json(route, token); }
    if (/^\/api\/v2\/auth\/tokens\/[^/]+$/.test(path) && method === 'DELETE') { this.tokens = this.tokens.filter((x) => x.id !== path.split('/').at(-1)); return route.fulfill({ status: 204 }); }
    if (path === '/api/v2/auth/webauthn/credentials' && method === 'GET') return this.json(route, this.passkeys);
    if (/^\/api\/v2\/auth\/webauthn\/credentials\/[^/]+$/.test(path) && method === 'DELETE') return route.fulfill({ status: 204 });
    if (path.endsWith('/webauthn/register/start') && method === 'POST') return this.json(route, { token: 'register-token', options: { publicKey: {} } });
    if (path.endsWith('/webauthn/register/finish') && method === 'POST') return this.json(route, this.passkeys[0]);
    if (path.endsWith('/webauthn/login/start') && method === 'POST') return this.json(route, { token: 'login-token', options: { publicKey: {} } });
    if (path.endsWith('/webauthn/login/finish') && method === 'POST') { await this.loginAs('admin'); return this.json(route, this.user); }
    if (path === '/api/v2/auth/audit-log' && method === 'GET') return this.json(route, [{ event: 'login', ip: '127.0.0.1', user_agent: 'Playwright', metadata: null, created_at: now }]);
    if (path === '/api/v2/auth/export' && method === 'GET') return this.json(route, { user: this.user, carts: this.carts });
    if (path === '/api/v2/auth/account' && method === 'DELETE') { this.user = null; return route.fulfill({ status: 204 }); }
    if (/^\/api\/v2\/auth\/studio-link\/[^/]+\/approve$/.test(path) && method === 'POST') return route.fulfill({ status: 204 });

    if (path === '/api/v2/carts' && method === 'GET') {
      let rows = [...this.carts]; const q = url.searchParams.get('q')?.toLowerCase(); const tag = url.searchParams.get('tag'); const author = url.searchParams.get('author');
      if (q) rows = rows.filter((x) => `${x.title} ${x.description}`.toLowerCase().includes(q));
      if (tag) rows = rows.filter((x) => x.tags.includes(tag)); if (author) rows = rows.filter((x) => x.owner === author);
      const page = Number(url.searchParams.get('page') ?? 0); const per_page = Number(url.searchParams.get('per_page') ?? 20);
      return this.json(route, { carts: rows.slice(page * per_page, (page + 1) * per_page), total: rows.length, page, per_page });
    }
    if (path === '/api/v2/carts' && method === 'POST') { const created = cart(`uploaded-${this.carts.length}`, 'Uploaded Cart', ['new'], this.user?.username ?? 'admin'); this.carts.unshift(created); return this.json(route, created); }
    const cartMatch = path.match(/^\/api\/v2\/carts\/([^/]+)$/);
    if (cartMatch && method === 'GET') { const found = this.carts.find((x) => x.id === cartMatch[1]); return found ? this.json(route, { ...found, versions: found.versions ?? [version()], own_rating: found.own_rating ?? null }) : this.json(route, { error: 'cart not found' }, 404); }
    if (cartMatch && method === 'PATCH') { const found = this.carts.find((x) => x.id === cartMatch[1]); if (!found) return this.json(route, { error: 'cart not found' }, 404); Object.assign(found, JSON.parse(request.postData() ?? '{}')); return this.json(route, found); }
    if (cartMatch && method === 'DELETE') { this.carts = this.carts.filter((x) => x.id !== cartMatch[1]); return route.fulfill({ status: 204 }); }
    const binaryMatch = path.match(/^\/api\/v2\/carts\/([^/]+)\/(cart|screenshot)$/);
    if (binaryMatch && method === 'GET' && binaryMatch[2] === 'cart') return route.fulfill({ status: 200, contentType: 'application/octet-stream', body: this.cartBytes });
    if (binaryMatch && method === 'GET') return route.fulfill({ status: 200, contentType: 'image/png', body: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M/wHwAFAgIAC4k9WQAAAABJRU5ErkJggg==', 'base64') });
    const versionMatch = path.match(/^\/api\/v2\/carts\/([^/]+)\/versions$/);
    if (versionMatch && method === 'POST') { const found = this.carts.find((x) => x.id === versionMatch[1]); if (!found) return this.json(route, { error: 'cart not found' }, 404); found.latest_version++; const created = version(found.latest_version); found.versions = [...(found.versions ?? []), created]; return this.json(route, created); }
    const ratingMatch = path.match(/^\/api\/v2\/carts\/([^/]+)\/rating$/);
    if (ratingMatch && ['PUT', 'DELETE'].includes(method)) { const found = this.carts.find((x) => x.id === ratingMatch[1]); if (found) found.own_rating = method === 'PUT' ? JSON.parse(request.postData() ?? '{}').score : null; return route.fulfill({ status: 204 }); }
    const commentsMatch = path.match(/^\/api\/v2\/carts\/([^/]+)\/comments(?:\/([^/]+))?$/);
    if (commentsMatch && method === 'GET') return this.json(route, this.comments.get(commentsMatch[1]) ?? []);
    if (commentsMatch && method === 'POST') { const row = { id: `comment-${Date.now()}`, author: this.user?.username ?? 'admin', body: JSON.parse(request.postData() ?? '{}').body, created_at: now }; this.comments.set(commentsMatch[1], [...(this.comments.get(commentsMatch[1]) ?? []), row]); return this.json(route, row); }
    if (commentsMatch && method === 'DELETE') { this.comments.set(commentsMatch[1], (this.comments.get(commentsMatch[1]) ?? []).filter((x) => x.id !== commentsMatch[2])); return route.fulfill({ status: 204 }); }
    if (/^\/api\/v2\/carts\/[^/]+\/play$/.test(path) && method === 'POST') { const found = this.carts.find((x) => path.includes(`/${x.id}/`)); if (found) found.plays++; return this.json(route, { counted: true, plays: found?.plays ?? 0 }); }

    if (path === '/api/v2/tags' && method === 'GET') return this.json(route, [...new Set(this.carts.flatMap((x) => x.tags))].map((tag) => ({ tag, count: this.carts.filter((x) => x.tags.includes(tag)).length })));
    const userMatch = path.match(/^\/api\/v2\/users\/([^/]+)(?:\/follow)?$/);
    if (userMatch && method === 'GET') { const found = this.users.get(userMatch[1]); return found ? this.json(route, { username: found.username, is_admin: found.is_admin, created_at: now, carts: this.carts.filter((x) => x.owner === found.username), total: 1, total_plays: 321, follower_count: 4, following_count: 2, followed_by_me: false }) : this.json(route, { error: 'user not found' }, 404); }
    if (userMatch && ['PUT', 'DELETE'].includes(method) && path.endsWith('/follow')) return route.fulfill({ status: 204 });
    if (path === '/api/v2/feed' && method === 'GET') return this.json(route, { events: [{ kind: 'cart_published', actor: 'admin', occurred_at: now, cart: this.carts[0], version: 1, collection_slug: null, collection_title: null, jam_slug: null, jam_title: null }], page: 0, per_page: 20, total: 1 });
    if (path === '/api/v2/dashboard' && method === 'GET') return this.json(route, { plays: { current: 321, previous: 200 }, unique_players: { current: 42, previous: 30 }, rating_avg: 4.5, followers: 4, new_followers: 1, daily: [{ date: '2026-07-28', plays: 12, unique_players: 4 }], carts: this.carts.filter((x) => x.owner === this.user?.username) });

    if (path === '/api/v2/collections' && method === 'GET') { const kind = url.searchParams.get('kind'); const owner = url.searchParams.get('owner'); return this.json(route, this.collections.filter((x) => (!kind || x.kind === kind) && (!owner || x.owner === owner))); }
    if (path === '/api/v2/collections' && method === 'POST' || path === '/api/v2/admin/collections' && method === 'POST') { const input = JSON.parse(request.postData() ?? '{}'); const created: Collection = { slug: input.title.toLowerCase().replace(/[^a-z0-9]+/g, '-'), title: input.title, description: input.description ?? '', kind: path.includes('/admin/') ? 'editorial' : 'player', featured_rank: input.featured_rank ?? null, owner: this.user?.username ?? 'admin', cart_count: 0, follower_count: 0, followed_by_me: false, carts: [], created_at: now, updated_at: now }; this.collections.push(created); return this.json(route, created); }
    const collectionMatch = path.match(/^\/api\/v2\/collections\/([^/]+)(?:\/(carts|order|follow)(?:\/([^/]+))?)?$/);
    if (collectionMatch) {
      const found = this.collections.find((x) => x.slug === collectionMatch[1]); if (!found) return this.json(route, { error: 'collection not found' }, 404);
      if (!collectionMatch[2] && method === 'GET') return this.json(route, found);
      if (!collectionMatch[2] && method === 'PATCH') { Object.assign(found, JSON.parse(request.postData() ?? '{}')); return this.json(route, found); }
      if (!collectionMatch[2] && method === 'DELETE') { this.collections = this.collections.filter((x) => x !== found); return route.fulfill({ status: 204 }); }
      if (collectionMatch[2] === 'carts' && method === 'POST') { const id = JSON.parse(request.postData() ?? '{}').cart_id; const selected = this.carts.find((x) => x.id === id); if (selected && !found.carts.includes(selected)) found.carts.push(selected); found.cart_count = found.carts.length; return this.json(route, found); }
      if (collectionMatch[2] === 'carts' && method === 'DELETE') { found.carts = found.carts.filter((x) => x.id !== collectionMatch[3]); found.cart_count = found.carts.length; return this.json(route, found); }
      if (collectionMatch[2] === 'order' && method === 'PUT') { const ids = JSON.parse(request.postData() ?? '{}').cart_ids as string[]; found.carts.sort((a, b) => ids.indexOf(a.id) - ids.indexOf(b.id)); return this.json(route, found); }
      if (collectionMatch[2] === 'follow' && ['PUT', 'DELETE'].includes(method)) { found.followed_by_me = method === 'PUT'; return route.fulfill({ status: 204 }); }
    }
    if (path === '/api/v2/jams' && method === 'GET') return this.json(route, this.jams);
    if (path === '/api/v2/admin/jams' && method === 'POST') { const input = JSON.parse(request.postData() ?? '{}'); const created: Jam = { ...input, slug: input.slug ?? input.title.toLowerCase().replace(/[^a-z0-9]+/g, '-'), description: input.description ?? '', rules: input.rules ?? '', status: 'upcoming', entry_count: 0, creator_count: 0, carts: [] }; this.jams.push(created); return this.json(route, created); }
    const jamMatch = path.match(/^\/api\/v2\/jams\/([^/]+)(?:\/entries(?:\/([^/]+))?)?$/);
    if (jamMatch) { const found = this.jams.find((x) => x.slug === jamMatch[1]); if (!found) return this.json(route, { error: 'jam not found' }, 404); if (!path.includes('/entries') && method === 'GET') return this.json(route, found); if (method === 'POST') { const id = JSON.parse(request.postData() ?? '{}').cart_id; const selected = this.carts.find((x) => x.id === id); if (selected && !found.carts.includes(selected)) found.carts.push(selected); found.entry_count = found.carts.length; return this.json(route, found); } if (method === 'DELETE') { found.carts = found.carts.filter((x) => x.id !== jamMatch[2]); found.entry_count = found.carts.length; return this.json(route, found); } }

    const key = `${method} ${path}`; this.unknown.push(key);
    await this.json(route, { error: `Unhandled mock request: ${key}` }, 599);
  }
}

declare global {
  interface Window { __CAIVEN_PORT_E2E__?: Readonly<{ mode: 'mock' | 'live' }>; }
}

export async function clearBrowserState(context: BrowserContext): Promise<void> {
  await context.clearCookies();
}
