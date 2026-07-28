import { test, expect } from '../support/fixtures';

test('protected route returns after login and logout clears session', async ({ page, mock }) => {
  await page.goto('/settings');
  await expect(page.getByText('Welcome back.')).toBeVisible();
  await page.getByLabel('Username or email').fill('admin');
  await page.getByLabel('Password').fill('GoodPass!1');
  await page.getByRole('button', { name: 'Log in', exact: true }).click();
  await expect(page).toHaveURL('/settings');
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
  const login = mock.calls('POST', '/api/v2/auth/login')[0];
  expect(JSON.parse(login.body!)).toMatchObject({ identifier: 'admin', password: 'GoodPass!1' });

  mock.allowRequestFailure('POST', '/api/v2/auth/logout');
  await page.locator('header').getByRole('button').last().click();
  await page.getByText('Log out').click();
  await expect(page.getByRole('link', { name: 'Log in' })).toBeVisible();
  expect(mock.user).toBeNull();
  expect(mock.calls('POST', '/api/v2/auth/logout')[0].headers['x-csrf-token']).toBe('mock-csrf');
});

test('registration, duplicate conflict, invalid login, MFA, OAuth presentation', async ({ page, mock }) => {
  await page.goto('/register');
  await expect(page.getByRole('button', { name: /github/i })).toBeVisible();
  await page.getByLabel('Username', { exact: true }).fill('new-player');
  await page.getByLabel('Email').fill('new@example.test');
  await page.getByLabel('Password').fill('GoodPass!1');
  await page.getByRole('button', { name: 'Create account' }).click();
  await expect(page).toHaveURL('/');
  expect(mock.users.has('new-player')).toBe(true);

  await page.goto('/login');
  mock.allowStatus(401);
  await page.getByLabel('Username or email').fill('admin');
  await page.getByLabel('Password').fill('wrong');
  await page.getByRole('button', { name: 'Log in', exact: true }).click();
  await expect(page.getByText('Unauthorized')).toBeVisible();

  await page.getByLabel('Username or email').fill('player');
  await page.getByLabel('Password').fill('MfaPass!1');
  await page.getByRole('button', { name: 'Log in', exact: true }).click();
  await expect(page.getByText(/authentication code/i)).toBeVisible();
  await page.getByRole('textbox').last().fill('123456');
  await page.getByRole('button', { name: /verify/i }).click();
  await expect(page).toHaveURL('/');
});
