export const MEMORY_COLUMNS = 16;
export const MEMORY_ROWS = 6;
export const MEMORY_PAGE_SIZE = MEMORY_COLUMNS * MEMORY_ROWS;

export const MEMORY_REGIONS = [
  { label: 'WORK', address: 0x0000 },
  { label: 'SPRITES', address: 0x4000 },
  { label: 'MAP', address: 0x8000 },
  { label: 'PALETTE', address: 0x9000 },
  { label: 'SFX', address: 0x9100 },
  { label: 'MUSIC', address: 0x9500 },
] as const;

export interface MemoryRow {
  address: string;
  hex: string;
  ascii: string;
}

export function formatMemoryAddress(address: number): string {
  return `0x${address.toString(16).toUpperCase().padStart(4, '0')}`;
}

export function clampMemoryBase(base: number, memoryLength: number): number {
  if (memoryLength <= 0) return 0;
  const maxBase = Math.max(0, memoryLength - MEMORY_PAGE_SIZE);
  const finiteBase = Number.isFinite(base) ? base : 0;
  const rowAligned = Math.floor(finiteBase / MEMORY_COLUMNS) * MEMORY_COLUMNS;
  return Math.min(maxBase, Math.max(0, rowAligned));
}

export function memoryBaseForAddress(address: number, memoryLength: number): number {
  return clampMemoryBase(address, memoryLength);
}

export function parseMemoryAddress(value: string, memoryLength: number): number | null {
  const trimmed = value.trim();
  if (!/^(?:0x)?[0-9a-f]+$/i.test(trimmed)) return null;
  const address = Number.parseInt(trimmed.replace(/^0x/i, ''), 16);
  if (!Number.isFinite(address)) return null;
  return memoryBaseForAddress(address, memoryLength);
}

export function formatMemoryRows(ram: number[], base: number): MemoryRow[] {
  const pageBase = clampMemoryBase(base, ram.length);
  const rows: MemoryRow[] = [];
  for (let row = 0; row < MEMORY_ROWS; row += 1) {
    const address = pageBase + row * MEMORY_COLUMNS;
    if (address >= ram.length) break;
    const bytes = ram.slice(address, Math.min(address + MEMORY_COLUMNS, ram.length));
    rows.push({
      address: formatMemoryAddress(address),
      hex: bytes.map((byte) => byte.toString(16).toUpperCase().padStart(2, '0')).join(' '),
      ascii: bytes.map((byte) => byte >= 32 && byte <= 126 ? String.fromCharCode(byte) : '.').join(''),
    });
  }
  return rows;
}
