<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    collisionFlagEdits, strokeCells, type CollisionBrush, type SpriteFlagEdit, type StrokeTool,
  } from '../lib/editorMath';

  type MapLayer = 'tiles' | 'collision';
  type MapTool = 'paint' | 'fill' | 'rect' | 'pick' | 'erase' | 'line';
  type Cell = { offset: number; tile: number };

  interface Props {
    map: number[];
    spriteSheet: number[];
    palette: string[];
    spriteFlags: number[];
    selectedTile: number;
    collision: boolean;
    layer: MapLayer;
    collisionBrush: CollisionBrush;
    tool: MapTool;
    zoom: number;
    onStroke: (cells: Cell[]) => void;
    onCollisionStroke: (edits: SpriteFlagEdit[]) => void;
    onPick: (tile: number) => void;
    onCollisionPick: (brush: CollisionBrush) => void;
    onHover?: (cell: { x: number; y: number; tile: number } | null) => void;
  }

  let {
    map, spriteSheet, palette, spriteFlags, selectedTile, collision, layer, collisionBrush,
    tool, zoom, onStroke, onCollisionStroke, onPick, onCollisionPick, onHover,
  }: Props = $props();
  let canvas: HTMLCanvasElement;
  let drawing = false;
  let anchor: number | null = null;
  let previousCell: number | null = null;
  let tileDraft = new Map<number, number>();
  let flagDraft = new Map<number, number>();
  let renderFrame: number | undefined;

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
      const flags = flagDraft.get(tile) ?? spriteFlags[tile] ?? 0;
      if (collision && (flags & 3) !== 0) {
        const hazard = (flags & 2) !== 0;
        const tint: [number, number, number, number] = hazard ? [229, 85, 95, 255] : [254, 176, 93, 255];
        for (let pixelY = 0; pixelY < 8; pixelY += 1) for (let pixelX = 0; pixelX < 8; pixelX += 1) {
          const border = pixelX === 0 || pixelX === 7 || pixelY === 0 || pixelY === 7;
          const hatch = hazard && (pixelX + pixelY) % 4 === 0;
          if (!border && !hatch) continue;
          const at = ((tileY * 8 + pixelY) * 512 + tileX * 8 + pixelX) * 4;
          image.data[at] = tint[0];
          image.data[at + 1] = tint[1];
          image.data[at + 2] = tint[2];
          image.data[at + 3] = 255;
        }
      }
    }
    context.putImageData(image, 0, 0);
  }

  function scheduleRender() {
    if (!canvas || renderFrame !== undefined) return;
    renderFrame = requestAnimationFrame(() => {
      renderFrame = undefined;
      render();
    });
  }

  $effect(() => {
    map; spriteSheet; palette; spriteFlags; collision; tileDraft; flagDraft; canvas;
    scheduleRender();
  });

  onDestroy(() => {
    if (renderFrame !== undefined) cancelAnimationFrame(renderFrame);
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

  function activeTile() {
    return tool === 'erase' ? 0 : selectedTile;
  }

  function activeCollisionBrush(): CollisionBrush {
    return tool === 'erase' ? 0 : collisionBrush;
  }

  function applyOffsets(offsets: readonly number[]) {
    if (layer === 'tiles') {
      const next = new Map(tileDraft);
      for (const offset of offsets) next.set(offset, activeTile());
      tileDraft = next;
      return;
    }
    const effectiveFlags = [...spriteFlags];
    for (const [tile, flags] of flagDraft) effectiveFlags[tile] = flags;
    const next = new Map(flagDraft);
    for (const edit of collisionFlagEdits(map, effectiveFlags, offsets, activeCollisionBrush())) {
      next.set(edit.tile, edit.flags);
    }
    flagDraft = next;
  }

  function collisionStates(): number[] {
    const effectiveFlags = [...spriteFlags];
    for (const [tile, flags] of flagDraft) effectiveFlags[tile] = flags;
    return map.map((tile) => (effectiveFlags[tile] ?? 0) & 3);
  }

  function drawStroke(at: number) {
    // Never called with tool === 'pick' — begin() branches to pick() first.
    const drawTool: StrokeTool = tool === 'paint' ? 'pencil' : (tool as Exclude<MapTool, 'paint' | 'pick'>);
    const values = layer === 'tiles' ? map : collisionStates();
    const replacement = layer === 'tiles' ? activeTile() : activeCollisionBrush();
    const offsets = strokeCells(drawTool, anchor ?? at, at, previousCell, values, replacement, 64, 64);
    // line/rect recompute the whole shape from anchor each move (live preview), so the
    // draft is replaced rather than accumulated; paint/erase/fill accumulate across a drag.
    if (tool === 'line' || tool === 'rect') {
      tileDraft = new Map();
      flagDraft = new Map();
    }
    applyOffsets(offsets);
  }

  function pick(at: number) {
    const tile = map[at] ?? 0;
    if (layer === 'tiles') {
      onPick(tile);
      return;
    }
    const flags = spriteFlags[tile] ?? 0;
    onCollisionPick((flags & 2) !== 0 ? 2 : (flags & 1) !== 0 ? 1 : 0);
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
    tileDraft = new Map();
    flagDraft = new Map();
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
  }

  function move(event: PointerEvent) {
    reportHover(event);
    if (!drawing) return;
    const at = pointerCell(event);
    if (tool === 'rect' || tool === 'line') {
      drawStroke(at);
    } else if ((tool === 'paint' || tool === 'erase') && previousCell !== at) {
      drawStroke(at);
      previousCell = at;
    }
  }

  function finish(event?: PointerEvent) {
    if (!drawing) return;
    drawing = false;
    const cells = [...tileDraft].map(([offset, tile]) => ({ offset, tile }));
    const flags = [...flagDraft].map(([tile, value]) => ({ tile, flags: value }));
    if (cells.length) onStroke(cells);
    if (flags.length) onCollisionStroke(flags);
    tileDraft = new Map();
    flagDraft = new Map();
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
  <div class="map-screen-region" aria-hidden="true"><span>screen 0,0</span></div>
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
</style>
