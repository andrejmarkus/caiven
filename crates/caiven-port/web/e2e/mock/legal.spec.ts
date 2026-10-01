import { test, expect } from '../support/fixtures';

test('legal pages name the operator and the footer reaches them', async ({ page }) => {
  await page.goto('/');
  await page.locator('footer').getByRole('link', { name: 'Privacy' }).click();
  await expect(page.getByRole('heading', { name: 'Privacy policy' })).toBeVisible();
  await expect(page.getByText('Test Operator, 1 Test St').first()).toBeVisible();
  await page.goto('/terms');
  await expect(page.getByRole('heading', { name: 'Terms of service' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'abuse@example.test' }).first()).toBeVisible();
});

test('footer Contact scrolls to the contact section from anywhere', async ({ page }) => {
  const contact = page.getByRole('heading', { name: '12. Legal notice and contact' });
  // dispatchEvent clicks without Playwright scrolling the footer (and section) into view first.
  const clickContact = () => page.locator('footer').getByRole('link', { name: 'Contact' }).dispatchEvent('click');
  await page.goto('/');
  await clickContact();
  await expect(page).toHaveURL('/terms#contact');
  await expect(contact).toBeInViewport();

  await page.evaluate(() => window.scrollTo(0, 0));
  await clickContact();
  await expect(contact).toBeInViewport();

  await page.goto('/privacy');
  await page.goto('/terms#contact');
  await expect(contact).toBeInViewport();
});

test('a copyright report sends every DSA/DMCA field', async ({ page, mock }) => {
  await page.goto('/report?category=copyright&url=https%3A%2F%2Fport.test%2Fcart%2Fx');
  await page.getByLabel('Explain why').fill('My game "Orbit", copied without permission.');
  await page.getByLabel(/Your name/).fill('Ada Owner');
  await page.getByLabel(/Your email/).fill('ada@example.test');
  await page.getByRole('checkbox').check();
  await page.getByRole('button', { name: 'Send report' }).click();
  await expect(page.getByText('Your report reached us')).toBeVisible();
  // Short page on a tall screen: the footer links sit at the bottom (above mobile tabs), not mid-page.
  const { width } = page.viewportSize()!;
  await page.setViewportSize({ width, height: 1400 });
  const links = (await page.locator('footer').getByRole('link', { name: 'Terms' }).boundingBox())!;
  expect(links.y).toBeGreaterThan(1400 - 64 - 80);
  expect(JSON.parse(mock.calls('POST', '/api/v1/reports')[0].body!)).toMatchObject({
    url: 'https://port.test/cart/x',
    category: 'copyright',
    name: 'Ada Owner',
    email: 'ada@example.test',
    good_faith: true,
  });
});
