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

export function sourceOffset(source: string, line: number, column = 1): number {
  const lines = source.split('\n');
  const targetLine = Math.max(1, Math.min(lines.length, Math.trunc(line) || 1));
  let offset = 0;
  for (let index = 0; index < targetLine - 1; index += 1) offset += lines[index].length + 1;
  const lineText = lines[targetLine - 1] ?? '';
  const targetColumn = Math.max(1, Math.min(lineText.length + 1, Math.trunc(column) || 1));
  return offset + targetColumn - 1;
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

export type StrokeTool = 'pencil' | 'line' | 'rect' | 'fill' | 'erase';

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
    case 'erase':
      return rasterLine(previous ?? current, current, width);
    case 'line':
      return rasterLine(anchor, current, width);
    case 'rect':
      return filledRectangle(anchor, current, width);
    case 'fill':
      return floodCells(values, current, replacement, width, height).map((cell) => cell.offset);
    default:
      return [];
  }
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
