<script lang="ts">
  import { onDestroy } from 'svelte';
  import { strokeCells, type StrokeTool } from '../lib/editorMath';

  export type SpriteTool = StrokeTool | 'pick';
  export type Pixel = { index: number; color: number };

  interface Props {
    sprite: number[];
    palette: string[];
    selectedColor: number;
    tool: SpriteTool;
    onStroke: (pixels: Pixel[]) => void;
    onPick: (color: number) => void;
  }

  const { sprite, palette, selectedColor, tool, onStroke, onPick }: Props = $props();

  const CELL = 32;
  const SIZE = CELL * 8;

  let canvas: HTMLCanvasElement;
  let drawing = false;
  let anchor: number | null = null;
  let previousPixel: number | null = null;
  let draft = new Map<number, number>();
  let renderFrame: number | undefined;

  function color(hex: string): [number, number, number, number] {
    const value = hex || '#000000';
    return [parseInt(value.slice(1, 3), 16), parseInt(value.slice(3, 5), 16), parseInt(value.slice(5, 7), 16), 255];
  }

  function render() {
    if (!canvas) return;
    const context = canvas.getContext('2d');
    if (!context) return;
    const colors = palette.map(color);
    for (let y = 0; y < 8; y += 1) for (let x = 0; x < 8; x += 1) {
      const index = y * 8 + x;
      const value = draft.get(index) ?? sprite[index] ?? 0;
      const rgba = colors[value] ?? colors[0] ?? [0, 0, 0, 255];
      context.fillStyle = `rgba(${rgba[0]},${rgba[1]},${rgba[2]},${rgba[3] / 255})`;
      context.fillRect(x * CELL, y * CELL, CELL, CELL);
    }
  }

  function scheduleRender() {
    if (!canvas || renderFrame !== undefined) return;
    renderFrame = requestAnimationFrame(() => {
      renderFrame = undefined;
      render();
    });
  }

  $effect(() => {
    sprite; palette; draft; canvas;
    scheduleRender();
  });

  onDestroy(() => {
    if (renderFrame !== undefined) cancelAnimationFrame(renderFrame);
  });

  function pointerPixel(event: PointerEvent) {
    const rect = canvas.getBoundingClientRect();
    const x = Math.max(0, Math.min(7, Math.floor(((event.clientX - rect.left) / rect.width) * 8)));
    const y = Math.max(0, Math.min(7, Math.floor(((event.clientY - rect.top) / rect.height) * 8)));
    return y * 8 + x;
  }

  function activeColor() {
    return tool === 'erase' ? 0 : selectedColor;
  }

  function applyStroke(at: number) {
    const drawTool: StrokeTool = tool === 'pick' ? 'pencil' : tool;
    const offsets = strokeCells(drawTool, anchor ?? at, at, previousPixel, sprite, activeColor(), 8, 8);
    // line/rect recompute the whole shape from anchor each move (live preview), so the
    // draft is replaced rather than accumulated; pencil/erase/fill accumulate across a drag.
    const next = tool === 'line' || tool === 'rect' ? new Map<number, number>() : new Map(draft);
    for (const offset of offsets) next.set(offset, activeColor());
    draft = next;
  }

  function pick(at: number) {
    onPick(sprite[at] ?? 0);
  }

  function begin(event: PointerEvent) {
    if (event.button !== 0 && event.button !== 2) return;
    event.preventDefault();
    const at = pointerPixel(event);
    if (event.button === 2 || event.ctrlKey || tool === 'pick') {
      pick(at);
      return;
    }
    draft = new Map();
    anchor = at;
    previousPixel = at;
    drawing = true;
    if (tool === 'fill') {
      applyStroke(at);
      finish(event);
      return;
    }
    canvas.setPointerCapture(event.pointerId);
    applyStroke(at);
  }

  function move(event: PointerEvent) {
    if (!drawing) return;
    const at = pointerPixel(event);
    if (tool === 'rect' || tool === 'line') {
      applyStroke(at);
    } else if ((tool === 'pencil' || tool === 'erase') && previousPixel !== at) {
      applyStroke(at);
      previousPixel = at;
    }
  }

  function finish(event?: PointerEvent) {
    if (!drawing) return;
    drawing = false;
    const pixels = [...draft].map(([index, value]) => ({ index, color: value }));
    if (pixels.length) onStroke(pixels);
    draft = new Map();
    anchor = null;
    previousPixel = null;
    if (event && canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
  }
</script>

<div class="sprite-canvas-wrap" data-sprite-canvas>
  <canvas
    bind:this={canvas}
    class="sprite-canvas"
    class:picking={tool === 'pick'}
    class:erasing={tool === 'erase'}
    width={SIZE}
    height={SIZE}
    aria-label="8 by 8 sprite grid"
    onpointerdown={begin}
    onpointermove={move}
    onpointerup={finish}
    onpointercancel={finish}
    onlostpointercapture={finish}
    oncontextmenu={(event) => event.preventDefault()}
  ></canvas>
  <div class="sprite-grid-overlay" aria-hidden="true"></div>
</div>

<style>
  .sprite-canvas-wrap { width: 256px; height: 256px; flex: none; position: relative; border: 1px solid var(--color-void-600); box-shadow: var(--shadow-lg); background: #000; }
  .sprite-canvas { width: 100%; height: 100%; display: block; image-rendering: pixelated; cursor: crosshair; touch-action: none; }
  .sprite-canvas.picking { cursor: copy; }
  .sprite-canvas.erasing { cursor: not-allowed; }
  .sprite-grid-overlay {
    position: absolute; inset: 0; pointer-events: none;
    background-image:
      linear-gradient(to right, rgba(96,94,94,.35) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(96,94,94,.35) 1px, transparent 1px);
    background-size: calc(100% / 8) 100%, 100% calc(100% / 8);
  }
</style>
