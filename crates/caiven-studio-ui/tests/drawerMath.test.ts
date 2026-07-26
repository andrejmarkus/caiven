import assert from 'node:assert/strict';
import test from 'node:test';
import {
  MEMORY_PAGE_SIZE, clampMemoryBase, formatMemoryRows, memoryBaseForAddress,
} from '../src/lib/drawerMath.ts';

test('memory address jumps keep exact row-aligned regions visible', () => {
  assert.equal(memoryBaseForAddress(0x4000, 0x10000), 0x4000);
  assert.equal(memoryBaseForAddress(0x8007, 0x10000), 0x8000);
  assert.equal(memoryBaseForAddress(0xffff, 0x10000), 0xffa0);
});

test('memory paging clamps both ends of RAM', () => {
  assert.equal(clampMemoryBase(-MEMORY_PAGE_SIZE, 0x10000), 0);
  assert.equal(clampMemoryBase(0x10000, 0x10000), 0xffa0);
  assert.equal(clampMemoryBase(0x4033, 0x10000), 0x4030);
});

test('memory rows show correct addresses, hex bytes, and printable ASCII', () => {
  const ram = Array(0x100).fill(0);
  ram.splice(0x20, 4, 0x43, 0x61, 0x76, 0x0a);
  const rows = formatMemoryRows(ram, 0x20);
  assert.equal(rows.length, 6);
  assert.equal(rows[0].address, '0x0020');
  assert.equal(rows[0].hex.startsWith('43 61 76 0A'), true);
  assert.equal(rows[0].ascii.startsWith('Cav.'), true);
});
