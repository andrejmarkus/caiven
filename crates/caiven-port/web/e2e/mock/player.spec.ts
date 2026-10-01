import { test, expect } from '../support/fixtures';

test('repository cart boots shipped WASM, paints canvas, records play, and persists history', async ({ page, mock }, testInfo) => {
  await page.goto('/play/demo');
  const canvas = page.locator('canvas');
  await expect(canvas).toBeVisible({ timeout: 30_000 });
  await expect.poll(() => mock.calls('POST', '/api/v1/carts/demo/play').length, { timeout: 30_000 }).toBe(1);
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

test('volume reaches the game audio and follows the viewer to remix', async ({ page, mock }, testInfo) => {
  test.skip(testInfo.project.name.startsWith('mobile'), 'phones use the hardware volume keys');
  await page.addInitScript(() => {
    const gains: GainNode[] = [];
    (window as unknown as { gains: GainNode[] }).gains = gains;
    const create = AudioContext.prototype.createGain;
    AudioContext.prototype.createGain = function () { const node = create.call(this); gains.push(node); return node; };
  });
  const gain = () => page.evaluate(() => (window as unknown as { gains: GainNode[] }).gains.at(-1)?.gain.value ?? null);
  await page.goto('/play/demo');
  await expect.poll(() => mock.calls('POST', '/api/v1/carts/demo/play').length, { timeout: 30_000 }).toBe(1);
  await page.locator('canvas').click();
  await expect.poll(gain, { timeout: 15_000 }).toBe(1);

  const volume = page.getByRole('slider', { name: 'Volume' });
  await volume.fill('0.25');
  await expect.poll(gain).toBe(0.25);
  await page.getByRole('button', { name: 'Mute' }).click();
  await expect(volume).toBeDisabled();

  mock.carts[0].remixable = true;
  await page.goto('/remix/demo');
  await expect(page.getByRole('button', { name: 'Unmute' })).toBeVisible();
  await page.getByRole('button', { name: 'Unmute' }).click();
  await expect(page.getByRole('slider', { name: 'Volume' })).toHaveValue('0.25');
});
