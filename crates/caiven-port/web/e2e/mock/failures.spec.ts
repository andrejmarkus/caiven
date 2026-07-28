import { test, expect } from '../support/fixtures';

test('server error is visible and failed mutation leaves state unchanged', async ({ page, mock }) => {
  mock.fault({ method: 'GET', path: '/api/v2/carts', status: 500, body: { error: 'Database unavailable' }, once: true });
  await page.goto('/browse');
  await expect(page.getByText('Database unavailable')).toBeVisible();

  await mock.loginAs('admin');
  const before = mock.comments.get('demo')?.length;
  mock.fault({ method: 'POST', path: '/api/v2/carts/demo/comments', status: 429, body: { error: 'Slow down' }, once: true });
  await page.goto('/cart/demo');
  await page.getByRole('button', { name: 'Comments' }).click();
  await page.getByPlaceholder('Add a comment…').fill('Must not persist');
  await page.getByRole('button', { name: 'Post comment' }).click();
  await expect(page.getByText('Slow down')).toBeVisible();
  expect(mock.comments.get('demo')?.length).toBe(before);
});

test('malformed JSON and delayed response race show latest route result', async ({ page, mock }) => {
  mock.fault({ method: 'GET', path: '/api/v2/tags', malformed: true, once: true });
  await page.goto('/tags');
  await expect(page.getByText(/Unexpected|JSON|position/i)).toBeVisible();

  mock.fault({ method: 'GET', path: '/api/v2/carts', delayMs: 150, body: { carts: [], total: 0, page: 0, per_page: 20 }, once: true });
  await page.goto('/browse?q=slow');
  await page.goto('/browse?q=orbit');
  await expect(page.getByText('Tiny Orbit').first()).toBeVisible();
  await expect(page).toHaveURL(/q=orbit/);
});
