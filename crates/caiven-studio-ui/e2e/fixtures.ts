import { expect, test as base, type ConsoleMessage } from '@playwright/test';

export type BankKind = 'sprites' | 'map' | 'palette' | 'sfx' | 'music';

export interface E2EControl {
  calls(): Promise<{ command: string; args: Record<string, unknown> }[]>;
  failNext(key: string, message: string): Promise<void>;
  delayNext(key: string, milliseconds: number): Promise<void>;
  queueDialog(kind: 'open' | 'save', value: string | null): Promise<void>;
  emit(event: string, payload: unknown): Promise<void>;
  setTickBanks(active: Partial<Record<BankKind, number>>): Promise<void>;
  setBankData(kind: BankKind, id: number, data: number[]): Promise<void>;
  snapshot(): Promise<Record<string, unknown>>;
}

declare global {
  interface Window {
    __CAIVEN_E2E__: E2EControl;
  }
}

function installBridge() {
  const sessionKey = '__caiven_e2e_session';
  const sourcesKey = '__caiven_e2e_sources';
  if (localStorage.getItem(sessionKey) !== 'active') {
    localStorage.clear();
    localStorage.setItem(sessionKey, 'active');
    localStorage.setItem('caiven-studio-tour-complete', '1');
  }

  type Kind = 'sprites' | 'map' | 'palette' | 'sfx' | 'music';
  const lengths: Record<Kind, number> = { sprites: 16384, map: 4096, palette: 48, sfx: 1024, music: 256 };
  const offsets: Record<Kind, number> = { sprites: 0x4000, map: 0x8000, palette: 0x9100, sfx: 0x9200, music: 0x9600 };
  const calls: { command: string; args: Record<string, unknown> }[] = [];
  const faults = new Map<string, string[]>();
  const delays = new Map<string, number[]>();
  const dialogs = { open: [] as (string | null)[], save: [] as (string | null)[] };
  const callbacks = new Map<number, (event: unknown) => void>();
  const listeners = new Map<string, Set<number>>();
  let callbackId = 0;
  let listenerId = 0;
  const listenerEvents = new Map<number, { event: string; callback: number }>();

  const make = (length: number, seed = 0) => Array.from({ length }, (_, i) => i === 0 ? seed : 0);
  const banks: Record<Kind, Map<number, number[]>> = {
    sprites: new Map([[0, make(lengths.sprites, 7)], [1, make(lengths.sprites, 3)]]),
    map: new Map([[0, make(lengths.map, 0)], [1, make(lengths.map, 9)]]),
    palette: new Map([
      [0, [0, 0, 0, 29, 43, 83, 126, 37, 83, 0, 135, 81, 171, 82, 54, 95, 87, 79, 194, 195, 199, 255, 241, 232, 255, 0, 77, 255, 163, 0, 255, 236, 39, 0, 228, 54, 41, 173, 255, 131, 118, 156, 255, 119, 168, 255, 204, 170]],
      [1, Array.from({ length: 16 }, (_, i) => [i, 255 - i, i * 8]).flat()],
    ]),
    sfx: new Map([[0, make(lengths.sfx, 12)], [1, make(lengths.sfx, 55)]]),
    music: new Map([[0, make(lengths.music, 1)], [1, make(lengths.music, 8)]]),
  };
  const flags = new Map([[0, make(256, 1)], [1, make(256, 2)]]);
  const COLLISION_OFFSET = 0x9703;
  const COLLISION_LEN = 4096;
  const collision = new Map([[0, make(COLLISION_LEN, 0)], [1, make(COLLISION_LEN, 0)]]);
  let collisionTypes: { id: number; name: string; color: [number, number, number]; solid: boolean }[] = [
    { id: 0, name: 'walkable', color: [0, 0, 0], solid: false },
    { id: 1, name: 'solid', color: [255, 176, 0], solid: true },
    { id: 2, name: 'hazard', color: [224, 32, 32], solid: false },
  ];
  const active: Record<Kind, number> = { sprites: 0, map: 0, palette: 0, sfx: 0, music: 0 };
  const tickActive: Record<Kind, number> = { ...active };
  const ram = make(65536);
  let runState: 'running' | 'paused' | 'stopped' = 'paused';
  let frame = 42;
  const initialSources = [
    { path: '/carts/test/main.lua', name: 'main.lua', text: 'function _update()\n  sprite(0, 1, 2)\nend', dirty: false },
    { path: '/carts/test/enemy.lua', name: 'enemy.lua', text: 'return {}', dirty: false },
  ];
  let sources = initialSources;
  try {
    const stored = localStorage.getItem(sourcesKey);
    if (stored) sources = JSON.parse(stored) as typeof initialSources;
  } catch {
    localStorage.removeItem(sourcesKey);
  }
  let recent = ['/carts/test', '/carts/other'];
  let breakpoints: { source: string; line: number }[] = [];
  let watches: { name: string; value: string }[] = [];
  let port = { authenticated: false, username: '', portUrl: 'http://port.test' };
  let assetIndexReads = 0;
  let cartSizeReads = 0;

  function sync(kind: Kind) {
    const data = banks[kind].get(active[kind])!;
    ram.splice(offsets[kind], data.length, ...data);
    if (kind === 'sprites') ram.splice(0x9000, 256, ...(flags.get(active.sprites) ?? make(256)));
    if (kind === 'map') ram.splice(COLLISION_OFFSET, COLLISION_LEN, ...(collision.get(active.map) ?? make(COLLISION_LEN)));
  }
  (Object.keys(active) as Kind[]).forEach(sync);

  const paletteHex = () => {
    const bytes = banks.palette.get(active.palette)!;
    return Array.from({ length: 16 }, (_, slot) => `#${bytes.slice(slot * 3, slot * 3 + 3).map((n) => n.toString(16).padStart(2, '0')).join('')}`.toUpperCase());
  };
  const audio = () => ({ sfxActive: false, sfxId: 0, sfxStep: 0, musicActive: false, musicPattern: 0, musicRow: 0, musicLoop: true });
  const index = () => ({
    entries: [
      { kind: 'sprite', id: 0, used: true, nonzero: true, bytes: 64, refs: [{ path: 'main.lua', line: 2, col: 3, label: 'main.lua:2' }] },
      { kind: 'sfx', id: 0, used: false, nonzero: true, bytes: 64, refs: [] },
    ],
    computedRefs: 0,
  });
  const bootstrap = () => ({
    connected: true, title: 'test-cart', path: '/carts/test', author: 'tester', runState, frame, fps: 60,
    cartSize: { packedBytes: 8192 + cartSizeReads, maxBytes: 131072 }, sources: structuredClone(sources), palette: paletteHex(),
    spriteSheet: [...banks.sprites.get(active.sprites)!], map: [...banks.map.get(active.map)!],
    spriteBanks: [...banks.sprites.keys()], mapBanks: [...banks.map.keys()], activeSpriteBank: active.sprites, activeMapBank: active.map,
    spriteFlags: [...flags.get(active.sprites)!], collision: [...(collision.get(active.map) ?? make(COLLISION_LEN))],
    collisionTypes: structuredClone(collisionTypes),
    sfx: [...banks.sfx.get(active.sfx)!], music: [...banks.music.get(active.music)!],
    paletteBanks: [...banks.palette.keys()], activePaletteBank: active.palette,
    sfxBanks: [...banks.sfx.keys()], activeSfxBank: active.sfx, musicBanks: [...banks.music.keys()], activeMusicBank: active.music,
    ram: [...ram], globals: [{ name: 'score', value: '7' }], watches: structuredClone(watches), callStack: [], breakpoints: structuredClone(breakpoints),
    pauseReason: null, diagnostics: [], output: ['mock runtime ready'], meta: { description: 'Test cartridge', tags: ['e2e'] },
    assetIndex: index(), audio: audio(), recent: [...recent],
    api: [{ name: 'sprite', params: [{ name: 'id', ty: 'int' }], returns: 'nil', doc: 'Draw sprite.', category: 'Graphics' }],
  });

  function requestKeys(command: string, args: Record<string, unknown>) {
    return [
      `${command}:${String(args.kind ?? '')}:${String(args.action ?? '')}`,
      `${command}:${String(args.kind ?? '')}`,
      command,
    ];
  }
  function maybeFail(command: string, args: Record<string, unknown>) {
    for (const key of requestKeys(command, args)) {
      const queue = faults.get(key);
      if (queue?.length) throw new Error(queue.shift()!);
    }
  }
  function takeDelay(command: string, args: Record<string, unknown>) {
    for (const key of requestKeys(command, args)) {
      const queue = delays.get(key);
      if (queue?.length) return queue.shift()!;
    }
    return 0;
  }
  function persistSources() {
    localStorage.setItem(sourcesKey, JSON.stringify(sources));
  }
  function emit(event: string, payload: unknown) {
    for (const id of listeners.get(event) ?? []) callbacks.get(id)?.({ event, id: 0, payload });
  }

  async function invoke(command: string, rawArgs?: Record<string, unknown>) {
    const args = rawArgs ?? {};
    calls.push({ command, args: JSON.parse(JSON.stringify(args)) as Record<string, unknown> });
    maybeFail(command, args);
    const delay = takeDelay(command, args);
    // studio_asset_bank mutates state synchronously, then delays only the reply (below) —
    // preserves "which mutation wins" ordering for out-of-order-response tests.
    if (delay > 0 && command !== 'studio_asset_bank') await new Promise((resolve) => setTimeout(resolve, delay));
    if (command === 'plugin:event|listen') {
      const id = ++listenerId;
      const event = String(args.event);
      const callback = Number(args.handler);
      listenerEvents.set(id, { event, callback });
      if (!listeners.has(event)) listeners.set(event, new Set());
      listeners.get(event)!.add(callback);
      return id;
    }
    if (command === 'plugin:event|unlisten') {
      const info = listenerEvents.get(Number(args.eventId));
      if (info) listeners.get(info.event)?.delete(info.callback);
      return null;
    }
    if (command === 'plugin:dialog|open') return dialogs.open.shift() ?? null;
    if (command === 'plugin:dialog|save') return dialogs.save.shift() ?? null;
    if (command === 'studio_bootstrap') return bootstrap();
    if (command === 'studio_list_templates') return [
      { id: 'top-down-mover', name: 'Top-down mover', description: 'Move a sprite' },
      { id: 'blank', name: 'Blank', description: 'Empty cart' },
    ];
    if (command === 'studio_cart_size') return { packedBytes: 8192 + ++cartSizeReads, maxBytes: 131072 };
    if (command === 'studio_asset_index') { assetIndexReads += 1; return index(); }
    if (command === 'studio_tick') return { runState, frame: frame++, fps: 60, frameTimeMs: 4.2, globals: [{ name: 'score', value: '7' }], watches, callStack: [], pauseReason: null, audio: audio(), diagnostics: [], output: ['mock runtime ready'], activeSpriteBank: tickActive.sprites, activeMapBank: tickActive.map, activePaletteBank: tickActive.palette, activeSfxBank: tickActive.sfx, activeMusicBank: tickActive.music };
    if (command === 'studio_frame') return Array(128 * 128).fill(0);
    if (command === 'studio_read_memory') return ram.slice(Number(args.address), Number(args.address) + Number(args.len));
    if (command === 'studio_asset_bank') {
      const kind = args.kind as Kind;
      const action = String(args.action);
      if (!(kind in banks)) throw new Error(`Unknown bank kind: ${String(kind)}`);
      if (action === 'create') {
        let id = 1; while (banks[kind].has(id)) id += 1;
        banks[kind].set(id, make(lengths[kind]));
        if (kind === 'sprites') flags.set(id, make(256));
        if (kind === 'map') collision.set(id, make(COLLISION_LEN));
        active[kind] = id; tickActive[kind] = id; sync(kind);
      } else if (action === 'select') {
        const id = Number(args.id);
        if (!banks[kind].has(id)) throw new Error(`Missing ${kind} bank ${id}`);
        active[kind] = id; tickActive[kind] = id; sync(kind);
      } else if (action === 'delete') {
        const id = Number(args.id);
        if (id === 0) throw new Error('Bank 0 cannot be deleted');
        banks[kind].delete(id); if (kind === 'sprites') flags.delete(id);
        if (kind === 'map') collision.delete(id);
        active[kind] = 0; tickActive[kind] = 0; sync(kind);
      } else if (action !== 'read') throw new Error(`Unknown bank action: ${action}`);
      const result = { kind, ids: [...banks[kind].keys()], active: active[kind], data: [...banks[kind].get(active[kind])!] };
      if (delay > 0) await new Promise((resolve) => setTimeout(resolve, delay));
      return result;
    }
    if (command === 'studio_write_buffer') { const source = sources.find((item) => item.path === args.path); if (source) { source.text = String(args.text); source.dirty = true; persistSources(); } return null; }
    if (command === 'studio_save') { for (const source of sources) source.dirty = false; persistSources(); return sources.map((source) => source.name); }
    if (command === 'studio_transport') { const action = String(args.action); runState = action === 'pause' || action === 'step' ? 'paused' : 'running'; if (action === 'step') frame += 1; return { ...(await invoke('studio_tick')), runState }; }
    if (command === 'studio_set_input') return null;
    if (command === 'studio_write_sprite') { const at = Number(args.sprite) * 64; banks.sprites.get(active.sprites)!.splice(at, 64, ...args.pixels as number[]); sync('sprites'); return null; }
    if (command === 'studio_write_palette') { const slot = Number(args.slot); const hex = String(args.hex); const bytes = [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16)); banks.palette.get(active.palette)!.splice(slot * 3, 3, ...bytes); sync('palette'); return null; }
    if (command === 'studio_write_map_cells') { for (const cell of args.cells as { offset: number; tile: number }[]) banks.map.get(active.map)![cell.offset] = cell.tile; sync('map'); return null; }
    if (command === 'studio_write_collision_cells') { for (const cell of args.cells as { offset: number; value: number }[]) collision.get(active.map)![cell.offset] = cell.value; sync('map'); return null; }
    if (command === 'studio_read_collision_types') return structuredClone(collisionTypes);
    if (command === 'studio_write_collision_types') { collisionTypes = args.types as typeof collisionTypes; return null; }
    if (command === 'studio_write_memory') {
      const address = Number(args.address); const bytes = args.bytes as number[];
      ram.splice(address, bytes.length, ...bytes);
      if (address >= 0x9000 && address < 0x9100) flags.get(active.sprites)!.splice(address - 0x9000, bytes.length, ...bytes);
      if (address >= COLLISION_OFFSET && address < COLLISION_OFFSET + COLLISION_LEN) collision.get(active.map)!.splice(address - COLLISION_OFFSET, bytes.length, ...bytes);
      for (const kind of ['sfx', 'music'] as Kind[]) if (address >= offsets[kind] && address < offsets[kind] + lengths[kind]) banks[kind].get(active[kind])!.splice(address - offsets[kind], bytes.length, ...bytes);
      return null;
    }
    if (command === 'studio_toggle_breakpoint') { const row = { source: String(args.source), line: Number(args.line) }; const found = breakpoints.findIndex((item) => item.source === row.source && item.line === row.line); found >= 0 ? breakpoints.splice(found, 1) : breakpoints.push(row); return structuredClone(breakpoints); }
    if (command === 'studio_add_watch') { const name = String(args.expression); if (!watches.some((item) => item.name === name)) watches.push({ name, value: '7' }); return structuredClone(watches); }
    if (command === 'studio_remove_watch') { watches = watches.filter((item) => item.name !== args.expression); return structuredClone(watches); }
    if (command === 'studio_clear_output') return null;
    if (command === 'studio_remove_recent') { recent = recent.filter((path) => path !== args.path); return [...recent]; }
    if (command === 'studio_write_meta') return null;
    if (command === 'studio_create_module') { const name = String(args.name); if (!/^[\w/-]+\.lua$/.test(name)) throw new Error('Module name must end in .lua'); const source = { path: `/carts/test/${name}`, name, text: '', dirty: true }; sources.push(source); persistSources(); return source; }
    if (command === 'studio_open_project' || command === 'studio_new_project') { if (command === 'studio_new_project') { sources = [{ path: `${args.path}/main.lua`, name: 'main.lua', text: '', dirty: false }]; persistSources(); } return { ...bootstrap(), path: String(args.path), title: command === 'studio_new_project' ? 'new-cart' : 'test-cart' }; }
    if (command === 'studio_close_project') return { ...bootstrap(), connected: false, title: '', path: '', sources: [] };
    if (command === 'studio_export') return null;
    if (command === 'studio_export_web') return null;
    if (command === 'studio_audio_transport') return { ...audio(), [`${String(args.kind)}Active`]: args.action === 'play', [`${String(args.kind)}${args.kind === 'sfx' ? 'Id' : 'Pattern'}`]: Number(args.id) };
    if (command === 'studio_scan_library') return [{ path: '/library/moon', name: 'moon', title: 'Moon', author: 'tester', modified: 1, project: true }];
    if (command === 'port_session') return port;
    if (command === 'port_link_start') return { requestId: 'request-1', pollSecret: 'secret', expiresAt: '2099-01-01T00:00:00Z' };
    if (command === 'port_link_poll') { port = { authenticated: true, username: 'tester', portUrl: 'http://port.test' }; return port; }
    if (command === 'port_link_cancel') return null;
    if (command === 'port_logout') { port = { authenticated: false, username: '', portUrl: port.portUrl }; return port; }
    if (command === 'port_list_carts') return { carts: [{ id: 'cart-1', title: 'Moon', author: 'maker', description: 'Demo', tags: ['arcade'], downloads: 4, owner: null, ratingAvg: 5, ratingCount: 1, latestVersion: 2, cartSize: 2048, hasScreenshot: false, screenshotUrl: '' }], total: 1, page: 0, perPage: 20, portUrl: port.portUrl };
    if (command === 'port_download') return '/downloads/moon';
    if (command === 'studio_port_publish') { emit('publish:progress', { step: 'upload', pct: 75, note: 'Uploading' }); return { cartId: 'cart-1', version: 3 }; }
    throw new Error(`Unexpected IPC command: ${command}`);
  }

  Object.defineProperty(window, '__TAURI_INTERNALS__', { value: {
    invoke,
    transformCallback(callback: (event: unknown) => void) { const id = ++callbackId; callbacks.set(id, callback); return id; },
    unregisterCallback(id: number) { callbacks.delete(id); },
    metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
  } });
  window.__CAIVEN_E2E__ = {
    async calls() { return structuredClone(calls); },
    async failNext(key, message) { const queue = faults.get(key) ?? []; queue.push(message); faults.set(key, queue); },
    async delayNext(key, milliseconds) { const queue = delays.get(key) ?? []; queue.push(milliseconds); delays.set(key, queue); },
    async queueDialog(kind, value) { dialogs[kind].push(value); },
    async emit(event, payload) { emit(event, payload); },
    async setTickBanks(next) {
      for (const [kind, id] of Object.entries(next) as [Kind, number][]) {
        if (!banks[kind].has(id)) throw new Error(`Missing ${kind} bank ${id}`);
        tickActive[kind] = id;
        active[kind] = id;
        sync(kind);
      }
    },
    async setBankData(kind, id, data) { banks[kind].set(id, [...data]); if (active[kind] === id) sync(kind); },
    async snapshot() { return { active: { ...active }, tickActive: { ...tickActive }, banks: Object.fromEntries((Object.keys(banks) as Kind[]).map((kind) => [kind, Object.fromEntries(banks[kind])])), flags: Object.fromEntries(flags), collision: Object.fromEntries(collision), ram: [...ram], sources: structuredClone(sources), recent: [...recent], breakpoints: structuredClone(breakpoints), watches: structuredClone(watches), port: { ...port }, assetIndexReads, cartSizeReads }; },
  };
}

type ErrorGuard = { allow(pattern: RegExp): void };

export const test = base.extend<{ e2e: E2EControl; errorGuard: ErrorGuard }>({
  e2e: async ({ page }, use) => {
    await page.addInitScript(installBridge);
    await page.goto('/');
    await expect(page.getByText('test-cart', { exact: true }).first()).toBeVisible();
    await use({
      calls: () => page.evaluate(() => window.__CAIVEN_E2E__.calls()),
      failNext: (key, message) => page.evaluate(([nextKey, nextMessage]) => window.__CAIVEN_E2E__.failNext(nextKey, nextMessage), [key, message] as const),
      delayNext: (key, milliseconds) => page.evaluate(([nextKey, nextMilliseconds]) => window.__CAIVEN_E2E__.delayNext(nextKey, nextMilliseconds), [key, milliseconds] as const),
      queueDialog: (kind, value) => page.evaluate(([nextKind, nextValue]) => window.__CAIVEN_E2E__.queueDialog(nextKind, nextValue), [kind, value] as const),
      emit: (event, payload) => page.evaluate(([nextEvent, nextPayload]) => window.__CAIVEN_E2E__.emit(nextEvent, nextPayload), [event, payload] as const),
      setTickBanks: (active) => page.evaluate((next) => window.__CAIVEN_E2E__.setTickBanks(next), active),
      setBankData: (kind, id, data) => page.evaluate(([nextKind, nextId, nextData]) => window.__CAIVEN_E2E__.setBankData(nextKind, nextId, nextData), [kind, id, data] as const),
      snapshot: () => page.evaluate(() => window.__CAIVEN_E2E__.snapshot()),
    });
  },
  errorGuard: [async ({ page }, use) => {
    const errors: string[] = [];
    const allowed: RegExp[] = [];
    const recordConsole = (message: ConsoleMessage) => {
      if (message.type() === 'error' || message.type() === 'warning') {
        const location = message.location();
        const source = location.url ? ` (${location.url}:${location.lineNumber}:${location.columnNumber})` : '';
        errors.push(`[${message.type()}] ${message.text()}${source}`);
      }
    };
    page.on('console', recordConsole);
    page.on('pageerror', (error) => errors.push(error.message));
    await use({ allow: (pattern) => allowed.push(pattern) });
    expect(errors.filter((message) => !allowed.some((pattern) => pattern.test(message))), 'unexpected console warnings/page errors').toEqual([]);
  }, { auto: true }],
});

export { expect };
