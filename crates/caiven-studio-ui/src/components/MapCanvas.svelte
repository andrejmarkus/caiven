<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    collisionCellEdits, strokeCells, type CollisionBrush, type CollisionEdit, type StrokeTool,
  } from '../lib/editorMath';
  import type { CollisionType } from '../types';

  type MapLayer = 'tiles' | 'collision';
  type MapTool = 'pencil' | 'fill' | 'rect' | 'pick' | 'erase' | 'line' | 'select';
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
  const selectRegion = $derived.by((): MapRegion | null => {
    if (selectAnchor === null || selectCurrent === null) return null;
    const ax = selectAnchor % 64, ay = Math.floor(selectAnchor / 64);
    const cx = selectCurrent % 64, cy = Math.floor(selectCurrent / 64);
    const x0 = Math.min(ax, cx), x1 = Math.max(ax, cx);
    const y0 = Math.min(ay, cy), y1 = Math.max(ay, cy);
    return { x0, y0, w: x1 - x0 + 1, h: y1 - y0 + 1 };
  });

  $effect(() => {
    if (tool !== 'select') { selectAnchor = null; selectCurrent = null; }
  });

  $effect(() => {
    onSelectionChange?.(selectRegion);
  });

  function color(hex: string): [number, number, number, number] {
    const value = hex || '#000000';
    return [parseInt(value.slice(1, 3), 16), parseInt(value.slice(3, 5), 16), parseInt(value.slice(5, 7), 16), 255];
  }

  function render() {
    if (!canvas) return;
    const context = canvas.getContext('2d');
    if (!context) return;
    const image = context.createImageData(512, 512);
    const colors = palette.map(color);
    for (let tileY = 0; tileY < 64; tileY += 1) for (let tileX = 0; tileX < 64; tileX += 1) {
      const offset = tileY * 64 + tileX;
      const tile = tileDraft.get(offset) ?? map[offset] ?? 0;
      if (tile !== 0) {
        for (let pixelY = 0; pixelY < 8; pixelY += 1) for (let pixelX = 0; pixelX < 8; pixelX += 1) {
          const paletteIndex = spriteSheet[tile * 64 + pixelY * 8 + pixelX] ?? 0;
          if (paletteIndex === 0) continue;
          const rgba = colors[paletteIndex] ?? colors[0] ?? [0, 0, 0, 255];
          const at = ((tileY * 8 + pixelY) * 512 + tileX * 8 + pixelX) * 4;
          image.data.set(rgba, at);
        }
      }
      const value = collisionDraft.get(offset) ?? collision[offset] ?? 0;
      const ctype = value !== 0 ? collisionTypes.find((t) => t.id === value) : undefined;
      if (showCollision && ctype) {
        const tint = ctype.color;
        const hatch = ctype.shape === 'none';
        for (let pixelY = 0; pixelY < 8; pixelY += 1) for (let pixelX = 0; pixelX < 8; pixelX += 1) {
          const border = pixelX <= 1 || pixelX >= 6 || pixelY <= 1 || pixelY >= 6;
          const dot = hatch && (pixelX + pixelY) % 4 === 0;
          const at = ((tileY * 8 + pixelY) * 512 + tileX * 8 + pixelX) * 4;
          const alpha = border || dot ? 0.85 : 0.3;
          image.data[at] = image.data[at] * (1 - alpha) + tint[0] * alpha;
          image.data[at + 1] = image.data[at + 1] * (1 - alpha) + tint[1] * alpha;
          image.data[at + 2] = image.data[at + 2] * (1 - alpha) + tint[2] * alpha;
          image.data[at + 3] = 255;
        }
      }
    }
    context.putImageData(image, 0, 0);
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
    scheduleRender();
  });

  onDestroy(() => {
    if (renderFrame !== undefined) clearTimeout(renderFrame);
  });

  function pointerCell(event: PointerEvent) {
    const rect = canvas.getBoundingClientRect();
    const x = Math.max(0, Math.min(63, Math.floor(((event.clientX - rect.left) / rect.width) * 64)));
    const y = Math.max(0, Math.min(63, Math.floor(((event.clientY - rect.top) / rect.height) * 64)));
    return y * 64 + x;
  }

  function reportHover(event: PointerEvent) {
    const at = pointerCell(event);
    onHover?.({ x: at % 64, y: Math.floor(at / 64), tile: tileDraft.get(at) ?? map[at] ?? 0 });
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

  function collisionValues(): number[] {
    const values = [...collision];
    for (const [offset, value] of collisionDraft) values[offset] = value;
    return values;
  }

  // Expands one path cell into the whole w×h stamp footprint anchored there
  // (top-left), clipped to the map bounds. Erasing clears every cell in the
  // footprint to 0 regardless of what the stamp's tiles are.
  function stampFootprint(base: number): Cell[] {
    const baseX = base % 64, baseY = Math.floor(base / 64);
    const cells: Cell[] = [];
    for (let dy = 0; dy < stamp.h; dy += 1) for (let dx = 0; dx < stamp.w; dx += 1) {
      const x = baseX + dx, y = baseY + dy;
      if (x >= 64 || y >= 64) continue;
      cells.push({ offset: y * 64 + x, tile: tool === 'erase' ? 0 : stamp.tiles[dy * stamp.w + dx] });
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
    for (const edit of collisionCellEdits(collisionValues(), offsets, activeCollisionBrush())) {
      next.set(edit.offset, edit.value);
    }
    collisionDraft = next;
  }

  function drawStroke(at: number) {
    // Never called with tool === 'pick'/'select' — begin() branches to pick()
    // or the select-marquee handling first for those.
    const drawTool: StrokeTool = tool as Exclude<MapTool, 'pick' | 'select'>;
    const values = layer === 'tiles' ? map : collisionValues();
    const replacement = layer === 'tiles' ? activeTile() : activeCollisionBrush();
    const offsets = strokeCells(drawTool, anchor ?? at, at, previousCell, values, replacement, 64, 64);
    // line/rect recompute the whole shape from anchor each move (live preview), so the
    // draft is replaced rather than accumulated; paint/erase/fill accumulate across a drag.
    if (tool === 'line' || tool === 'rect') {
      tileDraft = new Map();
      collisionDraft = new Map();
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

  function begin(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    const at = pointerCell(event);
    reportHover(event);
    if (event.ctrlKey || tool === 'pick') {
      pick(at);
      return;
    }
    if (tool === 'select') {
      selectAnchor = at;
      selectCurrent = at;
      drawing = true;
      canvas.setPointerCapture(event.pointerId);
      return;
    }
    tileDraft = new Map();
    collisionDraft = new Map();
    anchor = at;
    previousCell = at;
    drawing = true;
    if (tool === 'fill') {
      drawStroke(at);
      finish(event);
      return;
    }
    canvas.setPointerCapture(event.pointerId);
    drawStroke(at);
    render(); // paint inline — see move() for why this can't wait for scheduleRender()
  }

  function move(event: PointerEvent) {
    reportHover(event);
    if (!drawing) return;
    const at = pointerCell(event);
    if (tool === 'select') {
      selectCurrent = at;
      return;
    }
    // Paints happen synchronously, in the pointer handler itself, rather than deferring to
    // scheduleRender()'s timer: the timer callback still *runs* while the mouse button is
    // held, but WKWebView doesn't actually composite/flush the canvas to screen again until
    // the native tracking loop ends — confirmed by comparing a scheduled render (invisible
    // for the whole drag) against a synchronous one (paints every move) in the same build.
    if (tool === 'rect' || tool === 'line') {
      drawStroke(at);
      render();
    } else if ((tool === 'pencil' || tool === 'erase') && previousCell !== at) {
      drawStroke(at);
      previousCell = at;
      render();
    }
  }

  function finish(event?: PointerEvent) {
    if (!drawing) return;
    drawing = false;
    const cells = [...tileDraft].map(([offset, tile]) => ({ offset, tile }));
    const edits = [...collisionDraft].map(([offset, value]) => ({ offset, value }));
    if (cells.length) onStroke(cells);
    if (edits.length) onCollisionStroke(edits);
    tileDraft = new Map();
    collisionDraft = new Map();
    anchor = null;
    previousCell = null;
    if (event && canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
  }
</script>

<div class="map-canvas-wrap" data-map-canvas style={`--map-zoom:${zoom}`}>
  <canvas
    bind:this={canvas}
    class="map-canvas"
    class:collision={layer === 'collision'}
    class:picking={tool === 'pick'}
    class:erasing={tool === 'erase'}
    width="512"
    height="512"
    aria-label="64 by 64 tile map"
    onpointerdown={begin}
    onpointermove={move}
    onpointerup={finish}
    onpointercancel={finish}
    onlostpointercapture={finish}
    onpointerleave={() => onHover?.(null)}
    oncontextmenu={(event) => event.preventDefault()}
  ></canvas>
  <div class="map-grid-overlay" aria-hidden="true"></div>
  <!-- The heavier lines in map-grid-overlay already mark every 16-tile screen
       boundary; screen 0,0 gets the highlighted box because it's the camera a
       cart boots into, the rest just get a quiet coordinate label. -->
  <div class="map-screen-region" aria-hidden="true"><span>screen 0,0</span></div>
  {#each Array(16) as _, i}
    {@const sx = i % 4}
    {@const sy = Math.floor(i / 4)}
    {#if sx !== 0 || sy !== 0}
      <span class="screen-label" aria-hidden="true" style={`left:${sx * 25}%; top:${sy * 25}%`}>{sx},{sy}</span>
    {/if}
  {/each}
  {#if selectRegion}
    <div
      class="map-selection"
      aria-hidden="true"
      style={`left:${(selectRegion.x0 / 64) * 100}%; top:${(selectRegion.y0 / 64) * 100}%; width:${(selectRegion.w / 64) * 100}%; height:${(selectRegion.h / 64) * 100}%`}
    ></div>
  {/if}
</div>

<style>
  .map-canvas-wrap { width: calc(512px * var(--map-zoom)); height: calc(512px * var(--map-zoom)); flex: none; position: relative; border: 1px solid var(--color-void-600); box-shadow: var(--shadow-lg); background: #000; }
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
    background-size: calc(100% / 64) 100%, 100% calc(100% / 64), calc(100% / 4) 100%, 100% calc(100% / 4);
  }
  .map-screen-region { left: 0; top: 0; width: 25%; height: 25%; border: 2px solid var(--color-ember); box-shadow: var(--shadow-glow-ember); }
  .map-screen-region span { position: absolute; left: 3px; top: 3px; color: var(--color-ember); font-family: var(--font-mono); font-size: 9px; letter-spacing: .06em; text-transform: uppercase; }
  .screen-label { position: absolute; padding: 2px 3px; color: rgba(245,242,242,.55); font-family: var(--font-mono); font-size: 8px; letter-spacing: .06em; text-transform: uppercase; pointer-events: none; }
  .map-selection { position: absolute; border: 1px dashed var(--color-ember); background: rgba(254,176,93,.12); pointer-events: none; }
</style>
