import { expect, test, type BankKind } from './fixtures';
import type { Page } from '@playwright/test';

const kinds: BankKind[] = ['sprites', 'map', 'palette', 'sfx', 'music'];

// Bank naming uses Studio's own dialog — native window.prompt() never shows
// in the Tauri webview, so there's no native dialog to intercept anymore.
async function createBankViaDialog(page: Page, kind: BankKind, name: string) {
  await page.getByTitle(`Create ${kind} bank`).click();
  await page.locator('.module-dialog input').fill(name);
  await page.getByRole('button', { name: 'Create', exact: true }).click();
}

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

    await expect(picker).toHaveValue('default');
    await expect(deleteButton).toBeDisabled();
    await createBankViaDialog(page, kind, 'third');
    await expect(picker).toHaveValue('third');

    await picker.selectOption('second');
    await expect(picker).toHaveValue('second');
    const selected = await e2e.snapshot() as any;
    expect(selected.active[kind]).toBe('second');
    expect(selected.banks[kind]['second'][0]).toBe(kind === 'sprites' ? 3 : kind === 'map' ? 9 : kind === 'palette' ? 0 : kind === 'sfx' ? 55 : 8);

    await deleteButton.click();
    await expect(picker).toHaveValue('default');
    await expect(deleteButton).toBeDisabled();

    const finalState = await e2e.snapshot() as any;
    expect(finalState.banks[kind]['second']).toBeUndefined();
    expect(finalState.assetIndexReads).toBeGreaterThanOrEqual(3);
    expect(finalState.cartSizeReads).toBeGreaterThanOrEqual(3);
    const bankCalls = (await e2e.calls()).filter((call) => call.command === 'studio_asset_bank' && call.args.kind === kind);
    expect(bankCalls.map((call) => call.args.action)).toEqual(expect.arrayContaining(['create', 'select', 'delete']));
  });
}

test('sprite bank selection and palette converts RGB bytes', async ({ page, e2e: _e2e }) => {
  await openBankEditor(page, 'sprites');
  const picker = page.locator('.bank-picker select');
  await picker.selectOption('second');
  await expect(picker).toHaveValue('second');
  await picker.selectOption('default');
  await expect(picker).toHaveValue('default');

  await page.getByRole('button', { name: 'Palette', exact: true }).click();
  await page.locator('.bank-picker select').selectOption('second');
  await page.getByRole('button', { name: /^00 #00FF00/ }).click();
  await expect(page.getByRole('heading', { name: '#00FF00' })).toBeVisible();
});

test('runtime ticks refresh all visible bank editors', async ({ page, e2e }) => {
  for (const kind of kinds) {
    await openBankEditor(page, kind);
    await e2e.setTickBanks({ [kind]: 'second' });
    await expect(page.locator('.bank-picker select')).toHaveValue('second', { timeout: 2_000 });
    const calls = await e2e.calls();
    expect(calls.some((call) => call.command === 'studio_asset_bank' && call.args.kind === kind && call.args.action === 'read')).toBeTruthy();
    await e2e.setTickBanks({ [kind]: 'default' });
    await expect(page.locator('.bank-picker select')).toHaveValue('default', { timeout: 2_000 });
  }
});

test('bank create, select, and delete failures preserve active data and report toast', async ({ page, e2e }) => {
  await openBankEditor(page, 'palette');
  await e2e.failNext('studio_asset_bank:palette:create', 'create denied');
  await createBankViaDialog(page, 'palette', 'third');
  await expect(page.getByText('Bank create failed: create denied')).toBeVisible();
  await expect(page.locator('.bank-picker select')).toHaveValue('default');

  await e2e.failNext('studio_asset_bank:palette:select', 'disk denied');
  await page.locator('.bank-picker select').selectOption('second');
  await expect(page.getByText('Bank select failed: disk denied')).toBeVisible();
  await expect(page.locator('.bank-picker select')).toHaveValue('default');
  await page.getByRole('button', { name: /^00 #000000/ }).click();
  await expect(page.getByRole('heading', { name: '#000000' })).toBeVisible();
  expect((await e2e.snapshot() as any).active.palette).toBe('default');

  await page.locator('.bank-picker select').selectOption('second');
  await e2e.failNext('studio_asset_bank:palette:delete', 'delete denied');
  await page.getByTitle('Delete palette bank second').click();
  await expect(page.getByText('Bank delete failed: delete denied')).toBeVisible();
  await expect(page.locator('.bank-picker select')).toHaveValue('second');
  expect((await e2e.snapshot() as any).active.palette).toBe('second');
});

test('latest bank selection wins when an older response arrives late', async ({ page, e2e }) => {
  await openBankEditor(page, 'palette');
  const picker = page.locator('.bank-picker select');
  await e2e.delayNext('studio_asset_bank:palette:select', 300);

  await picker.selectOption('second');
  await expect.poll(async () => (await e2e.calls()).filter((call) => call.command === 'studio_asset_bank' && call.args.action === 'select').length).toBe(1);
  await picker.selectOption('default');

  await expect(picker).toHaveValue('default');
  await page.waitForTimeout(350);
  await expect(picker).toHaveValue('default');
  expect((await e2e.snapshot() as any).active.palette).toBe('default');
  await page.getByRole('button', { name: /^00 #000000/ }).click();
  await expect(page.getByRole('heading', { name: '#000000' })).toBeVisible();
});
