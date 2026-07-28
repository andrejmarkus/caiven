import { test, expect } from '../support/fixtures';

test('repository cart boots shipped WASM, paints canvas, records play, and persists history', async ({ page, mock }, testInfo) => {
  await page.goto('/play/demo');
  const canvas = page.locator('canvas');
  await expect(canvas).toBeVisible({ timeout: 30_000 });
  await expect.poll(() => mock.calls('POST', '/api/v2/carts/demo/play').length, { timeout: 30_000 }).toBe(1);
  await expect.poll(async () => canvas.evaluate((node: HTMLCanvasElement) => {
    const pixels = node.getContext('2d')!.getImageData(0, 0, node.width, node.height).data;
    return pixels.some((value, index) => index % 4 !== 3 && value !== 0);
  }), { timeout: 30_000 }).toBe(true);
  await page.keyboard.press('ArrowRight');
  if (testInfo.project.name.startsWith('mobile')) await expect(page.locator('.touch-btn').first()).toBeVisible();
  await page.getByRole('button', { name: 'Mute' }).click();
  await expect(page.getByRole('button', { name: 'Unmute' })).toBeVisible();
  await page.getByRole('button', { name: 'Restart cart' }).click();
  await page.goto('/library');
  await expect(page.getByText('Ember Quest')).toBeVisible();
});
