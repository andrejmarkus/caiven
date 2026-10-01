import { readFile } from 'node:fs/promises';
import { test, expect, type Page, type APIResponse } from '@playwright/test';
import { parseCav, withLuaSource } from '../../src/lib/cav.js';
import { editorText, fillEditor } from '../support/editor';

// Runs after full-stack.spec.ts (files run in name order on one worker),
// which registers the first — admin — account.
const creator = { username: 'e2e-seed-maker', email: 'seed@e2e.test', password: 'T6!Caiven-E2E-Seed#2026-q2' };
const remixer = { username: 'e2e-remixer', email: 'remixer@e2e.test', password: 'U5!Caiven-E2E-Remix#2026-w7' };
const third = { username: 'e2e-remixer-two', email: 'remixer2@e2e.test', password: 'V4!Caiven-E2E-Remix#2026-z3' };
// Registered by full-stack.spec.ts; only it may read the funnel.
const admin = { identifier: 'e2e-admin', password: 'Q7!Caiven-E2E-Admin#2026-x9' };
const SEED = 'local COLOR = 8\nfunction _update()\n  fill_screen(COLOR)\nend\n';

async function ok(response: APIResponse): Promise<APIResponse> {
  expect(response.ok(), `${response.url()}: ${response.status()} ${await response.text()}`).toBe(true);
  return response;
}

async function csrf(page: Page): Promise<Record<string, string>> {
  const cookie = (await page.context().cookies()).find((item) => item.name === 'caiven_csrf');
  return { 'X-CSRF-Token': cookie!.value };
}

async function centerPixel(page: Page): Promise<number[]> {
  // Read through a copy so repeated polling doesn't trip Chrome's readback warning.
  return page.locator('canvas').evaluate((node: HTMLCanvasElement) => {
    const copy = document.createElement('canvas').getContext('2d', { willReadFrequently: true })!;
    copy.drawImage(node, 0, 0);
    return Array.from(copy.getImageData(96, 64, 1, 1).data);
  });
}

test('anonymous player remixes a seed cart and publishes a linked child', async ({ page, browser }) => {
  const since = new Date().toISOString();
  await ok(await page.request.post('/api/v1/auth/register', { data: creator }));
  const smoke = new Uint8Array(await readFile('../../../carts/dev/smoke.cav'));
  const seedCav = Buffer.from(withLuaSource(parseCav(smoke), SEED, { title: 'Color Seed' }));
  const seed = await (await ok(await page.request.post('/api/v1/carts', {
    headers: await csrf(page),
    multipart: {
      cart: { name: 'seed.cav', mimeType: 'application/octet-stream', buffer: seedCav },
      meta: JSON.stringify({ title: 'Color Seed', description: 'Change one thing', tags: ['seed'], remixable: true }),
    },
  }))).json();
  await ok(await page.request.post('/api/v1/auth/logout', { headers: await csrf(page) }));
  await page.context().clearCookies();

  // 1-4: anonymous visitor plays from a plain URL.
  await page.goto(`/play/${seed.id}`);
  await expect.poll(() => centerPixel(page), { timeout: 30_000 }).not.toEqual([0, 0, 0, 0]);
  const original = await centerPixel(page);

  // 5-9: remix, see real Lua, change it, rerun, see the difference.
  await page.getByRole('link', { name: 'Remix this' }).click();
  const editor = page.getByLabel('Lua source');
  await expect.poll(() => editorText(editor)).toBe(SEED);
  await expect.poll(() => centerPixel(page), { timeout: 30_000 }).toEqual(original);
  await fillEditor(editor, SEED.replace('= 8', '= 12'));
  await editor.press('Control+Enter');
  await expect.poll(() => centerPixel(page)).not.toEqual(original);
  const remixed = await centerPixel(page);
  // A rerun keeps the last picture until the new build draws, never a blank frame.
  expect(remixed[3]).toBe(255);

  // 10-12: the edit survives the auth wall, which appears only at publish.
  await page.getByRole('button', { name: 'Publish my version' }).click();
  await expect(page).toHaveURL(/\/register\?next=/);
  await expect(page.getByTestId('remix-saved')).toBeVisible();
  await page.getByLabel('Username').fill(remixer.username);
  await page.getByLabel('Email').fill(remixer.email);
  await page.getByLabel('Password').fill(remixer.password);
  await page.getByRole('checkbox').check();
  await page.getByRole('button', { name: 'Create account' }).click();
  await expect(page).toHaveURL(new RegExp(`/remix/${seed.id}`));
  await expect.poll(() => editorText(editor)).toBe(SEED.replace('= 8', '= 12'));

  // 13-16: publish creates a new cart with structured parent attribution.
  const form = page.getByRole('form', { name: 'Publish your remix' });
  await expect(form).toBeVisible({ timeout: 30_000 });
  await form.getByLabel('Title').fill('Color Seed, teal');
  await form.getByRole('button', { name: `Publish as @${remixer.username}` }).click();
  await expect(page.getByText("Published. It's yours now.")).toBeVisible();
  const childUrl = await page.locator('code', { hasText: '/play/' }).textContent();
  const childId = childUrl!.split('/play/')[1];

  const child = await (await ok(await page.request.get(`/api/v1/carts/${childId}`))).json();
  expect(child).toMatchObject({ parent_cart_id: seed.id, root_cart_id: seed.id, owner: remixer.username, remixable: true });
  expect(child.parent).toMatchObject({ id: seed.id, title: 'Color Seed', owner: creator.username });
  expect(child.has_screenshot).toBe(true);
  const parent = await (await ok(await page.request.get(`/api/v1/carts/${seed.id}`))).json();
  expect(parent.remix_count).toBe(1);
  await page.goto(`/cart/${seed.id}`);
  await expect(page.getByText('1 remix', { exact: true })).toBeVisible();

  // 17-18: a different, logged-out visitor plays the child from its URL.
  const visitor = await browser.newContext();
  const other = await visitor.newPage();
  await other.goto(`/play/${childId}`);
  await expect.poll(() => centerPixel(other), { timeout: 30_000 }).toEqual(remixed);
  await other.goto(`/cart/${childId}`);
  await expect(other.getByText('Remixed from')).toBeVisible();

  // 19-23: that visitor remixes the remix; the grandchild keeps the root.
  await other.goto(`/play/${childId}`);
  await other.getByRole('link', { name: 'Remix this' }).click();
  const otherEditor = other.getByLabel('Lua source');
  await expect.poll(() => editorText(otherEditor)).toBe(SEED.replace('= 8', '= 12'));
  await expect.poll(() => centerPixel(other), { timeout: 30_000 }).toEqual(remixed);
  await fillEditor(otherEditor, SEED.replace('= 8', '= 9'));
  await otherEditor.press('Control+Enter');
  await expect.poll(() => centerPixel(other)).not.toEqual(remixed);
  await other.getByRole('button', { name: 'Publish my version' }).click();
  await expect(other).toHaveURL(/\/register\?next=/);
  await other.getByLabel('Username').fill(third.username);
  await other.getByLabel('Email').fill(third.email);
  await other.getByLabel('Password').fill(third.password);
  await other.getByRole('checkbox').check();
  await other.getByRole('button', { name: 'Create account' }).click();
  const otherForm = other.getByRole('form', { name: 'Publish your remix' });
  await expect(otherForm).toBeVisible({ timeout: 30_000 });
  await otherForm.getByRole('button', { name: `Publish as @${third.username}` }).click();
  await expect(other.getByText("Published. It's yours now.")).toBeVisible();
  const grandchildId = (await other.locator('code', { hasText: '/play/' }).textContent())!.split('/play/')[1];
  const grandchild = await (await ok(await other.request.get(`/api/v1/carts/${grandchildId}`))).json();
  expect(grandchild).toMatchObject({ parent_cart_id: childId, root_cart_id: seed.id, owner: third.username });
  expect(grandchild.parent).toMatchObject({ id: childId, owner: remixer.username });
  await visitor.close();

  // 24-25: the readout matches what happened. Every anonymous visitor here
  // is 127.0.0.1, so both remixers share one anonymous viewer key.
  const staff = await browser.newContext();
  await ok(await staff.request.post('/api/v1/auth/login', { data: admin }));
  const funnel = await (await ok(await staff.request.get(`/api/v1/admin/metrics/remix-funnel?since=${encodeURIComponent(since)}`))).json();
  await staff.close();
  const row = (id: string) => funnel.by_cart.find((r: { cart_id: string }) => r.cart_id === id);
  const step = { plays: 1, qualified_plays: 0, remix_opened: 1, remix_ran: 1, publish_started: 1, remixes_published: 1, remixes_with_external_play: 0 };
  expect(row(seed.id)).toMatchObject({ ...step, remixes_remixed: 1 });
  expect(row(childId)).toMatchObject({ ...step, parent_cart_id: seed.id, remixes_remixed: 0 });
  expect(funnel).toMatchObject({ carts_published: 3, remixes_published: 2, remix_opened: 2, remix_ran: 2, publish_started: 2 });
});
