import { test, expect } from '../support/fixtures';
import { readFile } from 'node:fs/promises';

test.beforeEach(async ({ mock }) => { await mock.loginAs('admin'); });

test('publish cart and version sends multipart plus CSRF', async ({ page, mock }) => {
  const bytes = await readFile('../../../carts/dev/smoke.cav');
  await page.goto('/upload');
  await page.locator('input[type=file]').setInputFiles({ name: 'smoke.cav', mimeType: 'application/octet-stream', buffer: bytes });
  await page.getByLabel('Title').fill('Published Smoke');
  await page.getByLabel('Short description').fill('E2E upload');
  await page.getByLabel('Tags').fill('smoke, test');
  await page.getByRole('button', { name: 'Publish cart' }).click();
  await expect(page).toHaveURL(/\/cart\/uploaded-/);
  await expect(page.getByRole('heading', { name: 'Uploaded Cart' })).toBeVisible();
  const upload = mock.calls('POST', '/api/v2/carts')[0];
  expect(upload.headers['x-csrf-token']).toBe('mock-csrf');
  expect(upload.headers['content-type']).toContain('multipart/form-data');

  await page.goto('/upload?cart=demo');
  await page.locator('input[type=file]').setInputFiles({ name: 'v2.cav', mimeType: 'application/octet-stream', buffer: bytes });
  await page.getByLabel('Changelog').fill('Second version');
  await page.getByRole('button', { name: 'Publish version' }).click();
  await expect(page).toHaveURL('/cart/demo');
  expect(mock.carts.find((x) => x.id === 'demo')?.latest_version).toBe(2);
});

test('comments, rating, collection, jam, and dashboard mutate state', async ({ page, mock }) => {
  await page.goto('/cart/demo');
  await page.getByRole('button', { name: 'Comments' }).click();
  await page.getByPlaceholder('Add a comment…').fill('Stateful comment');
  await page.getByRole('button', { name: 'Post comment' }).click();
  await expect(page.getByText('Stateful comment')).toBeVisible();
  expect(mock.comments.get('demo')?.at(-1)?.body).toBe('Stateful comment');
  expect(mock.calls('POST', '/api/v2/carts/demo/comments')[0].headers['x-csrf-token']).toBe('mock-csrf');

  await page.goto('/collections');
  await page.getByRole('button', { name: 'New collection' }).click();
  await page.getByLabel('Title').fill('My Shelf');
  await page.getByLabel('Description').fill('Favorites');
  await page.getByRole('button', { name: 'Create shelf' }).click();
  await expect(page).toHaveURL('/collections/my-shelf');
  expect(mock.collections.some((x) => x.slug === 'my-shelf')).toBe(true);

  await page.goto('/jams/one-button');
  await expect(page.getByRole('heading', { name: 'One Button Jam' })).toBeVisible();
  await page.goto('/dashboard');
  await expect(page.getByRole('heading', { name: 'Creator stats' })).toBeVisible();
});
