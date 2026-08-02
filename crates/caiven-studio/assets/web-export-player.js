// Vanilla (framework-free) cart player for Caiven's self-contained HTML
// export. Derived from crates/caiven-web/test.html (ccall sequence, render
// loop, key->button map) and the AudioEngine in
// crates/caiven-port/web/src/player.ts (worklet pacing) — kept in sync with
// both by hand since this file has no bundler/import access to either.
//
// Everything this script needs is inlined by web_export.rs as globals before
// this file runs, so there is no fetch()/XHR anywhere below (SPEC V19):
//   window.__CAIVEN_WASM_B64    - base64 of caiven_web.wasm
//   window.__CAIVEN_CART_B64    - base64 of the packed .cav cartridge
//   window.__CAIVEN_WORKLET_B64 - base64 of caiven-audio-worklet.js source
// `CaivenModule` (the emscripten glue, MODULARIZE=1/EXPORT_NAME=CaivenModule)
// is expected to already be a global from an earlier inlined <script>.
(function () {
  "use strict";

  function b64ToBytes(b64) {
    const bin = atob(b64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return bytes;
  }

  // Button indices per crates/caiven-vm/src/input/button.rs.
  const KEY_TO_BUTTON = {
    ArrowUp: 0, w: 0, W: 0,
    ArrowDown: 1, s: 1, S: 1,
    ArrowLeft: 2, a: 2, A: 2,
    ArrowRight: 3, d: 3, D: 3,
    j: 4, z: 4, Z: 4,
    k: 5, x: 5, X: 5,
  };

  // Standard-gamepad mapping button index -> Caiven button.
  const GAMEPAD_TO_BUTTON = { 12: 0, 13: 1, 14: 2, 15: 3, 0: 4, 1: 5 };

  // Renders audio on the main thread (the only place the emscripten module
  // lives) and hands pre-rendered PCM chunks to an AudioWorklet for
  // playback — mirrors AudioEngine in caiven-port/web/src/player.ts, with
  // the worklet module loaded from an inlined Blob URL instead of a
  // same-origin path (no network fetch allowed in the exported html).
  function AudioEngine(module, workletBlobUrl) {
    this.module = module;
    this.workletBlobUrl = workletBlobUrl;
    this.ctx = null;
    this.node = null;
    this.nextChunkTime = 0;
    this.muted = false;
  }
  AudioEngine.LOOKAHEAD_SEC = 0.015;
  AudioEngine.prototype.ensureStarted = function () {
    if (this.muted) return;
    if (this.ctx) {
      if (this.ctx.state === "suspended") this.ctx.resume();
      return;
    }
    const AudioCtx = window.AudioContext || window.webkitAudioContext;
    const ctx = new AudioCtx();
    this.ctx = ctx;
    const self = this;
    ctx.audioWorklet.addModule(this.workletBlobUrl).then(function () {
      const node = new AudioWorkletNode(ctx, "caiven-audio-processor", {
        numberOfInputs: 0,
        numberOfOutputs: 1,
        outputChannelCount: [1],
      });
      node.connect(ctx.destination);
      self.node = node;
      self.nextChunkTime = ctx.currentTime;
    }).catch(function (err) {
      // Without this, a one-time addModule failure (e.g. a sandboxed embed
      // that disallows worklet modules loaded from a blob: URL) would
      // permanently mute audio for the rest of the session: ensureStarted()
      // short-circuits as soon as `this.ctx` is truthy, and it was set
      // synchronously above, before this promise settled. Clearing it back
      // to null lets the next ensureStarted() call (next click/keypress)
      // retry from scratch instead of silently giving up forever.
      console.error("caiven: audio worklet failed to load, will retry on next interaction", err);
      void ctx.close();
      self.ctx = null;
    });
  };
  AudioEngine.prototype.pump = function () {
    if (!this.ctx || !this.node) return;
    const sampleRate = this.ctx.sampleRate;
    if (this.nextChunkTime < this.ctx.currentTime) this.nextChunkTime = this.ctx.currentTime;
    while (this.nextChunkTime < this.ctx.currentTime + AudioEngine.LOOKAHEAD_SEC) {
      const numFrames = Math.ceil(sampleRate / 60);
      this.module.ccall("caiven_audio_fill", null, ["number", "number"], [numFrames, sampleRate]);
      const ptr = this.module.ccall("caiven_audio_ptr", "number", [], []) / 4;
      const chunk = this.module.HEAPF32.slice(ptr, ptr + numFrames);
      this.node.port.postMessage(chunk, [chunk.buffer]);
      this.nextChunkTime += numFrames / sampleRate;
    }
  };

  function boot() {
    const cartBytes = b64ToBytes(window.__CAIVEN_CART_B64);
    const wasmBytes = b64ToBytes(window.__CAIVEN_WASM_B64);
    const workletSrc = new TextDecoder().decode(b64ToBytes(window.__CAIVEN_WORKLET_B64));
    const workletBlobUrl = URL.createObjectURL(new Blob([workletSrc], { type: "application/javascript" }));

    const statusEl = document.getElementById("status");
    const setStatus = function (msg) {
      if (statusEl) statusEl.textContent = msg;
    };

    // This emscripten build's glue keeps `wasmBinary` as a module-scope var
    // it populates itself from a fetch — it never reads back a
    // `Module.wasmBinary` override, so setting that field (an older/other
    // builds' documented override) silently does nothing here. The hook
    // this build *does* honor is `Module.instantiateWasm`: supplying it
    // fully replaces the default fetch-then-compile path, which is what
    // makes a single-file, network-free export possible.
    window
      .CaivenModule({
        instantiateWasm: function (imports, successCallback) {
          WebAssembly.instantiate(wasmBytes, imports).then(function (output) {
            successCallback(output.instance);
          });
          return {};
        },
      })
      .then(function (module) {
        const newRc = module.ccall("caiven_new", "number", [], []);
        if (newRc !== 0) throw new Error("caiven_new failed: " + newRc);

        const ptr = module._malloc(cartBytes.length);
        module.HEAPU8.set(cartBytes, ptr);
        const loadRc = module.ccall(
          "caiven_load_cart",
          "number",
          ["number", "number"],
          [ptr, cartBytes.length],
        );
        module._free(ptr);
        if (loadRc !== 0) throw new Error("caiven_load_cart failed: " + loadRc);

        const width = module.ccall("caiven_width", "number", [], []);
        const height = module.ccall("caiven_height", "number", [], []);

        const canvas = document.getElementById("screen");
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext("2d");
        const audio = new AudioEngine(module, workletBlobUrl);

        function setButton(btn, down) {
          module.ccall("caiven_set_button", null, ["number", "number"], [btn, down ? 1 : 0]);
        }

        canvas.tabIndex = 0;
        canvas.addEventListener("click", function () {
          canvas.focus();
          audio.ensureStarted();
        });
        canvas.focus();

        // This export is meant to be embeddable (itch.io project pages,
        // portfolio sites) alongside other interactive elements, so the
        // listeners have to live on `window` (there's no guarantee this
        // <canvas> owns the whole page) — but that means they must check
        // canvas focus themselves, or every WASD/arrow keypress on the
        // embedding page gets hijacked regardless of what the user actually
        // has focused. keydown is gated on focus; keyup always still
        // releases the button even if focus moved away mid-press, so a key
        // can't get stuck "held" — and window blur clears everything as a
        // second line of defense.
        window.addEventListener("keydown", function (e) {
          if (document.activeElement !== canvas) return;
          const btn = KEY_TO_BUTTON[e.key];
          if (btn === undefined) return;
          e.preventDefault();
          audio.ensureStarted();
          setButton(btn, true);
        });
        window.addEventListener("keyup", function (e) {
          const btn = KEY_TO_BUTTON[e.key];
          if (btn === undefined) return;
          if (document.activeElement === canvas) e.preventDefault();
          setButton(btn, false);
        });
        window.addEventListener("blur", function () {
          for (const btn of Object.values(KEY_TO_BUTTON)) setButton(btn, false);
        });

        let gamepadIndex = null;
        let gamepadPrev = new Set();
        window.addEventListener("gamepadconnected", function (e) {
          if (gamepadIndex === null) gamepadIndex = e.gamepad.index;
        });
        window.addEventListener("gamepaddisconnected", function (e) {
          if (gamepadIndex === e.gamepad.index) gamepadIndex = null;
        });
        function pollGamepad() {
          if (gamepadIndex === null) return;
          const pad = navigator.getGamepads()[gamepadIndex];
          if (!pad) return;
          const pressed = new Set();
          for (const padBtn in GAMEPAD_TO_BUTTON) {
            if (pad.buttons[padBtn] && pad.buttons[padBtn].pressed) {
              pressed.add(GAMEPAD_TO_BUTTON[padBtn]);
            }
          }
          pressed.forEach(function (btn) {
            if (!gamepadPrev.has(btn)) setButton(btn, true);
          });
          gamepadPrev.forEach(function (btn) {
            if (!pressed.has(btn)) setButton(btn, false);
          });
          gamepadPrev = pressed;
        }

        let faulted = false;
        setStatus("");
        function frame() {
          pollGamepad();
          if (!faulted) {
            module.ccall("caiven_tick", null, ["number"], [1]);
            audio.pump();
            const hasFault = module.ccall("caiven_has_fault", "number", [], []);
            if (hasFault) {
              faulted = true;
              const len = module.ccall("caiven_fault_len", "number", [], []);
              const faultPtr = module.ccall("caiven_fault_ptr", "number", [], []);
              const message = new TextDecoder().decode(module.HEAPU8.subarray(faultPtr, faultPtr + len));
              setStatus("cart error: " + message);
            }
          }
          const pixPtr = module.ccall("caiven_pixels", "number", [], []);
          const buf = module.HEAPU8.subarray(pixPtr, pixPtr + width * height * 4);
          ctx.putImageData(new ImageData(new Uint8ClampedArray(buf), width, height), 0, 0);
          requestAnimationFrame(frame);
        }
        requestAnimationFrame(frame);
      })
      .catch(function (err) {
        setStatus("failed to start: " + err.message);
        console.error(err);
      });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
