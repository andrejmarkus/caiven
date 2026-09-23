// Minimal `.cav` reader/writer mirroring crates/caiven-cart/src/format.rs, so
// Quick Remix can swap the LuaSource section in the browser and hand the
// result to the same WASM runtime. Only section contents are touched; every
// other section is carried through byte-for-byte.

const MAGIC = 'CAIVEN';
const FORMAT_VERSION = 5;
const FIXED_HDR = 82;
const ENTRY_LEN = 14;
const FIELD_LEN = 32;
export const LUA_SOURCE = 0x000a;
export const PROGRAM = 0x0001;
export const MAX_CART_BYTES = 128 * 1024;

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

/** @param {Uint8Array} bytes */
export function crc32(bytes) {
  let crc = 0xffffffff;
  for (const b of bytes) crc = CRC_TABLE[(crc ^ b) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

/**
 * @typedef {{ kind: number, data: Uint8Array }} Section
 * @typedef {{ version: number, title: string, author: string, entry: number, flags: number, sections: Section[] }} Cav
 */

/** @param {Uint8Array} bytes @param {number} start */
function readField(bytes, start) {
  const field = bytes.subarray(start, start + FIELD_LEN);
  const end = field.indexOf(0);
  return new TextDecoder().decode(end === -1 ? field : field.subarray(0, end));
}

/** @param {Uint8Array} bytes @returns {Cav} */
export function parseCav(bytes) {
  if (bytes.length < FIXED_HDR || new TextDecoder().decode(bytes.subarray(0, 6)) !== MAGIC) {
    throw new Error('not a Caiven cart');
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const version = view.getUint16(6, true);
  const count = view.getUint16(8, true);
  if (FIXED_HDR + count * ENTRY_LEN > bytes.length) throw new Error('cart is truncated');
  /** @type {Section[]} */
  const sections = [];
  for (let i = 0; i < count; i++) {
    const e = FIXED_HDR + i * ENTRY_LEN;
    const kind = view.getUint16(e, true);
    const offset = view.getUint32(e + 2, true);
    const len = view.getUint32(e + 6, true);
    if (offset + len > bytes.length) throw new Error('cart is truncated');
    const data = bytes.slice(offset, offset + len);
    if (crc32(data) !== view.getUint32(e + 10, true)) throw new Error('cart checksum mismatch');
    sections.push({ kind, data });
  }
  return {
    version,
    title: readField(bytes, 10),
    author: readField(bytes, 10 + FIELD_LEN),
    entry: view.getUint32(10 + 2 * FIELD_LEN, true),
    flags: view.getUint32(14 + 2 * FIELD_LEN, true),
    sections,
  };
}

/** Truncates to 32 bytes without splitting a UTF-8 sequence. @param {string} text */
function encodeField(text) {
  const out = new Uint8Array(FIELD_LEN);
  let used = 0;
  for (const ch of text) {
    const enc = new TextEncoder().encode(ch);
    if (used + enc.length > FIELD_LEN) break;
    out.set(enc, used);
    used += enc.length;
  }
  return out;
}

/** @param {Cav} cav @returns {Uint8Array} */
export function writeCav(cav) {
  const dataStart = FIXED_HDR + cav.sections.length * ENTRY_LEN;
  const total = dataStart + cav.sections.reduce((n, s) => n + s.data.length, 0);
  const out = new Uint8Array(total);
  const view = new DataView(out.buffer);
  out.set(new TextEncoder().encode(MAGIC), 0);
  view.setUint16(6, FORMAT_VERSION, true);
  view.setUint16(8, cav.sections.length, true);
  out.set(encodeField(cav.title), 10);
  out.set(encodeField(cav.author), 10 + FIELD_LEN);
  view.setUint32(10 + 2 * FIELD_LEN, cav.entry, true);
  view.setUint32(14 + 2 * FIELD_LEN, cav.flags, true);
  let offset = dataStart;
  cav.sections.forEach((s, i) => {
    const e = FIXED_HDR + i * ENTRY_LEN;
    view.setUint16(e, s.kind, true);
    view.setUint32(e + 2, offset, true);
    view.setUint32(e + 6, s.data.length, true);
    view.setUint32(e + 10, crc32(s.data), true);
    out.set(s.data, offset);
    offset += s.data.length;
  });
  return out;
}

/** @param {Cav} cav @returns {string | null} */
export function luaSource(cav) {
  const section = cav.sections.find((s) => s.kind === LUA_SOURCE);
  return section ? new TextDecoder().decode(section.data) : null;
}

/**
 * Same cart with its Lua replaced (and optionally retitled). Writes the
 * current format version, so it only accepts carts this runtime can load.
 * @param {Cav} cav @param {string} source @param {{ title?: string, author?: string }} [header]
 * @returns {Uint8Array}
 */
export function withLuaSource(cav, source, header = {}) {
  if (cav.version !== FORMAT_VERSION) throw new Error(`cart format v${cav.version} is not supported here`);
  const data = new TextEncoder().encode(source);
  let replaced = false;
  const sections = cav.sections.map((s) => {
    if (s.kind !== LUA_SOURCE) return s;
    replaced = true;
    return { kind: LUA_SOURCE, data };
  });
  if (!replaced) sections.push({ kind: LUA_SOURCE, data });
  const bytes = writeCav({ ...cav, title: header.title ?? cav.title, author: header.author ?? cav.author, sections });
  if (bytes.length > MAX_CART_BYTES) throw new Error(`cart is over the ${MAX_CART_BYTES / 1024} KiB limit`);
  return bytes;
}
