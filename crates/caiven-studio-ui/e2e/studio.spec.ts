import { expect, test } from './fixtures';

test('tour, template, folder dialog, and new cart flow', async ({ page, e2e }) => {
  await page.getByTitle('Take guided tour').click();
  for (let step = 1; step <= 4; step += 1) {
    await expect(page.getByRole('dialog', { name: `Tutorial step ${step}` })).toBeVisible();
    await page.getByRole('dialog', { name: `Tutorial step ${step}` }).getByRole('button', { name: step === 4 ? 'Start building' : /Next:/ }).click();
  }
  await page.getByTitle('Start screen').click();
  await e2e.queueDialog('open', '/carts/new-cart');
  await page.getByRole('button', { name: 'New cart' }).click();
  await page.getByRole('button', { name: /Blank/ }).click();
  await page.getByRole('button', { name: 'Choose folder' }).click();
  await expect(page.getByText('new-cart', { exact: true }).first()).toBeVisible();
  expect((await e2e.calls()).some((call) => call.command === 'studio_new_project' && call.args.templateId === 'blank')).toBeTruthy();
});

test('command palette exports a self-contained web build via studio_export_web', async ({ page, e2e }) => {
  await e2e.queueDialog('save', '/carts/test-cart/game.html');
  await page.keyboard.press('Control+K');
  await expect(page.getByPlaceholder('Search or run a command')).toBeVisible();
  await page.getByText('Export to web (.html)', { exact: true }).click();
  await expect(page.getByPlaceholder('Search or run a command')).toHaveCount(0);
  await expect.poll(async () => (await e2e.calls()).some((call) => call.command === 'studio_export_web')).toBeTruthy();
  const call = (await e2e.calls()).find((call) => call.command === 'studio_export_web');
  expect(call?.args.path).toBe('/carts/test-cart/game.html');
});

test('code, runtime, shortcuts, watches, module, drawer, and console flow', async ({ page, e2e }) => {
  const editor = page.locator('.cm-content');
  await editor.click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type('\n-- e2e edit');
  await expect(page.getByTitle('Unsaved changes')).toBeVisible();
  await expect.poll(async () => (await e2e.calls()).some((call) => call.command === 'studio_write_buffer')).toBeTruthy();

  await page.getByTitle('New Lua module').click();
  await page.locator('.module-dialog input').fill('ui/hud.lua');
  await page.getByRole('button', { name: 'Create module' }).click();
  await expect(page.getByText('ui/hud.lua', { exact: true }).first()).toBeVisible();

  await page.getByLabel('Watch expression').fill('score');
  await page.locator('form.add-watch').evaluate((form) => (form as HTMLFormElement).requestSubmit());
  await expect.poll(async () => (await e2e.calls()).some((call) => call.command === 'studio_add_watch')).toBeTruthy();
  await expect(page.getByText('score', { exact: true }).first()).toBeVisible();
  await page.getByRole('tab', { name: 'Globals' }).click();
  await expect(page.getByRole('button', { name: 'Expand player' })).toBeVisible();
  await page.getByRole('button', { name: 'Expand player' }).click();
  await expect.poll(async () => (await e2e.calls()).some((call) => call.command === 'studio_expand_debug_value')).toBeTruthy();
  await expect(page.getByText('x', { exact: true })).toBeVisible();
  await expect(page.getByText('y', { exact: true })).toBeVisible();
  await page.getByRole('tab', { name: 'Watches' }).click();

  await page.keyboard.press('Control+S');
  await expect(page.getByTitle('Unsaved changes')).toHaveCount(0);

  await page.getByRole('button', { name: /^Run/ }).click();
  await expect(page.getByRole('button', { name: /^Pause/ })).toBeVisible();
  await page.keyboard.down('w'); await page.keyboard.up('w');
  await page.keyboard.press('Control+R');
  await expect(page.getByRole('button', { name: /^Run/ })).toBeVisible();
  await page.getByTitle('Step one frame').click();
  await page.getByTitle('Reset').click();

  await page.keyboard.press('Control+K');
  await expect(page.getByPlaceholder('Search or run a command')).toBeVisible();
  await page.keyboard.press('Escape');
  await page.getByRole('tab', { name: /Output/ }).click();
  await expect(page.getByText('mock runtime ready')).toBeVisible();
  await page.getByRole('tab', { name: /Memory/ }).click();
  await expect(page.getByText('RAM', { exact: true })).toBeVisible();
  await page.getByTitle('Hide console').click();
  await expect(page.getByTitle('Show console')).toBeVisible();
  await page.getByTitle('Show console').click();

  const commands = (await e2e.calls()).map((call) => call.command);
  expect(commands).toEqual(expect.arrayContaining(['studio_save', 'studio_transport', 'studio_set_input', 'studio_add_watch', 'studio_create_module']));
});

test('debounced source buffer and saved dirty state survive browser reload', async ({ page, e2e }) => {
  const editor = page.locator('.cm-content');
  await editor.click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type('\n-- survives reload');
  await expect(page.getByTitle('Unsaved changes')).toBeVisible();
  await expect.poll(async () => (await e2e.calls()).some((call) => call.command === 'studio_write_buffer' && String(call.args.text).includes('-- survives reload'))).toBeTruthy();

  await page.reload();
  await expect(page.getByText('test-cart', { exact: true }).first()).toBeVisible();
  await expect(page.locator('.cm-content')).toContainText('-- survives reload');
  await expect(page.getByTitle('Unsaved changes')).toBeVisible();

  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByTitle('Unsaved changes')).toHaveCount(0);
  await page.reload();
  await expect(page.locator('.cm-content')).toContainText('-- survives reload');
  await expect(page.getByTitle('Unsaved changes')).toHaveCount(0);
});

test('art, sound, asset reference, and navigation flow', async ({ page, e2e }) => {
  await page.getByTitle(/^Art/).click();
  await page.getByLabel('Color 8').click();
  await page.getByLabel('8 by 8 sprite grid').click({ position: { x: 10, y: 10 } });
  await page.getByTitle('Flip horizontally').click();
  await page.getByTitle('Undo sprite edit').click();

  await page.getByRole('button', { name: 'Map', exact: true }).click();
  const tilePicker = page.getByLabel(/^Tile picker/);
  const pickerBox = await tilePicker.boundingBox();
  await tilePicker.click({ position: { x: (pickerBox!.width / 16) * 1.5, y: (pickerBox!.height / 16) * 0.5 } });
  await page.getByLabel('64 by 64 tile map').click({ position: { x: 10, y: 10 } });
  await page.getByTitle('Fill (f)').click();
  await page.getByRole('button', { name: 'Collision', exact: true }).click();
  await page.locator('.collision-type-picker select').selectOption({ label: 'solid' });
  await page.getByRole('button', { name: '100%', exact: true }).click();

  await page.getByRole('button', { name: 'Palette', exact: true }).click();
  const hex = page.getByLabel('Hex');
  await hex.fill('#123456'); await hex.blur();
  await expect(page.getByRole('heading', { name: '#123456' })).toBeVisible();

  await page.getByTitle(/^Sound/).click();
  await page.getByLabel(/Note pitch per step/).click({ position: { x: 8, y: 20 } });
  await page.getByRole('button', { name: 'Play', exact: true }).click();
  await page.getByRole('button', { name: 'Music', exact: true }).click();
  await page.locator('.music-grid button').first().click();
  await page.keyboard.press('Space');

  await page.getByTitle(/^Assets/).click();
  await expect(page.getByRole('heading', { name: 'Assets' })).toBeVisible();
  await page.getByRole('button', { name: 'main.lua:2' }).click();
  await expect(page.getByText('main.lua', { exact: true }).first()).toBeVisible();
  const commands = (await e2e.calls()).map((call) => call.command);
  expect(commands).toEqual(expect.arrayContaining(['studio_write_sprite', 'studio_write_map_cells', 'studio_write_palette', 'studio_write_memory', 'studio_audio_transport']));
});

test('project, library, Port account, download, and publish flow', async ({ page, e2e }) => {
  await page.getByTitle(/^Cart/).click();
  await expect(page.getByRole('heading', { name: 'Cart details' })).toBeVisible();
  const description = page.getByLabel('Description');
  await description.fill('Updated by E2E'); await description.blur();
  await page.getByPlaceholder('Add tag…').fill('test');
  await page.getByPlaceholder('Add tag…').press('Enter');
  await page.getByRole('button', { name: 'Save', exact: true }).click();

  await page.getByTitle(/^Library/).click();
  await e2e.queueDialog('open', '/library');
  await page.getByRole('button', { name: /Scan folder/ }).first().click();
  await expect(page.getByText('Moon', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Port', exact: true }).click();
  await expect(page.getByText('Moon', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: /Moon by maker/ }).click();
  await expect(page.getByText('/downloads/moon', { exact: true }).first()).toBeVisible();

  await page.getByTitle(/^Account/).click();
  await page.getByRole('button', { name: 'Link Port account' }).click();
  await expect(page.getByText('Finish linking in Port')).toBeVisible();
  await expect(page.getByText('tester', { exact: true })).toBeVisible({ timeout: 3_000 });

  await page.getByRole('button', { name: 'Publish', exact: true }).click();
  await page.getByPlaceholder('What changed?').fill('E2E release');
  await page.locator('.publish-dialog').getByRole('button', { name: 'Publish', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Cart shipped' })).toBeVisible();
  await expect(page.getByText('cart-1 · v3')).toBeVisible();

  const commands = (await e2e.calls()).map((call) => call.command);
  expect(commands).toEqual(expect.arrayContaining(['studio_write_meta', 'studio_scan_library', 'port_list_carts', 'port_download', 'port_link_start', 'port_link_poll', 'studio_port_publish']));
});

test('navigation reaches every top-level Studio screen', async ({ page, e2e: _e2e }) => {
  const destinations = [
    ['Start', 'Make small worlds.'], ['Code', 'Project'], ['Art', 'Sprite'], ['Sound', 'Sound effects'],
    ['Assets', 'Assets'], ['Cart', 'Cart details'], ['Library', 'Library'], ['Account', 'Account'], ['Docs', 'API reference'],
  ];
  for (const [title, text] of destinations) {
    await page.getByTitle(new RegExp(`^${title}`)).click();
    await expect(page.getByText(text).first()).toBeVisible();
  }
});

test('discard cancellation keeps dirty cart open', async ({ page, e2e }) => {
  await page.getByTitle(/^Cart/).click();
  const titleInput = page.getByLabel('Title');
  await titleInput.fill('dirty title'); await titleInput.blur();
  await page.getByTitle('Start screen').click();
  await e2e.queueDialog('open', '/carts/blocked');
  page.once('dialog', (dialog) => dialog.dismiss());
  await page.getByRole('button', { name: 'Open project' }).click();
  expect((await e2e.calls()).some((call) => call.command === 'studio_open_project')).toBeFalsy();
  await expect(page.getByText('dirty title', { exact: true }).first()).toBeVisible();
});

test('failed asset mutation restores prior visible and mock state', async ({ page, e2e }) => {
  await page.getByTitle(/^Art/).click();
  await page.getByRole('button', { name: 'Palette', exact: true }).click();
  await page.getByRole('button', { name: /^00 #000000/ }).click();
  await e2e.failNext('studio_write_palette', 'readonly cart');
  const hex = page.getByLabel('Hex'); await hex.fill('#ABCDEF'); await hex.blur();
  await expect(page.getByText('Palette slot 00 failed: readonly cart')).toBeVisible();
  await expect(page.getByRole('heading', { name: '#000000' })).toBeVisible();
  expect((await e2e.snapshot() as any).banks.palette['0'].slice(0, 3)).toEqual([0, 0, 0]);
});

test('invalid module stays open with actionable error', async ({ page, e2e: _e2e }) => {
  await page.getByTitle(/^Code/).click();
  await page.getByTitle('New Lua module').click();
  await page.locator('.module-dialog input').fill('bad.txt');
  await page.getByRole('button', { name: 'Create module' }).click();
  await expect(page.getByRole('alert')).toContainText('Module name must end in .lua');
  await expect(page.locator('.module-dialog input')).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.locator('.module-dialog')).toHaveCount(0);
});

test('save failure keeps edited source dirty', async ({ page, e2e }) => {
  await page.locator('.cm-content').click(); await page.keyboard.type('--dirty');
  await e2e.failNext('studio_save', 'disk full');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByText('Save failed: disk full')).toBeVisible();
  await expect(page.getByTitle('Unsaved changes')).toBeVisible();
});

test('frame polling runs on the console/cart screens and pauses elsewhere', async ({ page, e2e }) => {
  // Default landing screen is 'code', which shows the live console preview —
  // frame polling must already be running here, not just on the 'cart' screen.
  await expect.poll(async () => (await e2e.calls()).some((call) => call.command === 'studio_frame')).toBe(true);

  await page.keyboard.press('F7');
  await expect(page.getByText('Port preview')).toBeVisible();
  await expect.poll(async () => (await e2e.calls()).some((call) => call.command === 'studio_frame')).toBe(true);

  // 'docs' has no console preview and no cart cover-art — polling should stop there.
  await page.keyboard.press('F9');
  const callsAfterLeaving = (await e2e.calls()).filter((call) => call.command === 'studio_frame').length;
  await page.waitForTimeout(300);
  const callsLater = (await e2e.calls()).filter((call) => call.command === 'studio_frame').length;
  expect(callsLater).toBe(callsAfterLeaving);
});

test('tick and state polling do not overlap when a round-trip runs long', async ({ page, e2e }) => {
  await page.waitForTimeout(300);
  const baseline = (await e2e.calls()).filter((call) => call.command === 'studio_tick').length;

  // One slow studio_tick response (much longer than the 120ms poll interval) must not cause
  // a pile-up of concurrent in-flight requests — the poller should skip ticks until it
  // resolves. Wide margins (3s delay, sampled well clear of both ends) keep this robust
  // against normal test-runner scheduling jitter.
  await e2e.delayNext('studio_tick', 3000);
  await page.waitForTimeout(200); // let the slow call actually start before measuring
  const duringSlowCall = (await e2e.calls()).filter((call) => call.command === 'studio_tick').length;
  await page.waitForTimeout(2500); // still well within the 3s delay window
  const stillDuringSlowCall = (await e2e.calls()).filter((call) => call.command === 'studio_tick').length;
  expect(stillDuringSlowCall).toBe(duringSlowCall);
  expect(duringSlowCall).toBeGreaterThan(baseline);

  await expect.poll(async () => (await e2e.calls()).filter((call) => call.command === 'studio_tick').length)
    .toBeGreaterThan(stillDuringSlowCall);
});

test('Port unreachable, expired session, and publish failure stay actionable', async ({ page, e2e, errorGuard }) => {
  errorGuard.allow(/port request failed:/);
  await page.getByTitle(/^Library/).click();
  await e2e.failNext('port_list_carts', 'Connection Failed: Connect error');
  await page.getByRole('button', { name: 'Port', exact: true }).click();
  await expect(page.getByText(/Can’t reach http:\/\/port\.test/)).toBeVisible();

  await e2e.failNext('port_list_carts', '401 unauthorized');
  await page.getByRole('button', { name: 'Retry' }).click();
  await expect(page.getByText('Your port session has expired. Log in again.')).toBeVisible();

  await page.getByTitle(/^Account/).click();
  await page.getByRole('button', { name: 'Link Port account' }).click();
  await expect(page.getByText('tester', { exact: true })).toBeVisible({ timeout: 3_000 });
  await e2e.failNext('studio_port_publish', 'upload rejected');
  await page.getByRole('button', { name: 'Publish', exact: true }).click();
  await page.locator('.publish-dialog').getByRole('button', { name: 'Publish', exact: true }).click();
  await expect(page.locator('.publish-dialog')).toContainText('upload rejected');
  await expect(page.getByRole('heading', { name: 'Publishing to port' })).toBeVisible();
});
