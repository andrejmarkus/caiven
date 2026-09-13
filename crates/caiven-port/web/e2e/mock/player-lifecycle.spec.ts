import { test, expect } from '../support/fixtures';

test('player releases held input on blur, respects text fields, and cleans up on navigation', async ({ page }) => {
  await page.route('**/wasm/caiven_web.js', (route) => route.fulfill({
    contentType: 'application/javascript',
    body: `
      window.playerCalls = [];
      window.CaivenModule = async () => ({
        HEAPU8: new Uint8Array(192 * 128 * 4), HEAPF32: new Float32Array(65536),
        _malloc: () => 0, _free: () => {},
        ccall: (name, ret, types, args) => {
          window.playerCalls.push([name, ...args]);
          if (name === 'caiven_width') return 192;
          if (name === 'caiven_height') return 128;
          return 0;
        }
      });`,
  }));
  await page.goto('/play/demo');
  await expect.poll(() => page.evaluate(() =>
    (window as any).playerCalls?.some((call: unknown[]) => call[0] === 'caiven_tick'),
  )).toBe(true);

  await page.keyboard.down('ArrowRight');
  await page.evaluate(() => window.dispatchEvent(new Event('blur')));
  const lastRight = await page.evaluate(() => (window as any).playerCalls
    .filter((call: unknown[]) => call[0] === 'caiven_set_button' && call[1] === 3).at(-1));
  expect(lastRight).toEqual(['caiven_set_button', 3, 0]);
  await page.keyboard.up('ArrowRight');

  await page.evaluate(() => {
    const input = document.createElement('input');
    input.id = 'typing-test';
    document.body.append(input);
    input.focus();
    (window as any).playerCalls = [];
  });
  await page.keyboard.type('jwasd');
  await expect(page.locator('#typing-test')).toHaveValue('jwasd');
  expect(await page.evaluate(() => (window as any).playerCalls
    .some((call: unknown[]) => call[0] === 'caiven_set_button' && call[2] === 1))).toBe(false);

  // SPA navigation unmounts the player without resetting the instrumented module.
  await page.getByRole('link', { name: 'Ember Quest', exact: true }).click();
  await expect(page).toHaveURL(/\/cart\/demo$/);
  await page.evaluate(() => { (window as any).playerCalls = []; });
  await page.keyboard.press('ArrowRight');
  expect(await page.evaluate(() => (window as any).playerCalls)).toEqual([]);
});
