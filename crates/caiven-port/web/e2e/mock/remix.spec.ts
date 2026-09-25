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
  await expect(page.getByText("It runs. That's your change in the game.")).toBeVisible();

  // A broken edit keeps the last working build running and marks the line.
  await setSource(page, 'local COLOR = 3\nfunction _update()\n  fill_screen(COLOR\nend\n');
  await expect(page.getByRole('alert')).toContainText('Line 4');
  await expect(page.getByTestId('error-line')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Publish my version' })).toBeDisabled();
  await expect.poll(() => pixel(page)).toEqual(changed);

  await setSource(page, SEED.replace('= 8', '= 12'));
  await expect(page.getByRole('alert')).toHaveCount(0);

  // Anonymous publish goes through sign-up and comes back with the edit intact.
  await page.getByRole('button', { name: 'Publish my version' }).click();
  await expect(page).toHaveURL(/\/register\?next=/);
  await expect(page.getByTestId('remix-saved')).toContainText('Your remix is saved');
  const opened = () => mock.calls('POST', '/api/v2/carts/demo/funnel').filter((c) => c.body?.includes('remix_opened')).length;
  await expect.poll(opened).toBe(1);
  await mock.loginAs('player');
  await page.goto(decodeURIComponent(new URL(page.url()).searchParams.get('next')!));
  await expect(page.getByLabel('Lua source')).toHaveValue(SEED.replace('= 8', '= 12'));
  await expect(page.getByText('Restored your saved edit')).toBeVisible();
  const form = page.getByRole('form', { name: 'Publish your remix' });
  await expect(form).toBeVisible();
  await form.getByLabel('Title').fill('Ember Quest, but teal');
  await form.getByRole('button', { name: /Publish as @player/ }).click();

  await expect(page.getByText("Published. It's yours now.")).toBeVisible();
  // Coming back logged in is the same person, not a second remixer.
  expect(opened()).toBe(1);
  expect(mock.calls('POST', '/api/v2/carts/demo/funnel').filter((c) => c.body?.includes('publish_started'))).toHaveLength(1);
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
  await expect(page.getByText("It runs. That's your change in the game.")).toBeVisible();
  await expect(page.getByRole('button', { name: 'Publish my version' })).toBeEnabled();
  const changed = await pixel(page);

  // A half-typed line shows the error and leaves the working build alone.
  await page.getByLabel('Lua source').fill('local COLOR = 3\nfunction _update()\n  fill_screen(COLOR\nend\n');
  await expect(page.getByRole('alert')).toContainText('Line 4');
  await expect.poll(() => pixel(page)).toEqual(changed);
});

test('a shared remix shows its lineage and invites the next remix', async ({ page, mock }) => {
  mock.carts[0].remixable = true;
  Object.assign(mock.carts[1], {
    remixable: true, parent_cart_id: 'demo', root_cart_id: 'demo', owner: 'player', description: 'Doubled the speed',
  });

  await page.goto('/play/orbit');
  const lineage = page.getByTestId('remix-lineage');
  await expect(lineage).toContainText('@player remixed Ember Quest by @admin');
  await expect(lineage).toContainText('Doubled the speed');
  await expect(page.getByTestId('remix-invite')).toContainText('Your turn');

  // The original counts the remix and still invites another.
  await lineage.getByRole('link', { name: 'Ember Quest' }).click();
  await expect(page).toHaveURL(/\/play\/demo$/);
  await expect(page.getByTestId('remix-lineage')).toHaveCount(0);
  await expect(page.getByTestId('remix-invite')).toContainText('1 remix so far');
  await page.getByTestId('remix-invite').getByRole('link', { name: 'Remix it' }).click();
  await expect(page).toHaveURL(/\/remix\/demo$/);
});

test('home and browse surface new and most remixed carts', async ({ page, mock }) => {
  // Before any remix, neither row shows.
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Trending this week' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'New remixes' })).toHaveCount(0);

  mock.carts[0].remixable = true;
  Object.assign(mock.carts[1], { parent_cart_id: 'demo', root_cart_id: 'demo', owner: 'player' });
  await page.reload();
  const fresh = page.locator('section', { has: page.getByRole('heading', { name: 'New remixes' }) });
  await expect(fresh.getByText('Tiny Orbit')).toBeVisible();
  await expect(fresh.getByText('Ember Quest')).toHaveCount(0);
  const most = page.locator('section', { has: page.getByRole('heading', { name: 'Most remixed' }) });
  await expect(most.getByText('Ember Quest')).toBeVisible();

  await most.getByRole('link', { name: 'See all' }).click();
  await expect(page).toHaveURL(/\/browse\?.*sort=remixed/);
  await expect(page.getByText('Ember Quest').first()).toBeVisible();
  await expect(page.getByText('Pocket Garden')).toHaveCount(0);
});

const SEED_TRY = 'local COLOR = 8  -- try 12\nfunction _update()\n  fill_screen(COLOR)\nend\n';

test('an unconfirmed email keeps the remix, and the email link returns to Publish', async ({ page, mock }) => {
  mock.carts[0].remixable = true;
  mock.cartBytes = Buffer.from(withLuaSource(parseCav(new Uint8Array(mock.cartBytes)), SEED_TRY));
  const player = mock.users.get('player')!;
  player.email_verified = false;
  await mock.loginAs('player');

  await page.goto('/remix/demo');
  await expect.poll(() => pixel(page), { timeout: 30_000 }).not.toEqual([0, 0, 0, 0]);
  const before = await pixel(page);

  // The author's "try" value is one click away.
  await page.getByRole('button', { name: 'Try COLOR = 12' }).click();
  await expect(page.getByLabel('Lua source')).toHaveValue(SEED_TRY.replace('= 8', '= 12'));
  await expect.poll(() => pixel(page)).not.toEqual(before);
  await expect(page.getByRole('button', { name: 'Try COLOR = 12' })).toHaveCount(0);

  await page.getByRole('button', { name: 'Publish my version' }).click();
  const form = page.getByRole('form', { name: 'Publish your remix' });
  await expect(form.getByTestId('verify-notice')).toContainText('player@example.test');
  mock.fault({ method: 'POST', path: '/api/v2/carts', status: 403, body: { error: 'Forbidden' }, once: true });
  await form.getByRole('button', { name: /Publish as @player/ }).click();
  await expect(form.getByRole('alert')).toContainText("isn't confirmed yet");

  // The confirmation link, opened in this browser, lands back on the form.
  player.email_verified = true;
  await page.goto('/verify-email?token=mock-token');
  await expect(page).toHaveURL(/\/remix\/demo\?publish=1$/);
  await expect(page.getByLabel('Lua source')).toHaveValue(SEED_TRY.replace('= 8', '= 12'));
  await expect(form).toBeVisible();
  await expect(form.getByTestId('verify-notice')).toHaveCount(0);
  await form.getByRole('button', { name: /Publish as @player/ }).click();
  await expect(page.getByText("Published. It's yours now.")).toBeVisible();

  // Nothing is pending any more, so a later confirmation stays put.
  await page.goto('/verify-email?token=mock-token');
  await expect(page.getByText('Your email is confirmed.')).toBeVisible();
  await expect(page).toHaveURL(/\/verify-email/);
});

test('social sign-in from the remix wall carries the way back', async ({ page }) => {
  await page.route('**/api/v2/auth/oauth/**', (route) => route.fulfill({ status: 200, contentType: 'text/plain', body: 'provider' }));
  await page.goto(`/register?next=${encodeURIComponent('/remix/demo?publish=1')}`);
  await expect(page.getByTestId('remix-saved')).toBeVisible();
  const start = page.waitForRequest('**/api/v2/auth/oauth/github/start**');
  await page.getByRole('button', { name: 'Continue with GitHub' }).click();
  expect(new URL((await start).url()).searchParams.get('next')).toBe('/remix/demo?publish=1');
});

test('Space presses the A button', async ({ page, mock }) => {
  mock.cartBytes = Buffer.from(withLuaSource(parseCav(new Uint8Array(mock.cartBytes)), 'function _update()\n  fill_screen(button_down(4) and 12 or 8)\nend\n'));
  await page.goto('/play/demo');
  await expect.poll(() => pixel(page), { timeout: 30_000 }).not.toEqual([0, 0, 0, 0]);
  const idle = await pixel(page);
  await page.locator('canvas').click();
  await page.keyboard.down(' ');
  await expect.poll(() => pixel(page)).not.toEqual(idle);
  await page.keyboard.up(' ');
  await expect.poll(() => pixel(page)).toEqual(idle);
});
