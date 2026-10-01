export interface MapCell { offset: number; tile: number; }

export const MAP_ZOOM_LEVELS = [0.5, 1, 2, 4] as const;

export function nextMapZoom(current: number, deltaY: number): number {
  if (!Number.isFinite(deltaY) || deltaY === 0) return current;
  const next = current * Math.exp(-deltaY * 0.0015);
  return Math.max(MAP_ZOOM_LEVELS[0], Math.min(MAP_ZOOM_LEVELS.at(-1)!, next));
}

export function dragPanScroll(startScroll: number, startPointer: number, currentPointer: number): number {
  return startScroll + startPointer - currentPointer;
}

/** A collision-type id (u8) — the domain is whatever the cart's collision-type table defines, not a fixed enum. */
export type CollisionBrush = number;
export interface CollisionEdit { offset: number; value: number; }

export function collisionCellEdits(
  collision: readonly number[], offsets: readonly number[], brush: CollisionBrush,
): CollisionEdit[] {
  const edits = new Map<number, number>();
  for (const offset of offsets) {
    const before = collision[offset] ?? 0;
    if (before !== brush) edits.set(offset, brush);
  }
  return [...edits].map(([offset, value]) => ({ offset, value }));
}

export function rasterLine(from: number, to: number, width: number): number[] {
  const cells: number[] = [];
  let x0 = from % width;
  let y0 = Math.floor(from / width);
  const x1 = to % width;
  const y1 = Math.floor(to / width);
  const dx = Math.abs(x1 - x0);
  const sx = x0 < x1 ? 1 : -1;
  const dy = -Math.abs(y1 - y0);
  const sy = y0 < y1 ? 1 : -1;
  let error = dx + dy;
  while (true) {
    cells.push(y0 * width + x0);
    if (x0 === x1 && y0 === y1) break;
    const twice = error * 2;
    if (twice >= dy) { error += dy; x0 += sx; }
    if (twice <= dx) { error += dx; y0 += sy; }
  }
  return cells;
}

export function filledRectangle(from: number, to: number, width: number): number[] {
  const x0 = from % width;
  const y0 = Math.floor(from / width);
  const x1 = to % width;
  const y1 = Math.floor(to / width);
  const cells: number[] = [];
  for (let y = Math.min(y0, y1); y <= Math.max(y0, y1); y += 1) {
    for (let x = Math.min(x0, x1); x <= Math.max(x0, x1); x += 1) cells.push(y * width + x);
  }
  return cells;
}

/** Just the border cells of the rectangle {@link filledRectangle} would fill —
 *  PICO-8's rect/rectb pairing, so a wall or panel outline doesn't need a
 *  fill-then-erase-the-middle workaround. */
export function rectangleOutline(from: number, to: number, width: number): number[] {
  const x0 = from % width, y0 = Math.floor(from / width);
  const x1 = to % width, y1 = Math.floor(to / width);
  const left = Math.min(x0, x1), right = Math.max(x0, x1);
  const top = Math.min(y0, y1), bottom = Math.max(y0, y1);
  const cells: number[] = [];
  for (let x = left; x <= right; x += 1) {
    cells.push(top * width + x);
    if (bottom !== top) cells.push(bottom * width + x);
  }
  for (let y = top + 1; y < bottom; y += 1) {
    cells.push(y * width + left);
    if (right !== left) cells.push(y * width + right);
  }
  return cells;
}

export type StrokeTool = 'pencil' | 'line' | 'rect' | 'rect-outline' | 'fill' | 'erase' | 'autotile';

export function strokeCells(
  tool: StrokeTool,
  anchor: number,
  current: number,
  previous: number | null,
  values: readonly number[],
  replacement: number,
  width: number,
  height: number,
): number[] {
  switch (tool) {
    case 'pencil':
    case 'autotile':
    case 'erase':
      return rasterLine(previous ?? current, current, width);
    case 'line':
      return rasterLine(anchor, current, width);
    case 'rect':
      return filledRectangle(anchor, current, width);
    case 'rect-outline':
      return rectangleOutline(anchor, current, width);
    case 'fill':
      return floodCells(values, current, replacement, width, height).map((cell) => cell.offset);
    default:
      return [];
  }
}

/** Moves a flat grid offset by (dx,dy), clamped to stay inside width×height. */
export function moveCursor(at: number, dx: number, dy: number, width: number, height: number): number {
  const x = Math.max(0, Math.min(width - 1, (at % width) + dx));
  const y = Math.max(0, Math.min(height - 1, Math.floor(at / width) + dy));
  return y * width + x;
}

export interface PixelRegion { x0: number; y0: number; w: number; h: number; }

/** Marquee rectangle from two grid indices (any drag direction), for the
 *  select tool shared by the map and sprite editors. */
export function regionFromPoints(anchor: number, current: number, width: number): PixelRegion {
  const ax = anchor % width, ay = Math.floor(anchor / width);
  const cx = current % width, cy = Math.floor(current / width);
  const x0 = Math.min(ax, cx), x1 = Math.max(ax, cx);
  const y0 = Math.min(ay, cy), y1 = Math.max(ay, cy);
  return { x0, y0, w: x1 - x0 + 1, h: y1 - y0 + 1 };
}

/** Reads a region out of a flat width-major grid, row by row. */
export function regionValues(values: readonly number[], region: PixelRegion, width: number): number[] {
  const out: number[] = [];
  for (let dy = 0; dy < region.h; dy += 1) for (let dx = 0; dx < region.w; dx += 1) {
    out.push(values[(region.y0 + dy) * width + (region.x0 + dx)] ?? 0);
  }
  return out;
}

/** Places a patch (row-major, `patch.length === w*h`) at `x0,y0` in a
 *  width×height grid, clipping anything that would fall outside it. */
export function pasteRegion(
  x0: number, y0: number, w: number, h: number, patch: readonly number[], width: number, height: number,
): { index: number; value: number }[] {
  const edits: { index: number; value: number }[] = [];
  for (let dy = 0; dy < h; dy += 1) for (let dx = 0; dx < w; dx += 1) {
    const x = x0 + dx, y = y0 + dy;
    if (x < 0 || y < 0 || x >= width || y >= height) continue;
    edits.push({ index: y * width + x, value: patch[dy * w + dx] ?? 0 });
  }
  return edits;
}

/** Relocates a w×h region from (x0,y0) to (nx0,ny0) in a width×height grid.
 *  Vacated cells the destination doesn't itself cover are cleared to 0, so an
 *  overlapping move never leaves a stray copy behind. */
export function moveRegion(
  values: readonly number[], x0: number, y0: number, w: number, h: number,
  nx0: number, ny0: number, width: number, height: number,
): { index: number; value: number }[] {
  const patch = regionValues(values, { x0, y0, w, h }, width);
  const clears = pasteRegion(x0, y0, w, h, new Array(w * h).fill(0), width, height)
    .filter(({ index }) => {
      const ex = index % width, ey = Math.floor(index / width);
      return ex < nx0 || ex >= nx0 + w || ey < ny0 || ey >= ny0 + h;
    });
  const places = pasteRegion(nx0, ny0, w, h, patch, width, height);
  const merged = new Map<number, number>();
  for (const { index, value } of clears) merged.set(index, value);
  for (const { index, value } of places) merged.set(index, value);
  return [...merged].map(([index, value]) => ({ index, value }));
}

export function flipHorizontal(values: readonly number[], width: number, height: number): number[] {
  const out = new Array(width * height).fill(0);
  for (let y = 0; y < height; y += 1) for (let x = 0; x < width; x += 1) {
    out[y * width + (width - 1 - x)] = values[y * width + x] ?? 0;
  }
  return out;
}

export function flipVertical(values: readonly number[], width: number, height: number): number[] {
  const out = new Array(width * height).fill(0);
  for (let y = 0; y < height; y += 1) for (let x = 0; x < width; x += 1) {
    out[(height - 1 - y) * width + x] = values[y * width + x] ?? 0;
  }
  return out;
}

/** Only meaningful for a square grid (width === height) — callers must not
 *  offer rotate on a non-square sprite group, since the result would need a
 *  different-shaped canvas. */
export function rotateClockwise(values: readonly number[], width: number, height: number): number[] {
  const out = new Array(width * height).fill(0);
  for (let y = 0; y < height; y += 1) for (let x = 0; x < width; x += 1) {
    out[x * height + (height - 1 - y)] = values[y * width + x] ?? 0;
  }
  return out;
}

export function rotateCounterClockwise(values: readonly number[], width: number, height: number): number[] {
  const out = new Array(width * height).fill(0);
  for (let y = 0; y < height; y += 1) for (let x = 0; x < width; x += 1) {
    out[(width - 1 - x) * height + y] = values[y * width + x] ?? 0;
  }
  return out;
}

/** Composes an N×M block of adjacent sprite-sheet slots into one flat pixel
 *  grid, `(slotsW*8) × (slotsH*8)`, so the group can be edited as one canvas. */
export function composeGroup(
  sheet: readonly number[], originSlot: number, slotsW: number, slotsH: number, sheetCols: number,
): number[] {
  const width = slotsW * 8;
  const out = new Array(width * slotsH * 8).fill(0);
  const originX = originSlot % sheetCols, originY = Math.floor(originSlot / sheetCols);
  for (let sy = 0; sy < slotsH; sy += 1) for (let sx = 0; sx < slotsW; sx += 1) {
    const slot = (originY + sy) * sheetCols + (originX + sx);
    for (let py = 0; py < 8; py += 1) for (let px = 0; px < 8; px += 1) {
      out[(sy * 8 + py) * width + (sx * 8 + px)] = sheet[slot * 64 + py * 8 + px] ?? 0;
    }
  }
  return out;
}

/** Inverse of {@link composeGroup} — splits an edited group canvas back into
 *  per-slot 64-pixel arrays, ready for one `onSprite` call per slot. */
export function decomposeGroup(
  pixels: readonly number[], originSlot: number, slotsW: number, slotsH: number, sheetCols: number,
): { slot: number; pixels: number[] }[] {
  const width = slotsW * 8;
  const originX = originSlot % sheetCols, originY = Math.floor(originSlot / sheetCols);
  const out: { slot: number; pixels: number[] }[] = [];
  for (let sy = 0; sy < slotsH; sy += 1) for (let sx = 0; sx < slotsW; sx += 1) {
    const slot = (originY + sy) * sheetCols + (originX + sx);
    const slotPixels = new Array(64).fill(0);
    for (let py = 0; py < 8; py += 1) for (let px = 0; px < 8; px += 1) {
      slotPixels[py * 8 + px] = pixels[(sy * 8 + py) * width + (sx * 8 + px)] ?? 0;
    }
    out.push({ slot, pixels: slotPixels });
  }
  return out;
}

/** A terrain family is 16 consecutive tile ids, `base..base+15` (`base` is
 *  always a multiple of 16 — the sheet's own convention, not stored state).
 *  Tile 0 never belongs to any family: it's the map's "nothing here" value. */
function terrainBase(tile: number): number {
  return Math.floor(tile / 16) * 16;
}

function sameTerrain(tile: number, base: number): boolean {
  return tile !== 0 && tile >= base && tile < base + 16;
}

/** Cardinal-neighbor bitmask (1=N, 2=E, 4=S, 8=W) for whether each side's
 *  neighbor belongs to the same terrain family as `base`. The tile that
 *  belongs at `offset` is `base + autotileBitmask(...)` — the sheet must lay
 *  its 16 edge/corner variants out in that order for the family to read. */
export function autotileBitmask(
  tiles: readonly number[], offset: number, base: number, width: number, height: number,
): number {
  const x = offset % width, y = Math.floor(offset / width);
  const belongs = (nx: number, ny: number) => {
    if (nx < 0 || ny < 0 || nx >= width || ny >= height) return false;
    return sameTerrain(tiles[ny * width + nx] ?? 0, base);
  };
  let mask = 0;
  if (belongs(x, y - 1)) mask |= 1;
  if (belongs(x + 1, y)) mask |= 2;
  if (belongs(x, y + 1)) mask |= 4;
  if (belongs(x - 1, y)) mask |= 8;
  return mask;
}

/** Recomputes autotile variants around a set of just-painted seed cells: the
 *  seeds themselves plus their 4 neighbors, so placing one tile also fixes
 *  the edges of whatever it now touches. Only touches cells that already
 *  belong to the painted tile's own terrain family — a neighboring, unrelated
 *  terrain is left alone. */
export function autotileEdits(
  tiles: readonly number[], seeds: readonly number[], width: number, height: number,
): MapCell[] {
  const affected = new Set<number>();
  for (const seed of seeds) {
    affected.add(seed);
    const x = seed % width, y = Math.floor(seed / width);
    if (y > 0) affected.add(seed - width);
    if (y + 1 < height) affected.add(seed + width);
    if (x > 0) affected.add(seed - 1);
    if (x + 1 < width) affected.add(seed + 1);
  }
  const edits: MapCell[] = [];
  for (const offset of affected) {
    const tile = tiles[offset] ?? 0;
    if (tile === 0) continue;
    const base = terrainBase(tile);
    const next = base + autotileBitmask(tiles, offset, base, width, height);
    if (next !== tile) edits.push({ offset, tile: next });
  }
  return edits;
}

export function floodCells(
  values: readonly number[], start: number, replacement: number, width: number, height: number,
): MapCell[] {
  const target = values[start] ?? 0;
  if (target === replacement) return [];
  const cells: MapCell[] = [];
  const queue = [start];
  const seen = new Set<number>();
  while (queue.length) {
    const cell = queue.pop()!;
    if (seen.has(cell) || (values[cell] ?? 0) !== target) continue;
    seen.add(cell);
    cells.push({ offset: cell, tile: replacement });
    const x = cell % width;
    const y = Math.floor(cell / width);
    if (x > 0) queue.push(cell - 1);
    if (x + 1 < width) queue.push(cell + 1);
    if (y > 0) queue.push(cell - width);
    if (y + 1 < height) queue.push(cell + width);
  }
  return cells;
}

export function hexToRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16) || 0;
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
