import { readFileSync } from "node:fs";
import CaivenModule from "../../target/wasm32-unknown-emscripten/release/caiven_web.js";

const Module = await CaivenModule();

const rc = Module.ccall("caiven_new", "number", [], []);
if (rc !== 0) throw new Error(`caiven_new failed: ${rc}`);

const cartPath = process.argv[2] ?? "../../games/carts/stdlib_demo.cav";
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
console.log("OK");
