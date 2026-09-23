import type { Page } from '@playwright/test';
import { test, expect } from '../support/fixtures';
import { parseCav, withLuaSource } from '../../src/lib/cav.js';

const SEED = 'local COLOR = 8\nfunction _update()\n  fill_screen(COLOR)\nend\n';

async function pixel(page: Page): Promise<number[]> {
  // Read through a copy so repeated polling doesn't trip Chrome's readback warning.
  return page.locator('canvas').evaluate((node: HTMLCanvasElement) => {
    const copy = document.createElement('canvas').getContext('2d', { willReadFrequently: true })!;
    copy.drawImage(node, 0, 0);
    return Array.from(copy.getImageData(96, 64, 1, 1).data);
  });
}

async function setSource(page: Page, source: string) {
  const editor = page.getByLabel('Lua source');
  await editor.fill(source);
  await editor.press('Control+Enter');
}

test('play → remix → change → run → publish after login → child is linked', async ({ page, mock }) => {
  mock.carts[0].remixable = true;
  mock.cartBytes = Buffer.from(withLuaSource(parseCav(new Uint8Array(mock.cartBytes)), SEED));

  await page.goto('/play/demo');
  await page.getByRole('link', { name: 'Remix this' }).click();
  await expect(page).toHaveURL(/\/remix\/demo$/);
  await expect(page.getByLabel('Lua source')).toHaveValue(SEED);
  await expect.poll(() => mock.calls('POST', '/api/v2/carts/demo/funnel').length).toBeGreaterThan(0);

  await expect.poll(() => pixel(page), { timeout: 30_000 }).not.toEqual([0, 0, 0, 0]);
  const before = await pixel(page);
  await expect(page.getByRole('button', { name: 'Publish my version' })).toBeDisabled();

  // The constant chip edits the real Lua line.
  const chip = page.getByRole('spinbutton', { name: 'COLOR' });
  await chip.fill('12');
  await chip.dispatchEvent('change');
  await expect(page.getByLabel('Lua source')).toHaveValue(SEED.replace('= 8', '= 12'));
  await expect.poll(() => pixel(page)).not.toEqual(before);
  const changed = await pixel(page);
  await expect(page.getByText("It runs. That's your change on the left.")).toBeVisible();

  // A broken edit keeps the last working build running and marks the line.
  await setSource(page, 'local COLOR = 3\nfunction _update()\n  fill_screen(COLOR\nend\n');
  await expect(page.getByRole('alert')).toContainText('Line 4');
  await expect(page.getByTestId('error-line')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Publish my version' })).toBeDisabled();
  await expect.poll(() => pixel(page)).toEqual(changed);

  await setSource(page, SEED.replace('= 8', '= 12'));
  await expect(page.getByRole('alert')).toHaveCount(0);

  // Anonymous publish goes through login and comes back with the edit intact.
  await page.getByRole('button', { name: 'Publish my version' }).click();
  await expect(page).toHaveURL(/\/login\?next=/);
  await mock.loginAs('player');
  await page.goto(decodeURIComponent(new URL(page.url()).searchParams.get('next')!));
  await expect(page.getByLabel('Lua source')).toHaveValue(SEED.replace('= 8', '= 12'));
  await expect(page.getByText('Restored your saved edit')).toBeVisible();
  const form = page.getByRole('form', { name: 'Publish your remix' });
  await expect(form).toBeVisible();
  await form.getByLabel('Title').fill('Ember Quest, but teal');
  await form.getByRole('button', { name: /Publish as @player/ }).click();

  await expect(page.getByText("Published. It's yours now.")).toBeVisible();
  const child = mock.carts.find((c) => c.parent_cart_id === 'demo')!;
  expect(child).toMatchObject({ root_cart_id: 'demo', remixable: true, owner: 'player' });
  const upload = mock.calls('POST', '/api/v2/carts')[0];
  expect(upload.body).toContain('"parent_cart_id":"demo"');
  expect(upload.body).toContain('"title":"Ember Quest, but teal"');
  await expect(page.getByText(`/play/${child.id}`)).toBeVisible();

  await page.getByRole('link', { name: 'Cart page' }).click();
  const lineage = page.getByText('Remixed from');
  await expect(lineage.getByRole('link', { name: 'Ember Quest', exact: true })).toBeVisible();
  await expect(lineage.getByRole('link', { name: '@admin' })).toBeVisible();
  await page.goto('/cart/demo');
  await expect(page.getByText('1 remix', { exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: child.title, exact: true })).toBeVisible();
});

test('carts that are not opted in offer no remix', async ({ page, mock }) => {
  expect(mock.carts[1].remixable).toBe(false);
  await page.goto('/play/orbit');
  await expect(page.locator('canvas')).toBeVisible({ timeout: 30_000 });
  await expect(page.getByRole('link', { name: 'Remix this' })).toHaveCount(0);
  await page.goto('/remix/orbit');
  await expect(page.getByRole('heading', { name: "This cart isn't open for remixing" })).toBeVisible();
});

test('home Start here row lists curated remixable carts and opens Quick Remix', async ({ page, mock }) => {
  mock.carts[0].remixable = true;
  mock.collections.push({
    slug: 'start-here', title: 'Start here', description: '', kind: 'editorial',
    featured_rank: 0, owner: 'admin', cart_count: 2, follower_count: 0, followed_by_me: false,
    carts: [mock.carts[0], mock.carts[1]], created_at: mock.carts[0].uploaded_at, updated_at: mock.carts[0].uploaded_at,
  });

  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Change one number. Make it yours.' })).toBeVisible();
  // The curated row never takes over the editorial shelf.
  await expect(page.getByRole('heading', { name: 'Staff Picks' })).toBeVisible();
  // Closed carts stay out even when curated.
  await expect(page.getByRole('link', { name: 'Remix Tiny Orbit' })).toHaveCount(0);
  await page.getByRole('link', { name: 'Remix Ember Quest' }).click();
  await expect(page).toHaveURL(/\/remix\/demo$/);
});

test('typed edits rerun on their own once typing pauses', async ({ page, mock }) => {
  mock.carts[0].remixable = true;
  mock.cartBytes = Buffer.from(withLuaSource(parseCav(new Uint8Array(mock.cartBytes)), SEED));

  await page.goto('/remix/demo');
  await expect.poll(() => pixel(page), { timeout: 30_000 }).not.toEqual([0, 0, 0, 0]);
  const before = await pixel(page);

  // No Run, no Ctrl+Enter: the pause alone reruns it.
  await page.getByLabel('Lua source').fill(SEED.replace('= 8', '= 12'));
  await expect.poll(() => pixel(page)).not.toEqual(before);
  await expect(page.getByText("It runs. That's your change on the left.")).toBeVisible();
  await expect(page.getByRole('button', { name: 'Publish my version' })).toBeEnabled();
  const changed = await pixel(page);

  // A half-typed line shows the error and leaves the working build alone.
  await page.getByLabel('Lua source').fill('local COLOR = 3\nfunction _update()\n  fill_screen(COLOR\nend\n');
  await expect(page.getByRole('alert')).toContainText('Line 4');
  await expect.poll(() => pixel(page)).toEqual(changed);
});
