import { test, expect } from '../support/fixtures';

test('home, discovery, detail, history navigation, and 404', async ({ page, mock }, testInfo) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Ember Quest', level: 1 })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Trending this week' })).toBeVisible();
  expect(mock.calls('GET', '/api/v2/carts')).toHaveLength(3);
  expect(mock.calls('GET', '/api/v2/carts')[0].query).toMatchObject({ sort: 'top', per_page: '6' });

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
  const searchCall = mock.calls('GET', '/api/v2/carts').at(-1)!;
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
  const filtered = mock.calls('GET', '/api/v2/carts').at(-1)!;
  expect(filtered.query).toMatchObject({ tag: 'cozy', sort: 'top', page: '0' });
  await page.goto('/browse?q=does-not-exist');
  await expect(page.getByRole('heading', { name: 'Nothing matches that' })).toBeVisible();
});
