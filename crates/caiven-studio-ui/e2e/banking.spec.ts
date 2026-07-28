import { expect, test, type BankKind } from './fixtures';
import type { Page } from '@playwright/test';

const kinds: BankKind[] = ['sprites', 'map', 'palette', 'sfx', 'music'];

async function openBankEditor(page: Page, kind: BankKind) {
  if (kind === 'sprites' || kind === 'map' || kind === 'palette') {
    await page.getByTitle(/^Art/).click();
    if (kind !== 'sprites') await page.getByRole('button', { name: kind === 'map' ? 'Map' : 'Palette', exact: true }).click();
  } else {
    await page.getByTitle(/^Sound/).click();
    if (kind === 'music') await page.getByRole('button', { name: 'Music', exact: true }).click();
  }
  await expect(page.locator('.bank-picker select')).toBeVisible();
}

for (const kind of kinds) {
  test(`${kind} banks create, select, restore, and delete`, async ({ page, e2e }) => {
    await openBankEditor(page, kind);
    const picker = page.locator('.bank-picker select');
    const deleteButton = page.getByTitle(new RegExp(`^Delete ${kind} bank`));

    await expect(picker).toHaveValue('0');
    await expect(deleteButton).toBeDisabled();
    await page.getByTitle(`Create ${kind} bank`).click();
    await expect(picker).toHaveValue('2');

    await picker.selectOption('1');
    await expect(picker).toHaveValue('1');
    const selected = await e2e.snapshot() as any;
    expect(selected.active[kind]).toBe(1);
    expect(selected.banks[kind]['1'][0]).toBe(kind === 'sprites' ? 3 : kind === 'map' ? 9 : kind === 'palette' ? 0 : kind === 'sfx' ? 55 : 8);

    page.once('dialog', (dialog) => dialog.accept());
    await deleteButton.click();
    await expect(picker).toHaveValue('0');
    await expect(deleteButton).toBeDisabled();

    const finalState = await e2e.snapshot() as any;
    expect(finalState.banks[kind]['1']).toBeUndefined();
    expect(finalState.assetIndexReads).toBeGreaterThanOrEqual(3);
    expect(finalState.cartSizeReads).toBeGreaterThanOrEqual(3);
    const bankCalls = (await e2e.calls()).filter((call) => call.command === 'studio_asset_bank' && call.args.kind === kind);
    expect(bankCalls.map((call) => call.args.action)).toEqual(expect.arrayContaining(['create', 'select', 'delete']));
  });
}

test('sprite selection restores companion flags and palette converts RGB bytes', async ({ page, e2e: _e2e }) => {
  await openBankEditor(page, 'sprites');
  const picker = page.locator('.bank-picker select');
  await picker.selectOption('1');
  await expect(page.getByText('flags = 0x02')).toBeVisible({ timeout: 2_000 });
  await picker.selectOption('0');
  await expect(page.getByText('flags = 0x01')).toBeVisible({ timeout: 2_000 });

  await page.getByRole('button', { name: 'Palette', exact: true }).click();
  await page.locator('.bank-picker select').selectOption('1');
  await page.getByRole('button', { name: /^00 #00FF00/ }).click();
  await expect(page.getByRole('heading', { name: '#00FF00' })).toBeVisible();
});

test('runtime ticks refresh all visible bank editors', async ({ page, e2e }) => {
  for (const kind of kinds) {
    await openBankEditor(page, kind);
    await e2e.setTickBanks({ [kind]: 1 });
    await expect(page.locator('.bank-picker select')).toHaveValue('1', { timeout: 2_000 });
    const calls = await e2e.calls();
    expect(calls.some((call) => call.command === 'studio_asset_bank' && call.args.kind === kind && call.args.action === 'read')).toBeTruthy();
    await e2e.setTickBanks({ [kind]: 0 });
    await expect(page.locator('.bank-picker select')).toHaveValue('0', { timeout: 2_000 });
  }
});

test('bank create, select, and delete failures preserve active data and report toast', async ({ page, e2e }) => {
  await openBankEditor(page, 'palette');
  await e2e.failNext('studio_asset_bank:palette:create', 'create denied');
  await page.getByTitle('Create palette bank').click();
  await expect(page.getByText('Bank create failed: create denied')).toBeVisible();
  await expect(page.locator('.bank-picker select')).toHaveValue('0');

  await e2e.failNext('studio_asset_bank:palette:select', 'disk denied');
  await page.locator('.bank-picker select').selectOption('1');
  await expect(page.getByText('Bank select failed: disk denied')).toBeVisible();
  await expect(page.locator('.bank-picker select')).toHaveValue('0');
  await page.getByRole('button', { name: /^00 #000000/ }).click();
  await expect(page.getByRole('heading', { name: '#000000' })).toBeVisible();
  expect((await e2e.snapshot() as any).active.palette).toBe(0);

  await page.locator('.bank-picker select').selectOption('1');
  await e2e.failNext('studio_asset_bank:palette:delete', 'delete denied');
  page.once('dialog', (dialog) => dialog.accept());
  await page.getByTitle('Delete palette bank 1').click();
  await expect(page.getByText('Bank delete failed: delete denied')).toBeVisible();
  await expect(page.locator('.bank-picker select')).toHaveValue('1');
  expect((await e2e.snapshot() as any).active.palette).toBe(1);
});

test('latest bank selection wins when an older response arrives late', async ({ page, e2e }) => {
  await openBankEditor(page, 'palette');
  const picker = page.locator('.bank-picker select');
  await e2e.delayNext('studio_asset_bank:palette:select', 300);

  await picker.selectOption('1');
  await expect.poll(async () => (await e2e.calls()).filter((call) => call.command === 'studio_asset_bank' && call.args.action === 'select').length).toBe(1);
  await picker.selectOption('0');

  await expect(picker).toHaveValue('0');
  await page.waitForTimeout(350);
  await expect(picker).toHaveValue('0');
  expect((await e2e.snapshot() as any).active.palette).toBe(0);
  await page.getByRole('button', { name: /^00 #000000/ }).click();
  await expect(page.getByRole('heading', { name: '#000000' })).toBeVisible();
});
