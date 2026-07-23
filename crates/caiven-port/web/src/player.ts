// Wraps the emscripten-built caiven-web module (crates/caiven-web) for use
// from a Svelte page. Button indices mirror crates/caiven-vm/src/input/button.rs.

interface CaivenModuleInstance {
  ccall: (name: string, ret: string | null, argTypes: string[], args: unknown[]) => unknown;
  _malloc: (size: number) => number;
  _free: (ptr: number) => void;
  HEAPU8: Uint8Array;
  HEAPF32: Float32Array;
}

declare global {
  interface Window {
    CaivenModule?: () => Promise<CaivenModuleInstance>;
  }
}

const KEY_TO_BUTTON: Record<string, number> = {
  ArrowUp: 0,
  w: 0,
  W: 0,
  ArrowDown: 1,
  s: 1,
  S: 1,
  ArrowLeft: 2,
  a: 2,
  A: 2,
  ArrowRight: 3,
  d: 3,
  D: 3,
  j: 4,
  z: 4,
  Z: 4,
  k: 5,
  x: 5,
  X: 5,
};

// Standard-gamepad mapping button index -> Caiven button.
const GAMEPAD_TO_BUTTON: Record<number, number> = {
  12: 0, // d-pad up
  13: 1, // d-pad down
  14: 2, // d-pad left
  15: 3, // d-pad right
  0: 4, // A / bottom face button
  1: 5, // B / right face button
};

// Button labels for the on-screen touch d-pad, in Caiven button-index order.
const TOUCH_DPAD: Array<{ btn: number; cls: string; label: string }> = [
  { btn: 0, cls: 'up', label: '▲' },
  { btn: 1, cls: 'down', label: '▼' },
  { btn: 2, cls: 'left', label: '◀' },
  { btn: 3, cls: 'right', label: '▶' },
];
const TOUCH_FACE: Array<{ btn: number; cls: string; label: string }> = [
  { btn: 5, cls: 'b', label: 'B' },
  { btn: 4, cls: 'a', label: 'A' },
];

let scriptLoadPromise: Promise<void> | null = null;

function loadScript(src: string): Promise<void> {
  if (scriptLoadPromise) return scriptLoadPromise;
  scriptLoadPromise = new Promise((resolve, reject) => {
    const el = document.createElement('script');
    el.src = src;
    el.onload = () => resolve();
    el.onerror = () => reject(new Error(`failed to load ${src}`));
    document.body.appendChild(el);
  });
  return scriptLoadPromise;
}

// Renders audio on the main thread (the only place the emscripten module
// lives) and hands pre-rendered PCM chunks to an AudioWorklet
// (public/caiven-audio-worklet.js) for playback — the worklet's separate
// realm never touches wasm directly, it just plays back what it's given.
class AudioEngine {
  private module: CaivenModuleInstance;
  private ctx: AudioContext | null = null;
  private node: AudioWorkletNode | null = null;
  // How far ahead of the audio clock we've scheduled samples. rAF frequency
  // tracks the display's refresh rate, not the audio clock — on a >60Hz
  // monitor a naive "one ~16ms chunk per tick" schedule overproduces audio
  // faster than it plays back, so the queue (and latency) grows without
  // bound. Pacing off ctx.currentTime instead keeps production matched to
  // real playback regardless of rAF rate. Each chunk renders from whatever
  // Sound state exists at render time, so LOOKAHEAD_SEC also doubles as the
  // worst-case button-press-to-sound delay — kept small on purpose.
  private nextChunkTime = 0;
  private static readonly LOOKAHEAD_SEC = 0.03;

  constructor(module: CaivenModuleInstance) {
    this.module = module;
  }

  ensureStarted(): void {
    if (this.ctx) {
      if (this.ctx.state === 'suspended') void this.ctx.resume();
      return;
    }
    const AudioCtx = window.AudioContext ?? (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    const ctx = new AudioCtx();
    this.ctx = ctx;
    void ctx.audioWorklet.addModule('/caiven-audio-worklet.js').then(() => {
      const node = new AudioWorkletNode(ctx, 'caiven-audio-processor', {
        numberOfInputs: 0,
        numberOfOutputs: 1,
        outputChannelCount: [1],
      });
      node.connect(ctx.destination);
      this.node = node;
      this.nextChunkTime = ctx.currentTime;
    });
  }

  /// Tops up the worklet's queue to stay ~LOOKAHEAD_SEC ahead of the audio
  /// clock (`ctx.currentTime`), scheduling nothing if already buffered far
  /// enough — called once per rAF tick, a no-op until the worklet module has
  /// finished loading. Driving this off the audio clock rather than a fixed
  /// per-tick chunk keeps latency bounded and self-corrects after any stall
  /// (a hidden tab, GC pause) instead of drifting further behind forever.
  pump(): void {
    if (!this.ctx || !this.node) return;
    const sampleRate = this.ctx.sampleRate;
    if (this.nextChunkTime < this.ctx.currentTime) {
      this.nextChunkTime = this.ctx.currentTime;
    }
    while (this.nextChunkTime < this.ctx.currentTime + AudioEngine.LOOKAHEAD_SEC) {
      const numFrames = Math.ceil(sampleRate / 60);
      this.module.ccall('caiven_audio_fill', null, ['number', 'number'], [numFrames, sampleRate]);
      const ptr = (this.module.ccall('caiven_audio_ptr', 'number', [], []) as number) / 4;
      // .slice() (not .subarray()) — must copy out of the wasm heap before
      // transferring, since a transfer would detach the shared buffer.
      const chunk = this.module.HEAPF32.slice(ptr, ptr + numFrames);
      this.node.port.postMessage(chunk, [chunk.buffer]);
      this.nextChunkTime += numFrames / sampleRate;
    }
  }

  stop(): void {
    this.node?.disconnect();
    this.node = null;
    void this.ctx?.close();
    this.ctx = null;
    this.nextChunkTime = 0;
  }
}

export class CartPlayer {
  private module: CaivenModuleInstance;
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private width: number;
  private height: number;
  private rafId = 0;
  private gamepadIndex: number | null = null;
  private gamepadPrevState = new Set<number>();
  private audio: AudioEngine;
  private faulted = false;
  private onFault: ((message: string) => void) | null = null;
  private touchEls: HTMLElement[] = [];

  private constructor(module: CaivenModuleInstance, canvas: HTMLCanvasElement, width: number, height: number) {
    this.module = module;
    this.canvas = canvas;
    this.width = width;
    this.height = height;
    canvas.width = width;
    canvas.height = height;
    this.ctx = canvas.getContext('2d')!;
    this.audio = new AudioEngine(module);
  }

  static async load(canvas: HTMLCanvasElement, cartBytes: Uint8Array): Promise<CartPlayer> {
    await loadScript('/wasm/caiven_web.js');
    if (!window.CaivenModule) throw new Error('caiven_web.js did not register CaivenModule');
    const module = await window.CaivenModule();

    const newRc = module.ccall('caiven_new', 'number', [], []);
    if (newRc !== 0) throw new Error(`caiven_new failed: ${newRc}`);

    const ptr = module._malloc(cartBytes.length);
    module.HEAPU8.set(cartBytes, ptr);
    const loadRc = module.ccall('caiven_load_cart', 'number', ['number', 'number'], [ptr, cartBytes.length]);
    module._free(ptr);
    if (loadRc !== 0) throw new Error(`caiven_load_cart failed: ${loadRc}`);

    const width = module.ccall('caiven_width', 'number', [], []) as number;
    const height = module.ccall('caiven_height', 'number', [], []) as number;

    return new CartPlayer(module, canvas, width, height);
  }

  setButton(button: number, down: boolean): void {
    this.module.ccall('caiven_set_button', null, ['number', 'number'], [button, down ? 1 : 0]);
  }

  private onKeyDown = (e: KeyboardEvent): void => {
    const btn = KEY_TO_BUTTON[e.key];
    if (btn === undefined) return;
    e.preventDefault();
    this.audio.ensureStarted();
    this.setButton(btn, true);
  };

  private onKeyUp = (e: KeyboardEvent): void => {
    const btn = KEY_TO_BUTTON[e.key];
    if (btn === undefined) return;
    e.preventDefault();
    this.setButton(btn, false);
  };

  private onGamepadConnected = (e: GamepadEvent): void => {
    this.gamepadIndex ??= e.gamepad.index;
  };

  private onGamepadDisconnected = (e: GamepadEvent): void => {
    if (this.gamepadIndex === e.gamepad.index) this.gamepadIndex = null;
  };

  private pollGamepad(): void {
    if (this.gamepadIndex === null) return;
    const pad = navigator.getGamepads()[this.gamepadIndex];
    if (!pad) return;
    const pressed = new Set<number>();
    for (const [padBtn, caivenBtn] of Object.entries(GAMEPAD_TO_BUTTON)) {
      if (pad.buttons[Number(padBtn)]?.pressed) pressed.add(caivenBtn);
    }
    for (const btn of pressed) {
      if (!this.gamepadPrevState.has(btn)) this.setButton(btn, true);
    }
    for (const btn of this.gamepadPrevState) {
      if (!pressed.has(btn)) this.setButton(btn, false);
    }
    this.gamepadPrevState = pressed;
  }

  /// Builds an on-screen d-pad + A/B overlay inside `container` (hidden via
  /// CSS on non-touch viewports — always mounted so no layout-shift on the
  /// pointer-type change some hybrid devices report).
  mountTouchControls(container: HTMLElement): void {
    const mkButton = (cls: string, label: string, btn: number): HTMLElement => {
      const el = document.createElement('div');
      el.className = `touch-btn ${cls}`;
      el.textContent = label;
      const press = (e: Event) => {
        e.preventDefault();
        this.audio.ensureStarted();
        this.setButton(btn, true);
      };
      const release = (e: Event) => {
        e.preventDefault();
        this.setButton(btn, false);
      };
      el.addEventListener('pointerdown', press);
      el.addEventListener('pointerup', release);
      el.addEventListener('pointerleave', release);
      el.addEventListener('pointercancel', release);
      this.touchEls.push(el);
      return el;
    };

    const dpad = document.createElement('div');
    dpad.className = 'touch-dpad';
    for (const { btn, cls, label } of TOUCH_DPAD) dpad.appendChild(mkButton(cls, label, btn));

    const face = document.createElement('div');
    face.className = 'touch-face';
    for (const { btn, cls, label } of TOUCH_FACE) face.appendChild(mkButton(cls, label, btn));

    container.appendChild(dpad);
    container.appendChild(face);
  }

  start(onFault?: (message: string) => void): void {
    this.onFault = onFault ?? null;
    window.addEventListener('keydown', this.onKeyDown);
    window.addEventListener('keyup', this.onKeyUp);
    window.addEventListener('gamepadconnected', this.onGamepadConnected);
    window.addEventListener('gamepaddisconnected', this.onGamepadDisconnected);
    this.canvas.tabIndex = 0;
    this.canvas.addEventListener('click', () => {
      this.canvas.focus();
      this.audio.ensureStarted();
    });
    this.canvas.focus();

    const frame = () => {
      this.pollGamepad();
      if (!this.faulted) {
        this.module.ccall('caiven_tick', null, ['number'], [1]);
        this.audio.pump();
        const hasFault = this.module.ccall('caiven_has_fault', 'number', [], []) as number;
        if (hasFault) {
          this.faulted = true;
          const len = this.module.ccall('caiven_fault_len', 'number', [], []) as number;
          const ptr = this.module.ccall('caiven_fault_ptr', 'number', [], []) as number;
          const message = new TextDecoder().decode(this.module.HEAPU8.subarray(ptr, ptr + len));
          this.onFault?.(message);
        }
      }
      const pixPtr = this.module.ccall('caiven_pixels', 'number', [], []) as number;
      const buf = this.module.HEAPU8.subarray(pixPtr, pixPtr + this.width * this.height * 4);
      const imageData = new ImageData(new Uint8ClampedArray(buf), this.width, this.height);
      this.ctx.putImageData(imageData, 0, 0);
      this.rafId = requestAnimationFrame(frame);
    };
    this.rafId = requestAnimationFrame(frame);
  }

  stop(): void {
    cancelAnimationFrame(this.rafId);
    window.removeEventListener('keydown', this.onKeyDown);
    window.removeEventListener('keyup', this.onKeyUp);
    window.removeEventListener('gamepadconnected', this.onGamepadConnected);
    window.removeEventListener('gamepaddisconnected', this.onGamepadDisconnected);
    this.audio.stop();
    for (const el of this.touchEls) el.remove();
    this.touchEls = [];
  }
}
