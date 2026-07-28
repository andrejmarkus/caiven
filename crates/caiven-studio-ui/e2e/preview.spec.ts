import { expect, test } from '@playwright/test';

test('browser preview renders fallback cart without console or page errors', async ({ page }) => {
  const errors: string[] = [];
  page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()); });
  page.on('pageerror', (error) => errors.push(error.message));
  await page.addInitScript(() => {
    localStorage.clear();
    localStorage.setItem('caiven-studio-tour-complete', '1');
  });
  await page.goto('/');
  await expect(page.getByText('catch', { exact: true }).first()).toBeVisible();
  await expect(page.getByText('Preview', { exact: true })).toBeVisible();
  await expect(page.getByLabel('Cart framebuffer')).toBeVisible();
  expect(errors).toEqual([]);
});
