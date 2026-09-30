import assert from 'node:assert/strict';
import test from 'node:test';
import {
  builtinAliasEntries, declaresLocal, localName, moduleKey, projectEntries, requireNameAt, scanModule,
} from '../src/lib/luaModules.ts';

const ENEMY = `local M = {}
local helper = {}

-- Spawns an enemy at x, y.
function M.new(x, y) end
M.speed = 2
function helper.hidden() end
function draw_enemies(list) end
Enemy = {}
function Enemy.update(e) end
  function nested() end

return M
`;

test('module keys follow the require convention', () => {
  assert.equal(moduleKey('ui/hud.lua'), 'ui.hud');
  assert.equal(moduleKey('ui\\hud.lua'), 'ui.hud');
  assert.equal(moduleKey('enemy.lua'), 'enemy');
});

test('scan splits returned-table exports from globals and skips locals and nested code', () => {
  const scan = scanModule('enemy', ENEMY);
  assert.deepEqual(scan.exports.map((entry) => entry.name), ['new', 'speed']);
  assert.deepEqual(scan.globals.map((entry) => entry.name), ['draw_enemies', 'Enemy', 'Enemy.update']);
  const spawn = scan.exports[0];
  assert.equal(spawn.doc, 'Spawns an enemy at x, y.');
  assert.deepEqual(spawn.params.map((param) => param.name), ['x', 'y']);
  assert.equal(scan.exports[1].category, 'Project value');
});

test('project entries qualify exports by the local alias of each require', () => {
  const modules = [{ key: 'enemy', text: ENEMY, entry: false }];
  const names = projectEntries(modules, 'local foes = require("enemy")\n').map((entry) => entry.name);
  assert.ok(names.includes('foes.new'));
  assert.ok(names.includes('draw_enemies'));
  assert.ok(!projectEntries(modules, '').some((entry) => entry.name.endsWith('.new') && entry.name !== 'Enemy.new'));
});

test('require names are found under the cursor, slashes normalized', () => {
  const line = 'local hud = require("ui/hud")';
  assert.deepEqual(requireNameAt(line, 22), { from: 21, to: 27, name: 'ui.hud' });
  assert.equal(requireNameAt(line, 5), null);
});

test('built-in entries follow the alias a file binds the module to', () => {
  const api = [
    { name: 'tween.new', params: [], returns: 'table', doc: '', category: 'Gameplay stdlib' },
    { name: 'Camera.follow', params: [], returns: 'nil', doc: '', category: 'Gameplay stdlib' },
  ];
  const modules = [{ name: 'tween', export: 'tween' }, { name: 'camera', export: 'Camera' }];
  const text = 'local tw = require "tween"\nlocal Camera = require "camera"\n';
  assert.deepEqual(builtinAliasEntries(api, modules, text).map((entry) => entry.name), ['tw.new']);
  assert.deepEqual(builtinAliasEntries(api, modules, text, ['tween']), []);
});

test('local declarations are recognised, globals and fields are not', () => {
  assert.ok(declaresLocal('local Camera = require "camera"', 'Camera'));
  assert.ok(declaresLocal('local Vec2, Camera = a, b', 'Camera'));
  assert.ok(!declaresLocal('Camera = {}', 'Camera'));
  assert.ok(!declaresLocal('local CameraRig = 1', 'Camera'));
  assert.equal(localName('ui.hud'), 'hud');
});
