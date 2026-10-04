import { test, expect } from '../support/fixtures';

test('home, discovery, detail, history navigation, and 404', async ({ page, mock }, testInfo) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Ember Quest', level: 1 })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Trending this week' })).toBeVisible();
  expect(mock.calls('GET', '/api/v1/carts')).toHaveLength(5);
  expect(mock.calls('GET', '/api/v1/carts')[0].query).toMatchObject({ sort: 'top', per_page: '6' });

  if (testInfo.project.name.startsWith('desktop')) {
    await expect(page.getByPlaceholder('Search carts, creators, tags…')).toBeVisible();
    await page.getByPlaceholder('Search carts, creators, tags…').fill('orbit');
    await page.getByPlaceholder('Search carts, creators, tags…').press('Enter');
  } else {
    await expect(page.getByRole('navigation').getByText('Browse')).toBeVisible();
    await page.getByRole('navigation').getByText('Browse').click();
    await page.goto('/browse?q=orbit&sort=new');
  }
  await expect(page).toHaveURL(/\/browse\?.*q=orbit/);
  await expect(page.getByText('Tiny Orbit').first()).toBeVisible();
  const searchCall = mock.calls('GET', '/api/v1/carts').at(-1)!;
  expect(searchCall.query.q).toBe('orbit');

  await page.goto('/cart/demo');
  await expect(page.getByRole('heading', { name: 'Ember Quest' })).toBeVisible();
  await page.getByRole('button', { name: /Versions/ }).click();
  await expect(page.getByText('First release')).toBeVisible();
  await page.goto('/author/admin');
  await expect(page.getByRole('heading', { name: /admin/i })).toBeVisible();
  await page.goBack();
  await expect(page.getByRole('heading', { name: 'Ember Quest' })).toBeVisible();
  await page.goForward();
  await expect(page).toHaveURL('/author/admin');

  await page.goto('/nowhere/deep');
  await expect(page.getByRole('heading', { name: 'Page not found' })).toBeVisible();
});

test('filters and empty results preserve query contract', async ({ page, mock }) => {
  await page.goto('/browse?tag=cozy&sort=top&page=0');
  await expect(page.getByText('Pocket Garden').first()).toBeVisible();
  const filtered = mock.calls('GET', '/api/v1/carts').at(-1)!;
  expect(filtered.query).toMatchObject({ tag: 'cozy', sort: 'top', page: '0' });
  await page.goto('/browse?q=does-not-exist');
  await expect(page.getByRole('heading', { name: 'Nothing matches that' })).toBeVisible();
});

test('search bar previews matches and opens one with the keyboard', async ({ page, mock }, testInfo) => {
  test.skip(!testInfo.project.name.startsWith('desktop'), 'top-bar search is desktop-only');
  await page.goto('/');
  const search = page.getByRole('combobox');
  await search.fill('orb');
  await expect(page.getByRole('option', { name: /Tiny Orbit/ })).toBeVisible();
  expect(mock.calls('GET', '/api/v1/carts').at(-1)!.query).toMatchObject({ q: 'orb', per_page: '5' });
  await search.press('Escape');
  await expect(page.getByRole('listbox')).toBeHidden();
  await search.press('ArrowDown');
  await search.press('Enter');
  await expect(page).toHaveURL(/\/cart\/orbit$/);
});

test('Add to Home Screen metadata resolves to real icons', async ({ page }) => {
  await page.goto('/');
  const head = await page.evaluate(() => ({
    manifest: document.querySelector<HTMLLinkElement>('link[rel=manifest]')!.href,
    touchIcon: document.querySelector<HTMLLinkElement>('link[rel=apple-touch-icon]')!.href,
  }));
  const manifest = await (await page.request.get(head.manifest)).json();
  expect(manifest.display).toBe('standalone');
  for (const url of [head.touchIcon, ...manifest.icons.map((icon: { src: string }) => new URL(icon.src, head.manifest).href)]) {
    const res = await page.request.get(url);
    expect(res.status(), url).toBe(200);
    expect(res.headers()['content-type']).toContain('image/png');
  }
});

test('narrow phone pages never scroll sideways and the player fits above the tab bar', async ({ page, mock }, testInfo) => {
  test.skip(!testInfo.project.name.startsWith('mobile'), 'phone layout');
  await mock.loginAs('admin');
  mock.carts[0].remixable = true;
  await page.setViewportSize({ width: 360, height: 740 });
  for (const [path, ready] of [['/browse', 'heading'], ['/collections/staff-picks', 'heading'], ['/dashboard', 'heading'], ['/remix/demo', 'Lua source']]) {
    await page.goto(path);
    await expect(ready === 'heading' ? page.getByRole('heading', { level: 1 }) : page.getByLabel(ready)).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth), path).toBeLessThanOrEqual(360);
  }
  await page.goto('/play/demo');
  await expect(page.getByRole('link', { name: 'Remix this' })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth), '/play/demo').toBeLessThanOrEqual(360);
  const controls = await page.getByTestId('controls').boundingBox();
  const tabs = await page.locator('nav.fixed').boundingBox();
  expect(controls!.y + controls!.height).toBeLessThanOrEqual(tabs!.y);

  await page.setViewportSize({ width: 360, height: 600 });
  await page.goto('/cart/demo');
  await page.getByRole('button', { name: 'Add to collection' }).click();
  const picker = page.getByText('No available owned collections.');
  const tabTop = (await page.locator('nav.fixed').boundingBox())!.y;
  await expect.poll(async () => { const box = await picker.boundingBox(); return box ? box.y + box.height : Infinity; }).toBeLessThanOrEqual(tabTop);
});

test('home hero runs the featured cart live and plays it in place', async ({ page }) => {
  test.skip(test.info().project.name === 'mobile-chromium', 'touch taps open the full player instead');
  await page.goto('/');
  const screen = page.getByLabel(/Ember Quest, running/);
  await expect(page.getByText('Click the screen to play it here.')).toBeVisible();
  await screen.click();
  await expect(screen).toBeFocused();
  await expect(page.getByText('Arrows move. Z and X are the A and B buttons.')).toBeVisible();
});

test('home hero stays a still screenshot under reduced motion', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Ember Quest', level: 1 })).toBeVisible();
  await expect(page.getByText(/(Click|Tap) the screen to play/)).toHaveCount(0);
});

test('a still home hero opens the full player on click', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await page.locator('canvas[aria-label="Play Ember Quest"]').click();
  await expect(page).toHaveURL(/\/play\/demo$/);
});
