<script lang="ts">
  import { untrack } from 'svelte';
  import { Maximize2, PanelRightClose, Plus, X } from '@lucide/svelte';
  import { Button } from '@caiven/ui/button';
  import { Input } from '@caiven/ui/input';
  import * as Tabs from '@caiven/ui/tabs';
  import type { CallFrame, Diagnostic, GlobalValue, PauseReason, RunState } from '../types';

  interface Props {
    runState: RunState;
    frame: number;
    fps: number;
    frameTime: number;
    frameData: Uint8Array | null;
    onFocus: () => void;
    /** Buttons currently held, from either the keyboard or these chips. */
    held: number[];
    onInput: (button: number, pressed: boolean) => void;
    globals: GlobalValue[];
    watches: GlobalValue[];
    callStack: CallFrame[];
    breakpointCount: number;
    diagnostics: Diagnostic[];
    pauseReason: PauseReason | null;
    onJumpToError: (diagnostic: Diagnostic) => void;
    onJumpToLocation: (source: string, line: number | null) => void;
    onAddWatch: (expression: string) => Promise<string | null>;
    onRemoveWatch: (expression: string) => void;
    onClose: () => void;
  }

  let { runState, frame, fps, frameTime, frameData, onFocus, held, onInput, globals, watches, callStack, breakpointCount, diagnostics, pauseReason, onJumpToError, onJumpToLocation, onAddWatch, onRemoveWatch, onClose }: Props = $props();
  let canvas: HTMLCanvasElement;
  let debugTab = $state<'watches' | 'globals' | 'stack'>('watches');
  let watchExpression = $state('');
  let watchError = $state('');
  let watchBusy = $state(false);
  const frameBarWindow = 48;
  let frameBars = $state<number[]>(Array(frameBarWindow).fill(4));

  const running = $derived(runState === 'running');
  const watchRows = $derived(watches.map((item) => [item.name, item.value]));

  $effect(() => {
    frameTime;
    if (!running) return;
    frameBars = [...untrack(() => frameBars).slice(1), frameTime];
  });
  const scriptError = $derived(diagnostics.find((item) => item.severity === 'error'));
  const keys = [
    ['↑ W', 0], ['↓ S', 1], ['← A', 2],
    ['→ D', 3], ['J  A', 4], ['K  B', 5],
  ] as const;

  function press(button: number) {
    onInput(button, true);
  }

  function release(button: number) {
    onInput(button, false);
  }

  async function submitWatch() {
    const expression = watchExpression.trim();
    if (!expression || watchBusy) return;
    watchBusy = true;
    watchError = await onAddWatch(expression) ?? '';
    watchBusy = false;
    if (!watchError) watchExpression = '';
  }

  function jumpFrame(location: string) {
    const colon = location.lastIndexOf(':');
    if (colon < 0) return;
    const line = Number(location.slice(colon + 1));
    onJumpToLocation(location.slice(0, colon), Number.isFinite(line) ? line : null);
  }

  $effect(() => {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    if (frameData?.length === 128 * 128 * 4) {
      ctx.putImageData(new ImageData(new Uint8ClampedArray(frameData), 128, 128), 0, 0);
      return;
    }
    ctx.fillStyle = '#080818';
    ctx.fillRect(0, 0, 128, 128);
    for (let y = 0; y < 16; y += 1) {
      for (let x = 0; x < 16; x += 1) {
        if (x === 0 || y === 0 || x === 15 || y === 15) ctx.fillStyle = '#5F574F';
        else if ((x * 7 + y * 13 + 93) % 11 === 0) ctx.fillStyle = '#008751';
        else ctx.fillStyle = '#1D2B53';
        ctx.fillRect(x * 8, y * 8, 8, 8);
      }
    }
    ctx.fillStyle = '#FFF1E8';
    ctx.fillRect(60, 56, 8, 8);
    ctx.fillStyle = '#FFEC27';
    ctx.fillRect(88, 32, 6, 6);
    ctx.font = '5px monospace';
    ctx.fillStyle = '#FFF1E8';
    ctx.fillText('SCORE 7', 3, 7);
  });
</script>

<aside class="console-pane">
  <div class="panel-cap">
    <span class="eyebrow">Console</span>
    <code>128 × 128 · <span class="scale-wide">4×</span><span class="scale-narrow">3×</span></code>
    <Button variant="ghost" size="sm" class="ghost-action" onclick={onFocus}><Maximize2 size={14} />Focus</Button>
    <Button variant="ghost" size="icon-sm" class="ghost-action icon-only" title="Hide console" onclick={onClose}><PanelRightClose size={14} /></Button>
  </div>

  <div class="screen-stage">
    <div class="console-screen" class:running>
      <canvas bind:this={canvas} width="128" height="128" aria-label="Cart framebuffer"></canvas>
      <div class="scanline-overlay"></div>
      <div class="crt-vignette"></div>
      {#if !running}
        <div class="pause-scrim">
          {#if scriptError}
            <span>Script error</span><strong>{scriptError.title}</strong><p>{scriptError.detail}</p><Button variant="link" size="sm" onclick={() => onJumpToError(scriptError)}>Jump to line</Button>
          {:else if pauseReason?.kind === 'breakpoint'}
            <span>Breakpoint</span><strong>{pauseReason.source}:{pauseReason.line ?? '?'}</strong><p>Execution paused before next frame.</p><Button variant="link" size="sm" onclick={() => onJumpToLocation(pauseReason.source ?? '', pauseReason.line)}>Jump to line</Button>
          {:else if runState === 'stopped'}
            <span>Stopped</span><strong>Cart not running</strong><p>Run cart to start VM.</p>
          {:else}
            <span>Paused</span><strong>Frame {frame.toLocaleString()}</strong><p>Step a frame at a time, or resume.</p>
          {/if}
        </div>
      {/if}
    </div>
    <div class="input-map">
      {#each keys as key}
        <button
          class:pressed={held.includes(key[1])}
          onpointerdown={(event) => { event.currentTarget.setPointerCapture(event.pointerId); press(key[1]); }}
          onpointerup={() => release(key[1])}
          onpointercancel={() => release(key[1])}
        >{key[0]}</button>
      {/each}
    </div>
  </div>

  <Tabs.Root value={debugTab} class="debugger gap-0">
    <Tabs.List variant="line" class="debug-tabs w-full">
      <Tabs.Trigger value="watches" class={debugTab === 'watches' ? 'active' : undefined} onclick={() => debugTab = 'watches'}>Watches</Tabs.Trigger>
      <Tabs.Trigger value="globals" class={debugTab === 'globals' ? 'active' : undefined} onclick={() => debugTab = 'globals'}>Globals</Tabs.Trigger>
      <Tabs.Trigger value="stack" class={debugTab === 'stack' ? 'active' : undefined} onclick={() => debugTab = 'stack'}>Call stack</Tabs.Trigger>
      <span>{breakpointCount} breakpoint{breakpointCount === 1 ? '' : 's'}</span>
    </Tabs.List>
    {#if debugTab === 'watches'}
      <div class="watch-list">
        {#each watchRows as watch}
          <div class="watch-row">
            <code>{watch[0]}</code><i>=</i><strong>{watch[1]}</strong><Button variant="ghost" size="icon-xs" title={`Remove ${watch[0]}`} onclick={() => onRemoveWatch(watch[0])}><X size={12} /></Button>
          </div>
        {/each}
        {#if !watchRows.length}<div class="watch-empty">No watches. Add Lua expression below.</div>{/if}
      </div>
      {#if watchError}<div class="watch-error" role="alert">{watchError}</div>{/if}
      <form class="add-watch" onsubmit={(event) => { event.preventDefault(); void submitWatch(); }}><Plus size={13} /><Input bind:value={watchExpression} placeholder="player.x" aria-label="Watch expression" oninput={() => watchError = ''} /><Button variant="outline" size="xs" disabled={watchBusy || !watchExpression.trim()}>{watchBusy ? '…' : 'Add'}</Button></form>
    {:else if debugTab === 'globals'}
      <div class="watch-list">
        {#each globals as global}
          <div class="watch-row"><code>{global.name}</code><i>=</i><strong>{global.value}</strong></div>
        {/each}
        {#if !globals.length}<div class="watch-empty">Pause cart to inspect globals.</div>{/if}
      </div>
    {:else}
      <div class="watch-list">
        {#if callStack.length}
          {#each callStack as frame}
            <Button variant="ghost" class="watch-row stack-frame" onclick={() => jumpFrame(frame.location)}>
              <strong>{frame.label}</strong><code>{frame.location}</code>
            </Button>
          {/each}
        {:else}
          <div class="watch-row"><span>Pause at a breakpoint to see the call stack.</span></div>
        {/if}
      </div>
    {/if}
  </Tabs.Root>

  <section class="frame-time">
    <div>
      <span class="eyebrow">Frame time</span>
      <strong>{running ? `${frameTime.toFixed(1)} ms` : '—'}</strong>
      <code>budget 16.6 ms</code>
    </div>
    <div class="frame-bars">
      {#each frameBars as value}
        <i class:hot={value > 8} style={`height:${Math.round((value / 16.6) * 38)}px`}></i>
      {/each}
    </div>
  </section>
</aside>
