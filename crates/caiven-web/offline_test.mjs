import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const require = createRequire(new URL('../caiven-port/web/package.json', import.meta.url));
const { chromium } = require('@playwright/test');
if (!process.argv[2]) throw new Error('Usage: node crates/caiven-web/offline_test.mjs <export.html>');

const browser = await chromium.launch();
try {
  const page = await browser.newPage();
  const errors = [];
  const requests = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  await page.route(/^https?:/, (route) => {
    requests.push(route.request().url());
    return route.abort();
  });
  await page.goto(pathToFileURL(resolve(process.argv[2])).href);
  await page.waitForFunction(() => {
    const canvas = document.getElementById('screen');
    const pixels = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
    return pixels.some((value, i) => i % 4 !== 3 && value !== 0);
  });
  const canvas = page.locator('canvas');
  const bounds = await canvas.boundingBox();
  // Ignore the one-pixel border when measuring console aspect ratio.
  assert.ok(Math.abs((bounds.width - 2) / (bounds.height - 2) - 1.5) < 0.01);
  await canvas.click();
  if (process.argv.includes('--creator-workflow')) {
    const pixel = (x) => page.evaluate((x) => Array.from(document.getElementById('screen')
      .getContext('2d').getImageData(x, 16, 1, 1).data), x);
    assert.deepEqual(await pixel(16), [18, 52, 86, 255], 'saved sprite/palette missing');
    const background = await pixel(40);
    await page.keyboard.down('ArrowRight');
    try {
      await page.waitForFunction(() => {
        const p = document.getElementById('screen').getContext('2d').getImageData(40, 16, 1, 1).data;
        return p[0] === 18 && p[1] === 52 && p[2] === 86;
      });
    } finally {
      await page.keyboard.up('ArrowRight');
    }
    assert.deepEqual(await pixel(16), background, 'input did not move the saved sprite');
  }
  await page.keyboard.press('ArrowRight');
  assert.equal(await page.locator('#status').textContent(), '');
  assert.deepEqual(requests, [], 'offline export attempted a network request');
  assert.deepEqual(errors, [], 'offline export reported browser errors');
  console.log('Offline export: current cart renders at 3:2, input works, no network requests or browser errors');
} finally {
  await browser.close();
}
