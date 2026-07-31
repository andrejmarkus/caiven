import assert from 'node:assert/strict';
import test from 'node:test';
import {
  collisionCellEdits, dragPanScroll, filledRectangle, floodCells, nextMapZoom, rasterLine,
  sourceOffset, strokeCells,
} from '../src/lib/editorMath.ts';

test('rasterLine bridges skipped pointer cells', () => {
  assert.deepEqual(rasterLine(0, 3, 8), [0, 1, 2, 3]);
  assert.deepEqual(rasterLine(0, 27, 8), [0, 9, 18, 27]);
});

test('filledRectangle works in either drag direction', () => {
  assert.deepEqual(filledRectangle(9, 0, 8), [0, 1, 8, 9]);
});

test('floodCells stops at tile boundaries and map edges', () => {
  const map = [0, 0, 2, 0, 2, 2, 0, 0, 2];
  assert.deepEqual(
    floodCells(map, 0, 7, 3, 3).map((cell) => cell.offset).sort((a, b) => a - b),
    [0, 1, 3, 6, 7],
  );
  assert.deepEqual(floodCells(map, 2, 2, 3, 3), []);
});

test('mouse wheel zoom changes continuously and clamps at limits', () => {
  const zoomedIn = nextMapZoom(1, -120);
  const zoomedOut = nextMapZoom(1, 120);
  assert.ok(zoomedIn > 1 && zoomedIn < 1.5);
  assert.ok(zoomedOut < 1 && zoomedOut > 0.5);
  assert.equal(nextMapZoom(4, -1_000), 4);
  assert.equal(nextMapZoom(0.5, 1_000), 0.5);
  assert.equal(nextMapZoom(2, 0), 2);
});

test('right-button drag pans map opposite pointer movement', () => {
  assert.equal(dragPanScroll(200, 100, 140), 160);
  assert.equal(dragPanScroll(200, 100, 60), 240);
});

test('source navigation resolves exact one-based line and column', () => {
  const source = 'local x = 1\n  sprite(7, x, 8)\nreturn x';
  assert.equal(sourceOffset(source, 2, 3), 14);
  assert.equal(sourceOffset(source, 99, 99), source.length);
  assert.equal(sourceOffset(source, 0, 0), 0);
});

test('collision cell edits only report cells whose brush value actually changes', () => {
  const collision = [0, 0, 1, 2];
  assert.deepEqual(collisionCellEdits(collision, [0, 1, 2, 3], 1), [
    { offset: 0, value: 1 },
    { offset: 1, value: 1 },
    { offset: 3, value: 1 },
  ]);
  assert.deepEqual(collisionCellEdits(collision, [2], 1), []);
});

test('strokeCells: pencil/erase bridges from previous point, or is a dot with no previous', () => {
  assert.deepEqual(strokeCells('pencil', 0, 3, 0, [], 1, 8, 8), [0, 1, 2, 3]);
  assert.deepEqual(strokeCells('erase', 0, 5, null, [], 0, 8, 8), [5]);
});

test('strokeCells: line recomputes from anchor each call for live preview', () => {
  assert.deepEqual(strokeCells('line', 0, 3, 1, [], 1, 8, 8), [0, 1, 2, 3]);
  assert.deepEqual(strokeCells('line', 0, 27, 99, [], 1, 8, 8), [0, 9, 18, 27]);
});

test('strokeCells: rect is filled between anchor and current', () => {
  assert.deepEqual(strokeCells('rect', 9, 0, null, [], 1, 8, 8), [0, 1, 8, 9]);
});

test('strokeCells: fill floods from current using values/replacement, ignoring anchor/previous', () => {
  const map = [0, 0, 2, 0, 2, 2, 0, 0, 2];
  assert.deepEqual(
    strokeCells('fill', 99, 0, null, map, 7, 3, 3).sort((a, b) => a - b),
    [0, 1, 3, 6, 7],
  );
});
