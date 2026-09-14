import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";
import { runInNewContext } from "node:vm";

// Test the checked-in artifact that actually ships, not an unrelated local build.
const runtimeUrl = new URL("../caiven-port/web/public/wasm/caiven_web.js", import.meta.url);
const runtimePath = fileURLToPath(runtimeUrl);
const sandbox = {
  module: { exports: {} }, require: createRequire(runtimeUrl),
  __dirname: dirname(runtimePath), __filename: runtimePath,
  process, console, performance, URL, TextDecoder, TextEncoder,
  setTimeout, clearTimeout, Buffer, WebAssembly,
};
const CaivenModule = runInNewContext(`${readFileSync(runtimeUrl, "utf8")}\n;CaivenModule;`, sandbox);
const wasmBytes = readFileSync(new URL("caiven_web.wasm", runtimeUrl));
const Module = await CaivenModule({
  // Exercise the same offline hook that Studio's exported HTML requires.
  instantiateWasm(imports, ready) {
    const instance = new WebAssembly.Instance(new WebAssembly.Module(wasmBytes), imports);
    ready(instance);
    return instance.exports;
  },
});

const rc = Module.ccall("caiven_new", "number", [], []);
if (rc !== 0) throw new Error(`caiven_new failed: ${rc}`);

const cartPath = process.argv[2] ?? new URL("../../carts/dev/smoke.cav", import.meta.url);
const bytes = readFileSync(cartPath);
const ptr = Module._malloc(bytes.length);
Module.HEAPU8.set(bytes, ptr);
const loadRc = Module.ccall(
  "caiven_load_cart",
  "number",
  ["number", "number"],
  [ptr, bytes.length],
);
Module._free(ptr);
if (loadRc !== 0) throw new Error(`caiven_load_cart failed: ${loadRc}`);

const width = Module.ccall("caiven_width", "number", [], []);
const height = Module.ccall("caiven_height", "number", [], []);
console.log(`dims: ${width}x${height}`);
if (width !== 192 || height !== 128) throw new Error("runtime framebuffer differs from console hardware");

for (let i = 0; i < 30; i++) {
  Module.ccall("caiven_tick", null, ["number"], [1]);
}

const pixPtr = Module.ccall("caiven_pixels", "number", [], []);
const buf = Module.HEAPU8.subarray(pixPtr, pixPtr + width * height * 4);
let checksum = 0;
let nonZero = 0;
for (const b of buf) {
  checksum = (checksum + b) >>> 0;
  if (b !== 0) nonZero++;
}
console.log(`checksum=${checksum} nonZeroBytes=${nonZero}/${buf.length}`);
if (checksum === 0) throw new Error("framebuffer is all zero after 30 frames");

Module.ccall("caiven_audio_fill", null, ["number", "number"], [256, 44100]);
const audioPtr = Module.ccall("caiven_audio_ptr", "number", [], []) / 4;
const samples = Module.HEAPF32.subarray(audioPtr, audioPtr + 256);
console.log(`audio: first sample=${samples[0]}`);

const hasFault = Module.ccall("caiven_has_fault", "number", [], []);
console.log(`hasFault=${hasFault}`);
if (hasFault !== 0) throw new Error("unexpected fault after a clean run");

// Validate the parser in the artifact itself, so a stale WASM binary cannot
// silently bypass the native cartridge hardening tests.
function expectRejected(invalid, label) {
  const allocation = Module._malloc(invalid.length);
  try {
    Module.HEAPU8.set(invalid, allocation);
    const result = Module.ccall("caiven_load_cart", "number", ["number", "number"], [allocation, invalid.length]);
    if (result === 0) throw new Error(`shipped runtime accepted ${label}`);
  } finally {
    Module._free(allocation);
  }
}

const oversized = Buffer.alloc(128 * 1024 + 1);
bytes.copy(oversized);
expectRejected(oversized, "oversized cartridge");

const overlapping = Buffer.from(bytes);
const sectionCount = overlapping.readUInt16LE(8);
let payloadEntry;
for (let i = 1; i < sectionCount; i++) {
  const entry = 82 + i * 14;
  if (overlapping.readUInt32LE(entry + 6) > 0) { payloadEntry = entry; break; }
}
if (payloadEntry === undefined) throw new Error("smoke fixture needs a nonempty asset section");
// Give Program the same range and valid CRC as an asset without changing kinds.
overlapping.copy(overlapping, 84, payloadEntry + 2, payloadEntry + 14);
expectRejected(overlapping, "overlapping cartridge sections");

// A bad upload must not prevent a subsequent valid load.
const valid = Module._malloc(bytes.length);
try {
  Module.HEAPU8.set(bytes, valid);
  if (Module.ccall("caiven_load_cart", "number", ["number", "number"], [valid, bytes.length]) !== 0) {
    throw new Error("runtime failed to recover after invalid cartridge");
  }
} finally {
  Module._free(valid);
}
console.log("OK (rendering, audio, parser rejection, recovery)");
