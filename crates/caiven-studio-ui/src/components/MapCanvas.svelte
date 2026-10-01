<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    collisionCellEdits, hexToRgb, moveCursor, strokeCells, type CollisionBrush, type CollisionEdit, type StrokeTool,
  } from '../lib/editorMath';
  import {
    MAP_H, MAP_PX_H, MAP_PX_W, MAP_W, SCREEN_TILES_H, SCREEN_TILES_W, TILE_SIZE,
  } from '../lib/ipc';
  import type { CollisionType } from '../types';

  type MapLayer = 'tiles' | 'collision';
  type MapTool = 'pencil' | 'fill' | 'rect' | 'rect-outline' | 'pick' | 'erase' | 'line' | 'select' | 'autotile';
  type Cell = { offset: number; tile: number };

  interface Stamp { w: number; h: number; tiles: number[]; }
  export interface MapRegion { x0: number; y0: number; w: number; h: number; }

  interface Props {
    map: number[];
    spriteSheet: number[];
    palette: string[];
    collision: number[];
    collisionTypes: CollisionType[];
    /** Multi-tile brush picked from the tile-sheet picker; {w:1,h:1} is a plain
     *  single-tile brush, same as before this existed. */
    stamp: Stamp;
    showCollision: boolean;
    layer: MapLayer;
    collisionBrush: CollisionBrush;
    tool: MapTool;
    zoom: number;
    onStroke: (cells: Cell[]) => void;
    onCollisionStroke: (edits: CollisionEdit[]) => void;
    onPick: (tile: number) => void;
    onCollisionPick: (brush: CollisionBrush) => void;
    onHover?: (cell: { x: number; y: number; tile: number } | null) => void;
    /** The 'select' tool's marquee, in tile coordinates; null once cleared or
     *  when a different tool is active. Workspace uses this to build the
     *  clipboard on Ctrl+C/Ctrl+X. */
    onSelectionChange?: (region: MapRegion | null) => void;
  }

  let {
    map, spriteSheet, palette, collision, collisionTypes, stamp, showCollision, layer, collisionBrush,
    tool, zoom, onStroke, onCollisionStroke, onPick, onCollisionPick, onHover, onSelectionChange,
  }: Props = $props();
  let canvas: HTMLCanvasElement;
  let drawing = false;
  let anchor: number | null = null;
  let previousCell: number | null = null;
  let tileDraft = new Map<number, number>();
  let collisionDraft = new Map<number, number>();
  let renderFrame: number | undefined;
  let selectAnchor = $state<number | null>(null);
  let selectCurrent = $state<number | null>(null);
  /** Flat tile offset the keyboard operates on; independent of pointer state. */
  let cursor = $state(0);
  /** Non-null while a keyboard-initiated stroke/select is in progress — mirrors
   *  a held mouse button, so arrow keys live-preview the same way a drag does. */
  let kbAnchor: number | null = $state(null);
  let focused = $state(false);
  const selectRegion = $derived.by((): MapRegion | null => {
    if (selectAnchor === null || selectCurrent === null) return null;
    const ax = selectAnchor % MAP_W, ay = Math.floor(selectAnchor / MAP_W);
    const cx = selectCurrent % MAP_W, cy = Math.floor(selectCurrent / MAP_W);
    const x0 = Math.min(ax, cx), x1 = Math.max(ax, cx);
    const y0 = Math.min(ay, cy), y1 = Math.max(ay, cy);
    return { x0, y0, w: x1 - x0 + 1, h: y1 - y0 + 1 };
  });

  // The map is not a whole number of screens wide (128 tiles / 24 per
  // screen), so a trailing sliver of tiles on the right isn't a full screen.
  // Only whole screens get a coordinate label — numbering an 8-tile remnant
  // "screen 5,y" reads as a broken/cut-off screen rather than a fractional one.
  const screenCols = Math.floor(MAP_W / SCREEN_TILES_W);
  const screenRows = Math.floor(MAP_H / SCREEN_TILES_H);
  const screenPctX = (SCREEN_TILES_W / MAP_W) * 100;
  const screenPctY = (SCREEN_TILES_H / MAP_H) * 100;

  $effect(() => {
    if (tool !== 'select') { selectAnchor = null; selectCurrent = null; }
  });

  $effect(() => {
    onSelectionChange?.(selectRegion);
  });

  // The full-map image is kept between renders so a paint stroke can repaint
  // only the tiles it touched. Rebuilding all 16384 tiles costs ~23 ms, which
  // a per-pointer-move redraw cannot afford; a dirty repaint is a handful of
  // tiles regardless of map size.
  let buffer: ImageData | undefined;

  function paintTile(image: ImageData, colors: [number, number, number, number][], tileX: number, tileY: number) {
    const offset = tileY * MAP_W + tileX;
    const tile = tileDraft.get(offset) ?? map[offset] ?? 0;
    const sheet = tile !== 0 ? tile * 64 : -1;
    for (let pixelY = 0; pixelY < TILE_SIZE; pixelY += 1) for (let pixelX = 0; pixelX < TILE_SIZE; pixelX += 1) {
      const at = ((tileY * TILE_SIZE + pixelY) * MAP_PX_W + tileX * TILE_SIZE + pixelX) * 4;
      const paletteIndex = sheet < 0 ? 0 : spriteSheet[sheet + pixelY * TILE_SIZE + pixelX] ?? 0;
      // Tile 0 and palette index 0 are both "nothing here" — clear to the
      // canvas's own black so a repaint erases whatever was there before.
      const rgba = paletteIndex === 0 ? [0, 0, 0, 255] : colors[paletteIndex] ?? colors[0] ?? [0, 0, 0, 255];
      image.data.set(rgba, at);
    }
    const value = collisionDraft.get(offset) ?? collision[offset] ?? 0;
    const ctype = value !== 0 ? collisionTypes.find((t) => t.id === value) : undefined;
    if (!showCollision || !ctype) return;
    const tint = ctype.color;
    const hatch = ctype.shape === 'none';
    for (let pixelY = 0; pixelY < TILE_SIZE; pixelY += 1) for (let pixelX = 0; pixelX < TILE_SIZE; pixelX += 1) {
      const border = pixelX <= 1 || pixelX >= 6 || pixelY <= 1 || pixelY >= 6;
      const dot = hatch && (pixelX + pixelY) % 4 === 0;
      const at = ((tileY * TILE_SIZE + pixelY) * MAP_PX_W + tileX * TILE_SIZE + pixelX) * 4;
      const alpha = border || dot ? 0.85 : 0.3;
      image.data[at] = image.data[at] * (1 - alpha) + tint[0] * alpha;
      image.data[at + 1] = image.data[at + 1] * (1 - alpha) + tint[1] * alpha;
      image.data[at + 2] = image.data[at + 2] * (1 - alpha) + tint[2] * alpha;
      image.data[at + 3] = 255;
    }
  }

  function render() {
    if (!canvas) return;
    const context = canvas.getContext('2d');
    if (!context) return;
    const image = context.createImageData(MAP_PX_W, MAP_PX_H);
    const colors = palette.map((hex): [number, number, number, number] => [...hexToRgb(hex), 255]);
    for (let tileY = 0; tileY < MAP_H; tileY += 1) for (let tileX = 0; tileX < MAP_W; tileX += 1) {
      paintTile(image, colors, tileX, tileY);
    }
    buffer = image;
    context.putImageData(image, 0, 0);
  }

  // Cells the in-progress stroke has already painted, so the next repaint can
  // also cover the ones it is about to stop painting (line/rect rebuild their
  // preview from the anchor on every move).
  let paintedCells = new Set<number>();

  // Repaints just the cells the current stroke touches, adding the ones the
  // previous repaint painted so an abandoned preview is erased.
  function renderStroke() {
    const next = new Set<number>([...tileDraft.keys(), ...collisionDraft.keys()]);
    const dirty = new Set<number>(paintedCells);
    for (const offset of next) dirty.add(offset);
    paintedCells = next;
    renderCells(dirty);
  }

  // Repaints just the given cell offsets, plus whatever the previous draft
  // painted (so a line/rect preview erases its last position). Falls back to a
  // full render before the first one has built the buffer.
  function renderCells(offsets: Iterable<number>) {
    if (!canvas || !buffer) { render(); return; }
    const context = canvas.getContext('2d');
    if (!context) return;
    const colors = palette.map((hex): [number, number, number, number] => [...hexToRgb(hex), 255]);
    let x0 = MAP_W, y0 = MAP_H, x1 = -1, y1 = -1;
    for (const offset of offsets) {
      const tileX = offset % MAP_W, tileY = Math.floor(offset / MAP_W);
      if (tileX < 0 || tileY < 0 || tileX >= MAP_W || tileY >= MAP_H) continue;
      paintTile(buffer, colors, tileX, tileY);
      if (tileX < x0) x0 = tileX;
      if (tileY < y0) y0 = tileY;
      if (tileX > x1) x1 = tileX;
      if (tileY > y1) y1 = tileY;
    }
    if (x1 < 0) return;
    context.putImageData(
      buffer, 0, 0,
      x0 * TILE_SIZE, y0 * TILE_SIZE,
      (x1 - x0 + 1) * TILE_SIZE, (y1 - y0 + 1) * TILE_SIZE,
    );
  }

  // Coalesces redraws triggered by prop changes (bank switch, external map edits) that
  // aren't already followed by a direct render() call. Not requestAnimationFrame: WKWebView's
  // native mouse-tracking run loop (active for the whole time a button is held) starves rAF's
  // display-link callback, so a deferred rAF redraw can silently stall for seconds — confirmed
  // by instrumentation. setTimeout keeps running in that mode.
  function scheduleRender() {
    if (!canvas || renderFrame !== undefined) return;
    renderFrame = window.setTimeout(() => {
      renderFrame = undefined;
      render();
    }, 16);
  }

  $effect(() => {
    map; spriteSheet; palette; collision; collisionTypes; showCollision; tileDraft; collisionDraft; canvas;
    collisionWorking = null;
    scheduleRender();
  });

  onDestroy(() => {
    if (renderFrame !== undefined) clearTimeout(renderFrame);
  });

  function pointerCell(event: PointerEvent) {
    const rect = canvas.getBoundingClientRect();
    const x = Math.max(0, Math.min(MAP_W - 1, Math.floor(((event.clientX - rect.left) / rect.width) * MAP_W)));
    const y = Math.max(0, Math.min(MAP_H - 1, Math.floor(((event.clientY - rect.top) / rect.height) * MAP_H)));
    return y * MAP_W + x;
  }

  function reportHover(event: PointerEvent) {
    const at = pointerCell(event);
    onHover?.({ x: at % MAP_W, y: Math.floor(at / MAP_W), tile: tileDraft.get(at) ?? map[at] ?? 0 });
  }

  // The single-tile value used by tools that don't paint a footprint (fill picks
  // its flood-fill target from this; rect/line/erase paint one tile per cell too
  // — a bigger stamp only "spreads" for the pencil/erase brush, see below).
  function activeTile() {
    return tool === 'erase' ? 0 : stamp.tiles[0];
  }

  function activeCollisionBrush(): CollisionBrush {
    return tool === 'erase' ? 0 : collisionBrush;
  }

  // The collision layer's flood/stroke maths want a plain array of the current
  // values, drafts included. Copying all 16384 cells on every pointer move is
  // what makes a collision drag stutter, so the copy is made once per stroke
  // and then kept in step with the draft; `null` means "rebuild on next use".
  let collisionWorking: number[] | null = null;

  function collisionValues(): number[] {
    if (collisionWorking) return collisionWorking;
    const values = [...collision];
    for (const [offset, value] of collisionDraft) values[offset] = value;
    collisionWorking = values;
    return values;
  }

  // Expands one path cell into the whole w×h stamp footprint anchored there
  // (top-left), clipped to the map bounds. Erasing clears every cell in the
  // footprint to 0 regardless of what the stamp's tiles are.
  function stampFootprint(base: number): Cell[] {
    const baseX = base % MAP_W, baseY = Math.floor(base / MAP_W);
    const cells: Cell[] = [];
    for (let dy = 0; dy < stamp.h; dy += 1) for (let dx = 0; dx < stamp.w; dx += 1) {
      const x = baseX + dx, y = baseY + dy;
      if (x >= MAP_W || y >= MAP_H) continue;
      cells.push({ offset: y * MAP_W + x, tile: tool === 'erase' ? 0 : stamp.tiles[dy * stamp.w + dx] });
    }
    return cells;
  }

  function applyOffsets(offsets: readonly number[]) {
    if (layer === 'tiles') {
      const next = new Map(tileDraft);
      const brushed = (tool === 'pencil' || tool === 'erase') && (stamp.w > 1 || stamp.h > 1);
      if (brushed) {
        for (const base of offsets) for (const cell of stampFootprint(base)) next.set(cell.offset, cell.tile);
      } else {
        const value = activeTile();
        for (const offset of offsets) next.set(offset, value);
      }
      tileDraft = next;
      return;
    }
    const next = new Map(collisionDraft);
    const values = collisionValues();
    for (const edit of collisionCellEdits(values, offsets, activeCollisionBrush())) {
      next.set(edit.offset, edit.value);
      values[edit.offset] = edit.value;
    }
    collisionDraft = next;
  }

  function drawStroke(at: number) {
    // Never called with tool === 'pick'/'select' — begin() branches to pick()
    // or the select-marquee handling first for those.
    const drawTool: StrokeTool = tool as Exclude<MapTool, 'pick' | 'select'>;
    const values = layer === 'tiles' ? map : collisionValues();
    const replacement = layer === 'tiles' ? activeTile() : activeCollisionBrush();
    const offsets = strokeCells(drawTool, anchor ?? at, at, previousCell, values, replacement, MAP_W, MAP_H);
    // line/rect/rect-outline recompute the whole shape from anchor each move (live
    // preview), so the draft is replaced rather than accumulated; paint/erase/fill/
    // autotile accumulate across a drag.
    if (tool === 'line' || tool === 'rect' || tool === 'rect-outline') {
      tileDraft = new Map();
      collisionDraft = new Map();
      collisionWorking = null; // the discarded preview is still in the working copy
    }
    applyOffsets(offsets);
  }

  function pick(at: number) {
    if (layer === 'tiles') {
      onPick(map[at] ?? 0);
      return;
    }
    const value = collision[at] ?? 0;
    onCollisionPick(collisionTypes.some((t) => t.id === value) ? value : 0);
  }

  function beginAt(at: number, options: { pick: boolean }) {
    if (options.pick || tool === 'pick') {
      pick(at);
      return;
    }
    if (tool === 'select') {
      selectAnchor = at;
      selectCurrent = at;
      drawing = true;
      return;
    }
    tileDraft = new Map();
    collisionDraft = new Map();
    collisionWorking = null;
    anchor = at;
    previousCell = at;
    drawing = true;
    if (tool === 'fill') {
      drawStroke(at);
      finishAt();
      return;
    }
    paintedCells = new Set();
    drawStroke(at);
    renderStroke(); // paint inline — see moveAt() for why this can't wait for scheduleRender()
  }

  function moveAt(at: number) {
    if (!drawing) return;
    if (tool === 'select') {
      selectCurrent = at;
      return;
    }
    // Paints happen synchronously, in the pointer handler itself, rather than deferring to
    // scheduleRender()'s timer: the timer callback still *runs* while the mouse button is
    // held, but WKWebView doesn't actually composite/flush the canvas to screen again until
    // the native tracking loop ends — confirmed by comparing a scheduled render (invisible
    // for the whole drag) against a synchronous one (paints every move) in the same build.
    if (tool === 'rect' || tool === 'rect-outline' || tool === 'line') {
      drawStroke(at);
      renderStroke();
    } else if ((tool === 'pencil' || tool === 'erase' || tool === 'autotile') && previousCell !== at) {
      drawStroke(at);
      previousCell = at;
      renderStroke();
    }
  }

  function finishAt() {
    if (!drawing) return;
    drawing = false;
    const cells = [...tileDraft].map(([offset, tile]) => ({ offset, tile }));
    const edits = [...collisionDraft].map(([offset, value]) => ({ offset, value }));
    if (cells.length) onStroke(cells);
    if (edits.length) onCollisionStroke(edits);
    tileDraft = new Map();
    collisionDraft = new Map();
    paintedCells = new Set();
    collisionWorking = null;
    anchor = null;
    previousCell = null;
  }

  function begin(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    const at = pointerCell(event);
    reportHover(event);
    beginAt(at, { pick: event.ctrlKey });
    if (tool === 'select' || drawing) canvas.setPointerCapture(event.pointerId);
  }

  function move(event: PointerEvent) {
    reportHover(event);
    moveAt(pointerCell(event));
  }

  function finish(event?: PointerEvent) {
    finishAt();
    if (event && canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
  }

  const KB_STROKE_TOOLS: MapTool[] = ['line', 'rect', 'rect-outline', 'select'];

  function handleKey(event: KeyboardEvent) {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight' || event.key === 'ArrowUp' || event.key === 'ArrowDown') {
      event.preventDefault();
      event.stopPropagation();
      const dx = event.key === 'ArrowLeft' ? -1 : event.key === 'ArrowRight' ? 1 : 0;
      const dy = event.key === 'ArrowUp' ? -1 : event.key === 'ArrowDown' ? 1 : 0;
      cursor = moveCursor(cursor, dx, dy, MAP_W, MAP_H);
      if (kbAnchor !== null) moveAt(cursor);
      return;
    }
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      event.stopPropagation();
      if (tool === 'pick') {
        pick(cursor);
        return;
      }
      if (KB_STROKE_TOOLS.includes(tool)) {
        if (kbAnchor === null) {
          kbAnchor = cursor;
          beginAt(cursor, { pick: false });
        } else {
          finishAt();
          kbAnchor = null;
        }
        return;
      }
      beginAt(cursor, { pick: false });
      finishAt();
      return;
    }
    if (event.key === 'Escape' && kbAnchor !== null) {
      event.preventDefault();
      event.stopPropagation();
      tileDraft = new Map();
      collisionDraft = new Map();
      collisionWorking = null;
      paintedCells = new Set();
      kbAnchor = null;
      drawing = false;
      anchor = null;
      previousCell = null;
      if (tool === 'select') { selectAnchor = null; selectCurrent = null; }
    }
  }
</script>

<div
  class="map-canvas-wrap"
  data-map-canvas
  style={`--map-zoom:${zoom}; --map-px-w:${MAP_PX_W}px; --map-px-h:${MAP_PX_H}px; --tile-pct-x:${100 / MAP_W}%; --tile-pct-y:${100 / MAP_H}%; --screen-pct-x:${screenPctX}%; --screen-pct-y:${screenPctY}%`}
>
  <canvas
    bind:this={canvas}
    class="map-canvas"
    class:collision={layer === 'collision'}
    class:picking={tool === 'pick'}
    class:erasing={tool === 'erase'}
    width={MAP_PX_W}
    height={MAP_PX_H}
    tabindex="0"
    aria-label={`${MAP_W} by ${MAP_H} tile map`}
    onpointerdown={begin}
    onpointermove={move}
    onpointerup={finish}
    onpointercancel={finish}
    onlostpointercapture={finish}
    onpointerleave={() => onHover?.(null)}
    onkeydown={handleKey}
    onfocus={() => focused = true}
    onblur={() => focused = false}
    oncontextmenu={(event) => event.preventDefault()}
  ></canvas>
  <div class="map-grid-overlay" aria-hidden="true"></div>
  <!-- The heavier lines in map-grid-overlay already mark every screen
       boundary; screen 0,0 gets the highlighted box because it's the camera a
       cart boots into, the rest just get a quiet coordinate label. -->
  <div class="map-screen-region" aria-hidden="true"><span>screen 0,0</span></div>
  {#each Array(screenCols * screenRows) as _, i}
    {@const sx = i % screenCols}
    {@const sy = Math.floor(i / screenCols)}
    {#if sx !== 0 || sy !== 0}
      <span class="screen-label" aria-hidden="true" style={`left:${sx * screenPctX}%; top:${sy * screenPctY}%`}>{sx},{sy}</span>
    {/if}
  {/each}
  {#if selectRegion}
    <div
      class="map-selection"
      aria-hidden="true"
      style={`left:${(selectRegion.x0 / MAP_W) * 100}%; top:${(selectRegion.y0 / MAP_H) * 100}%; width:${(selectRegion.w / MAP_W) * 100}%; height:${(selectRegion.h / MAP_H) * 100}%`}
    ></div>
  {/if}
  {#if focused}
    <div
      class="map-cursor"
      aria-hidden="true"
      style={`left:${((cursor % MAP_W) / MAP_W) * 100}%; top:${(Math.floor(cursor / MAP_W) / MAP_H) * 100}%; width:${(1 / MAP_W) * 100}%; height:${(1 / MAP_H) * 100}%`}
    ></div>
  {/if}
</div>

<style>
  .map-canvas-wrap { width: calc(var(--map-px-w) * var(--map-zoom)); height: calc(var(--map-px-h) * var(--map-zoom)); flex: none; position: relative; border: 1px solid var(--color-void-600); box-shadow: var(--shadow-lg); background: #000; }
  .map-canvas { width: 100%; height: 100%; display: block; image-rendering: pixelated; cursor: crosshair; touch-action: none; }
  .map-canvas.collision { cursor: cell; }
  .map-canvas.picking { cursor: copy; }
  .map-canvas.erasing { cursor: not-allowed; }
  .map-grid-overlay,
  .map-screen-region { position: absolute; pointer-events: none; }
  .map-grid-overlay {
    inset: 0;
    background-image:
      linear-gradient(to right, rgba(96,94,94,.35) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(96,94,94,.35) 1px, transparent 1px),
      linear-gradient(to right, rgba(245,242,242,.28) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(245,242,242,.28) 1px, transparent 1px);
    background-size: var(--tile-pct-x) 100%, 100% var(--tile-pct-y), var(--screen-pct-x) 100%, 100% var(--screen-pct-y);
  }
  .map-screen-region { left: 0; top: 0; width: var(--screen-pct-x); height: var(--screen-pct-y); border: 2px solid var(--color-ember); box-shadow: var(--shadow-glow-ember); }
  .map-screen-region span { position: absolute; left: 3px; top: 3px; color: var(--color-ember); font-family: var(--font-mono); font-size: 9px; letter-spacing: .06em; text-transform: uppercase; }
  .screen-label { position: absolute; padding: 2px 3px; color: rgba(245,242,242,.55); font-family: var(--font-mono); font-size: 8px; letter-spacing: .06em; text-transform: uppercase; pointer-events: none; }
  .map-selection { position: absolute; border: 1px dashed var(--color-ember); background: rgba(254,176,93,.12); pointer-events: none; }
  .map-cursor { position: absolute; outline: 2px solid var(--color-sheen-bright); outline-offset: -2px; pointer-events: none; }
</style>
