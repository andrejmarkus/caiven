import { createHmac } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { test, expect, type Page, type APIResponse } from '@playwright/test';

test.describe.configure({ mode: 'serial' });

const admin = { username: 'e2e-admin', email: 'admin@e2e.test', password: 'Q7!Caiven-E2E-Admin#2026-x9' };
const player = { username: 'e2e-player', email: 'player@e2e.test', password: 'R8!Caiven-E2E-Player#2026-y4' };

async function csrf(page: Page): Promise<Record<string, string>> {
  const cookie = (await page.context().cookies()).find((item) => item.name === 'caiven_csrf');
  expect(cookie, 'CSRF cookie must exist after authentication').toBeTruthy();
  return { 'X-CSRF-Token': cookie!.value };
}

async function ok(response: APIResponse): Promise<APIResponse> {
  expect(response.ok(), `${response.url()}: ${response.status()} ${await response.text()}`).toBe(true);
  return response;
}

async function register(page: Page, input: typeof admin) {
  const response = await ok(await page.request.post('/api/v2/auth/register', { data: input }));
  return response.json();
}

function decodeBase32(value: string): Buffer {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  let bits = '';
  for (const char of value.replace(/=+$/, '').toUpperCase()) bits += alphabet.indexOf(char).toString(2).padStart(5, '0');
  const bytes: number[] = [];
  for (let index = 0; index + 8 <= bits.length; index += 8) bytes.push(Number.parseInt(bits.slice(index, index + 8), 2));
  return Buffer.from(bytes);
}

function totp(secret: string): string {
  const counter = Math.floor(Date.now() / 30_000);
  const value = Buffer.alloc(8); value.writeBigUInt64BE(BigInt(counter));
  const digest = createHmac('sha1', decodeBase32(secret)).update(value).digest();
  const offset = digest.at(-1)! & 0x0f;
  return String((digest.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).padStart(6, '0');
}

test('real SQLite contracts: auth, bytes, community, security, Studio link, passkey, player', async ({ page, browserName }) => {
  expect(browserName).toBe('chromium');
  await page.addInitScript(() => {
    Object.defineProperty(window, '__CAIVEN_PORT_E2E__', {
      value: Object.freeze({ mode: 'live' }), configurable: false, enumerable: false, writable: false,
    });
  });
  const adminUser = await register(page, admin);
  expect(adminUser).toMatchObject({ username: admin.username, is_admin: true, email_verified: true });
  const cav = await readFile('../../../carts/dev/smoke.cav');
  const upload = await ok(await page.request.post('/api/v2/carts', {
    headers: await csrf(page),
    multipart: {
      cart: { name: 'demo_smoke.cav', mimeType: 'application/octet-stream', buffer: cav },
      meta: JSON.stringify({ title: 'Live Smoke Cart', description: 'Real Rocket upload', tags: ['e2e', 'smoke'] }),
    },
  }));
  const published = await upload.json();
  const downloaded = await ok(await page.request.get(`/api/v2/carts/${published.id}/cart`));
  expect(Buffer.from(await downloaded.body())).toEqual(cav);

  const version = await ok(await page.request.post(`/api/v2/carts/${published.id}/versions`, {
    headers: await csrf(page),
    multipart: {
      cart: { name: 'demo_smoke-v2.cav', mimeType: 'application/octet-stream', buffer: cav },
      meta: JSON.stringify({ changelog: 'Live second version' }),
    },
  }));
  expect((await version.json()).version).toBe(2);

  const jamResponse = await ok(await page.request.post('/api/v2/admin/jams', {
    headers: await csrf(page), data: { title: 'Live Jam', slug: 'live-jam', description: 'Full stack', rules: 'Ship it', starts_at: '2026-07-01T00:00:00Z', submissions_close_at: '2026-08-10T00:00:00Z', ends_at: '2026-08-12T00:00:00Z' },
  }));
  expect((await jamResponse.json()).slug).toBe('live-jam');

  const studioStart = await ok(await page.request.post('/api/v2/auth/studio-link'));
  const link = await studioStart.json();
  await ok(await page.request.post(`/api/v2/auth/studio-link/${link.request_id}/approve`, { headers: await csrf(page) }));
  const studioPoll = await ok(await page.request.post('/api/v2/auth/studio-link/poll', { data: { request_id: link.request_id, poll_secret: link.poll_secret } }));
  expect(await studioPoll.json()).toMatchObject({ status: 'linked', username: admin.username });

  const token = await ok(await page.request.post('/api/v2/auth/tokens', { headers: await csrf(page), data: { name: 'E2E token' } }));
  expect((await token.json()).token).toBeTruthy();
  expect((await (await ok(await page.request.get('/api/v2/auth/sessions'))).json()).some((row: { current: boolean }) => row.current)).toBe(true);

  const cdp = await page.context().newCDPSession(page);
  await cdp.send('WebAuthn.enable');
  await cdp.send('WebAuthn.addVirtualAuthenticator', { options: { protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true } });
  await page.goto('/settings');
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
  await page.getByPlaceholder('Passkey name — e.g. YubiKey').fill('E2E passkey');
  await page.getByRole('button', { name: 'Add passkey' }).click();
  await expect(page.getByText('E2E passkey')).toBeVisible();

  await page.goto(`/play/${published.id}`);
  const canvas = page.locator('canvas');
  await expect(canvas).toBeVisible({ timeout: 30_000 });
  await expect.poll(async () => canvas.evaluate((node: HTMLCanvasElement) => node.getContext('2d')!.getImageData(0, 0, node.width, node.height).data.some((value, i) => i % 4 !== 3 && value !== 0)), { timeout: 30_000 }).toBe(true);
  await expect.poll(async () => (await (await page.request.get(`/api/v2/carts/${published.id}`)).json()).plays, { timeout: 10_000 }).toBeGreaterThan(0);

  await ok(await page.request.post('/api/v2/auth/logout', { headers: await csrf(page) }));
  const second = await register(page, player);
  expect(second.is_admin).toBe(false);
  await ok(await page.request.post(`/api/v2/carts/${published.id}/comments`, { headers: await csrf(page), data: { body: 'Real comment' } }));
  await ok(await page.request.put(`/api/v2/carts/${published.id}/rating`, { headers: await csrf(page), data: { score: 5 } }));
  await ok(await page.request.put(`/api/v2/users/${admin.username}/follow`, { headers: await csrf(page) }));
  const collection = await ok(await page.request.post('/api/v2/collections', { headers: await csrf(page), data: { title: 'Live Shelf', description: 'Real collection' } }));
  const shelf = await collection.json();
  await ok(await page.request.post(`/api/v2/collections/${shelf.slug}/carts`, { headers: await csrf(page), data: { cart_id: published.id } }));
  await ok(await page.request.put(`/api/v2/collections/${shelf.slug}/follow`, { headers: await csrf(page) }));

  const setup = await ok(await page.request.post('/api/v2/auth/mfa/setup', { headers: await csrf(page) }));
  const secret = (await setup.json()).secret;
  const confirmed = await ok(await page.request.post('/api/v2/auth/mfa/confirm', { headers: await csrf(page), data: { code: totp(secret) } }));
  const backup = (await confirmed.json()).backup_codes[0];
  await ok(await page.request.post('/api/v2/auth/logout', { headers: await csrf(page) }));
  const login = await ok(await page.request.post('/api/v2/auth/login', { data: { identifier: player.username, password: player.password } }));
  const pending = await login.json();
  expect(pending.mfa_required).toBe(true);
  const mfaLogin = await ok(await page.request.post('/api/v2/auth/login/mfa', { data: { pending_token: pending.pending_token, code: backup } }));
  expect((await mfaLogin.json()).username).toBe(player.username);
});
