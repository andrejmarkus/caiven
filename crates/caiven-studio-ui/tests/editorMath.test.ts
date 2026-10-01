import assert from 'node:assert/strict';
import test from 'node:test';
import {
  autotileBitmask, autotileEdits, collisionCellEdits, composeGroup, decomposeGroup, dragPanScroll, filledRectangle,
  flipHorizontal, flipVertical, floodCells, hexToRgb, moveCursor, moveRegion, nextMapZoom, pasteRegion, rasterLine,
  rectangleOutline, regionFromPoints, regionValues, rotateClockwise, rotateCounterClockwise,
  strokeCells,
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

test('moveCursor steps within bounds on a 3x3 grid', () => {
  assert.equal(moveCursor(4, 1, 0, 3, 3), 5); // (1,1) -> (2,1)
  assert.equal(moveCursor(4, -1, 0, 3, 3), 3); // (1,1) -> (0,1)
  assert.equal(moveCursor(4, 0, 1, 3, 3), 7); // (1,1) -> (1,2)
  assert.equal(moveCursor(4, 0, -1, 3, 3), 1); // (1,1) -> (1,0)
});

test('moveCursor clamps at each edge instead of wrapping', () => {
  assert.equal(moveCursor(0, -1, 0, 3, 3), 0); // already at left edge
  assert.equal(moveCursor(2, 1, 0, 3, 3), 2); // already at right edge
  assert.equal(moveCursor(0, 0, -1, 3, 3), 0); // already at top edge
  assert.equal(moveCursor(6, 0, 1, 3, 3), 6); // already at bottom edge
});

test('moveCursor is a no-op exactly at a boundary in the boundary direction', () => {
  assert.equal(moveCursor(8, 1, 1, 3, 3), 8); // bottom-right corner, pushed further out
});

test('regionFromPoints normalizes either drag direction into x0/y0/w/h', () => {
  assert.deepEqual(regionFromPoints(0, 0, 8), { x0: 0, y0: 0, w: 1, h: 1 });
  assert.deepEqual(regionFromPoints(9, 0, 8), { x0: 0, y0: 0, w: 2, h: 2 });
  assert.deepEqual(regionFromPoints(0, 9, 8), { x0: 0, y0: 0, w: 2, h: 2 });
});

test('regionValues reads a sub-rectangle row by row', () => {
  const grid = [1, 2, 3, 4, 5, 6, 7, 8, 9]; // 3x3
  assert.deepEqual(regionValues(grid, { x0: 1, y0: 0, w: 2, h: 2 }, 3), [2, 3, 5, 6]);
});

test('pasteRegion clips a patch against the target grid bounds', () => {
  const edits = pasteRegion(2, 2, 2, 2, [1, 2, 3, 4], 3, 3);
  assert.deepEqual(edits, [{ index: 8, value: 1 }]); // only (2,2) is in-bounds
});

test('flipHorizontal and flipVertical mirror a rectangular grid', () => {
  const grid = [1, 2, 3, 4, 5, 6]; // 3x2
  assert.deepEqual(flipHorizontal(grid, 3, 2), [3, 2, 1, 6, 5, 4]);
  assert.deepEqual(flipVertical(grid, 3, 2), [4, 5, 6, 1, 2, 3]);
});

test('rotateClockwise and rotateCounterClockwise are inverses on a square grid', () => {
  const grid = [1, 2, 3, 4]; // 2x2
  const cw = rotateClockwise(grid, 2, 2);
  assert.deepEqual(cw, [3, 1, 4, 2]);
  assert.deepEqual(rotateCounterClockwise(cw, 2, 2), grid);
});

test('rectangleOutline traces only the border of the box filledRectangle would fill', () => {
  const outline = rectangleOutline(0, 12, 5); // 3x3 box, width 5
  assert.deepEqual(outline.slice().sort((a, b) => a - b), [0, 1, 2, 5, 7, 10, 11, 12]);
  // Center cell (1,1) = offset 6 is excluded — that's the whole point of "outline".
  assert.ok(!outline.includes(6));
});

test('moveRegion relocates a region and clears the part of the source the destination does not cover', () => {
  const grid = [1, 2, 3, 4, 5, 6, 7, 8, 9]; // 3x3
  assert.deepEqual(moveRegion(grid, 0, 0, 1, 1, 1, 0, 3, 3), [{ index: 0, value: 0 }, { index: 1, value: 1 }]);
});

test('moveRegion does not clear cells the overlapping destination itself writes', () => {
  const grid = [1, 2, 3, 4]; // 1x4 row
  assert.deepEqual(
    moveRegion(grid, 0, 0, 2, 1, 1, 0, 4, 1),
    [{ index: 0, value: 0 }, { index: 1, value: 1 }, { index: 2, value: 2 }],
  );
});

test('autotileBitmask sets one bit per same-terrain cardinal neighbor', () => {
  const tiles = [0, 20, 0, 0, 16, 21, 0, 0, 0]; // 3x3; base 16 terrain at offset 4
  assert.equal(autotileBitmask(tiles, 4, 16, 3, 3), 1 | 2); // N (20) and E (21) belong; S, W don't
});

test('autotileEdits recomputes the painted cell and its same-terrain neighbors, ignoring tile 0', () => {
  const projected = [16, 16, 0]; // 1x3 row, seed painted at offset 1
  const edits = autotileEdits(projected, [1], 3, 1).sort((a, b) => a.offset - b.offset);
  assert.deepEqual(edits, [{ offset: 0, tile: 16 + 2 }, { offset: 1, tile: 16 + 8 }]);
});

test('autotileEdits leaves an unrelated terrain family untouched', () => {
  const projected = [16, 32]; // adjacent but different 16-wide families
  assert.deepEqual(autotileEdits(projected, [0, 1], 2, 1), []);
});

test('composeGroup/decomposeGroup round-trip a multi-slot sprite-sheet block', () => {
  const sheetCols = 4;
  const sheet = new Array(16 * 64).fill(0);
  // Slot 0 (top-left) gets pixel (0,0)=9; slot 1 (top-right of the 2x1 group) gets pixel (0,0)=3.
  sheet[0] = 9;
  sheet[1 * 64] = 3;
  const group = composeGroup(sheet, 0, 2, 1, sheetCols);
  assert.equal(group.length, 16 * 8);
  assert.equal(group[0], 9); // slot 0's (0,0)
  assert.equal(group[8], 3); // slot 1 starts at group x=8

  const parts = decomposeGroup(group, 0, 2, 1, sheetCols);
  assert.deepEqual(parts.map((p) => p.slot), [0, 1]);
  assert.equal(parts[0].pixels[0], 9);
  assert.equal(parts[1].pixels[0], 3);
});

test('hexToRgb parses #rrggbb and treats empty as black', () => {
  assert.deepEqual(hexToRgb('#ff8001'), [255, 128, 1]);
  assert.deepEqual(hexToRgb(''), [0, 0, 0]);
});
