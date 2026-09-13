import { test, expect } from '../support/fixtures';

test('admin user moderation updates server state and search results', async ({ page, mock }) => {
  await mock.loginAs();
  await page.goto('/admin/users');
  const player = page.getByRole('row').filter({ hasText: 'player@example.test' });
  await expect(player).toBeVisible();
  await player.getByRole('button', { name: 'Ban', exact: true }).click();
  await player.getByPlaceholder('Reason…').fill('Spam');
  await player.getByRole('button', { name: 'Confirm' }).click();
  await expect(player.getByText('Banned', { exact: true })).toBeVisible();
  expect(mock.users.get('player')?.banned_reason).toBe('Spam');
  await player.getByRole('button', { name: 'Unban' }).click();
  await expect(player.getByRole('button', { name: 'Ban', exact: true })).toBeVisible();
  await player.getByRole('button', { name: 'Promote' }).click();
  await expect(player.getByText('Admin', { exact: true })).toBeVisible();
  await player.getByRole('button', { name: 'Demote' }).click();
  await expect(player.getByRole('button', { name: 'Promote' })).toBeVisible();
  await page.getByPlaceholder('Search username or email…').fill('missing');
  await page.getByRole('button', { name: 'Search', exact: true }).click();
  await expect(page.getByText('No users match this search.')).toBeVisible();
});
