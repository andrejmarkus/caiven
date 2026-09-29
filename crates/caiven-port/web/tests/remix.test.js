import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { parseCav, writeCav, luaSource, withLuaSource, crc32, FORMAT_VERSION, LUA_SOURCE } from '../src/lib/cav.js';
import { findConstants, setConstant, parseLuaError, errorHint } from '../src/lib/remix.js';

const smoke = new Uint8Array(await readFile(new URL('../../../../carts/dev/smoke.cav', import.meta.url)));

test('crc32 matches the standard check value', () => {
  assert.equal(crc32(new TextEncoder().encode('123456789')), 0xcbf43926);
});

test('parse then write reproduces a Studio-built cart byte for byte', () => {
  assert.equal(new DataView(smoke.buffer).getUint16(6, true), FORMAT_VERSION);
  assert.deepEqual(writeCav(parseCav(smoke)), smoke);
});

test('other format versions are rejected', () => {
  const other = smoke.slice();
  new DataView(other.buffer).setUint16(6, FORMAT_VERSION + 1, true);
  assert.throws(() => parseCav(other), /format v2 is not supported/);
});

test('swapping Lua keeps every other section and re-parses', () => {
  const cav = parseCav(smoke);
  const bytes = withLuaSource(cav, 'local SPEED = 4\n', { title: 'My remix', author: 'me' });
  const back = parseCav(bytes);
  assert.equal(luaSource(back), 'local SPEED = 4\n');
  assert.equal(back.title, 'My remix');
  assert.equal(back.author, 'me');
  const others = (c) => c.sections.filter((s) => s.kind !== LUA_SOURCE);
  assert.deepEqual(others(back), others(cav));
});

test('header fields truncate on a character boundary', () => {
  const back = parseCav(withLuaSource(parseCav(smoke), '', { title: 'é'.repeat(40) }));
  assert.equal(back.title, 'é'.repeat(16));
});

test('corrupt carts are rejected', () => {
  const bad = smoke.slice();
  bad[bad.length - 1] ^= 0xff;
  assert.throws(() => parseCav(bad), /checksum/);
  assert.throws(() => parseCav(smoke.subarray(0, 40)), /not a Caiven cart/);
});

test('constants are found and rewritten in the real source', () => {
  const src = 'local SPEED = 2 -- pixels\nlocal name = 3\nlocal GRAVITY = 0.5  -- try 0.05\nfunction _update() end\n';
  assert.deepEqual(findConstants(src), [
    { name: 'SPEED', value: '2', line: 1, suggestion: null },
    { name: 'GRAVITY', value: '0.5', line: 3, suggestion: '0.05' },
  ]);
  assert.equal(setConstant(src, 3, '0.05').split('\n')[2], 'local GRAVITY = 0.05  -- try 0.05');
  assert.equal(setConstant(src, 1, '5').split('\n')[0], 'local SPEED = 5 -- pixels');
  assert.equal(setConstant(src, 1, 'os.exit()'), src);
  assert.equal(setConstant(src, 2, '9'), src);
});

test('every remix starter offers six constants, each with a try value', async () => {
  for (const name of ['chain', 'hop', 'juggle', 'meteor']) {
    const src = await readFile(new URL(`../../../../projects/remix/${name}/main.lua`, import.meta.url), 'utf8');
    const found = findConstants(src.replace(/\r\n/g, '\n'));
    assert.equal(found.length, 6, name);
    for (const c of found) assert.notEqual(c.suggestion, null, `${name} ${c.name}`);
  }
});

test('errors map to cart lines and plain-language hints', () => {
  assert.deepEqual(parseLuaError('cart:12: attempt to call a nil value (global \'draw_sprit\')'), {
    line: 12, detail: 'attempt to call a nil value (global \'draw_sprit\')',
  });
  assert.equal(parseLuaError('syntax error: [string "cart"]:3: \'end\' expected near <eof>').line, 3);
  assert.equal(parseLuaError('cart requires mod \'x\'').line, null);
  // Verbatim from the shipped WASM runtime.
  assert.deepEqual(parseLuaError('cart:3: cart:3: attempt to perform arithmetic on a nil value\nstack traceback:\n\t[C]: in metamethod \'add\'\n\tcart:3: in function \'_update\''), {
    line: 3, detail: 'attempt to perform arithmetic on a nil value',
  });
  assert.deepEqual(parseLuaError('runtime error: your game did not finish drawing this frame — is there a loop that never ends?\nstack traceback:\n\tcart:1: in main chunk\n'), {
    line: 1, detail: 'your game did not finish drawing this frame — is there a loop that never ends?',
  });
  assert.deepEqual(parseLuaError('runtime error: cart:2: attempt to index a nil value (global \'io\')\nstack traceback:\n\t[C]: in metamethod \'index\'\n\tcart:2: in function \'_init\''), {
    line: 2, detail: 'attempt to index a nil value (global \'io\')',
  });
  assert.deepEqual(parseLuaError('syntax error: cart:4: \')\' expected (to close \'(\' at line 3) near \'end\''), {
    line: 4, detail: '\')\' expected (to close \'(\' at line 3) near \'end\'',
  });
  assert.match(errorHint('attempt to call a nil value (global \'draw_sprit\')'), /draw_sprit/);
  assert.match(errorHint('\'end\' expected near <eof>'), /end/);
  assert.equal(errorHint('something novel'), null);
});
