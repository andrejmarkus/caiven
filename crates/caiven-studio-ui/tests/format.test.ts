import assert from 'node:assert/strict';
import test from 'node:test';
import { fileName, keyLabel, safeFileName, shortcut, tidyPath } from '../src/lib/format.ts';

test('keyLabel names a physical key, preferring what the keyboard layout prints', () => {
  assert.equal(keyLabel('KeyW'), 'W');
  assert.equal(keyLabel('Digit1'), '1');
  assert.equal(keyLabel('ArrowUp'), 'Up');
  assert.equal(keyLabel('ShiftLeft'), 'Shift Left');
  assert.equal(keyLabel('Backspace'), 'Backspace');
  const qwertz = new Map([['KeyZ', 'y'], ['Space', ' ']]);
  assert.equal(keyLabel('KeyZ', qwertz), 'Y');
  assert.equal(keyLabel('Space', qwertz), 'Space');
});

test('safeFileName keeps a readable title that every OS accepts as a file name', () => {
  assert.equal(safeFileName('Space: Invaders?'), 'Space_ Invaders_');
  assert.equal(safeFileName('Hra č. 1 / demo'), 'Hra č. 1 _ demo');
  assert.equal(safeFileName('trailing dot. '), 'trailing dot');
  assert.equal(safeFileName('CON'), 'CON_');
  assert.equal(safeFileName('  '), 'cart');
});

test('shortcut keeps Mac symbols on Mac and spells Ctrl/Shift elsewhere', () => {
  assert.equal(shortcut('⇧⌘Z', true), '⇧⌘Z');
  assert.equal(shortcut('⌘K', false), 'Ctrl+K');
  assert.equal(shortcut('⇧⌘P', false), 'Ctrl+Shift+P');
  assert.equal(shortcut('F2', false), 'F2');
});

test('fileName returns the last segment for POSIX and Windows paths', () => {
  assert.equal(fileName('/Users/me/carts/lantern'), 'lantern');
  assert.equal(fileName('C:\\Users\\me\\carts\\lantern'), 'lantern');
  assert.equal(fileName('C:\\Users\\me\\carts\\game.cav'), 'game.cav');
  assert.equal(fileName('C:\\Users\\me\\carts\\lantern\\'), 'lantern');
});

test('tidyPath keeps the last two POSIX segments', () => {
  assert.equal(tidyPath('/Users/me/carts/lantern/main.lua'), '…/lantern/main.lua');
  assert.equal(tidyPath('/carts/one'), '/carts/one');
});

test('tidyPath trims Windows backslash paths instead of returning them whole', () => {
  assert.equal(tidyPath('C:\\Users\\me\\carts\\lantern\\main.lua'), '…\\lantern\\main.lua');
  assert.equal(tidyPath('C:\\carts'), 'C:\\carts');
});
